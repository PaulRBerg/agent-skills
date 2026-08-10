use crate::{
    domain::{Client, InventoryResult, ProcessProbe},
    error::Result,
    hooks::{config::inspect_hooks, specs::Client as HookClient, trust::inspect_codex_hook_trust},
    host::{
        ClaudeSessionObservation, CodexHookLedgerEvidence, ProviderContext, codex_provider_report,
        collect_claude_inventory, inventory_result,
    },
    state::Store,
};

pub(crate) struct InventoryObservation {
    pub(crate) result: InventoryResult,
    pub(crate) claude_sessions: Vec<ClaudeSessionObservation>,
    pub(crate) claude_authoritative: bool,
}

pub(crate) trait ProviderInventory: Send {
    fn cache_key(&self) -> &str;
    fn refresh(&mut self, store: &Store, probe: &dyn ProcessProbe) -> Result<InventoryObservation>;
}

pub(crate) struct HostInventory {
    context: ProviderContext,
}

impl HostInventory {
    pub(crate) fn discover() -> Self {
        Self { context: ProviderContext::discover() }
    }
}

impl ProviderInventory for HostInventory {
    fn cache_key(&self) -> &str {
        &self.context.cache_key
    }

    fn refresh(&mut self, store: &Store, probe: &dyn ProcessProbe) -> Result<InventoryObservation> {
        let hooks = inspect_hooks(HookClient::Codex, &self.context.codex_hooks_path());
        let last_hook_error_code = store
            .hook_health()?
            .into_iter()
            .rev()
            .find(|row| row.client == Client::Codex && row.last_error_code.is_some())
            .and_then(|row| row.last_error_code);
        let trust = if hooks.ok && last_hook_error_code.is_none() {
            Some(inspect_codex_hook_trust(Some(&hooks.path)))
        } else {
            None
        };
        let codex = codex_provider_report(
            self.context.codex_executable.as_deref(),
            &CodexHookLedgerEvidence {
                hooks_ok: hooks.ok,
                hooks_error: hooks.error,
                missing_hooks: hooks.missing,
                last_hook_error_code,
                trust_ok: trust.as_ref().is_some_and(|check| check.ok),
                trust_error: trust.and_then(|check| check.error),
            },
        );
        let claude = collect_claude_inventory(self.context.claude_executable.as_deref(), probe);
        Ok(InventoryObservation {
            result: inventory_result(vec![codex, claude.report.clone()]),
            claude_sessions: claude.sessions,
            claude_authoritative: claude.authoritative,
        })
    }
}

#[cfg(test)]
pub(crate) struct StaticInventory {
    pub(crate) complete: bool,
    pub(crate) refreshes: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

#[cfg(test)]
impl ProviderInventory for StaticInventory {
    fn cache_key(&self) -> &str {
        "static"
    }

    fn refresh(&mut self, _store: &Store, _probe: &dyn ProcessProbe) -> Result<InventoryObservation> {
        self.refreshes.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let reports = [Client::Codex, Client::Claude]
            .into_iter()
            .map(|client| crate::domain::ProviderReport {
                client,
                ok: self.complete,
                source: "static".to_owned(),
                enabled: true,
                dropped: 0,
                error: (!self.complete).then(|| "incomplete".to_owned()),
            })
            .collect();
        Ok(InventoryObservation {
            result: InventoryResult { complete: self.complete, providers: reports },
            claude_sessions: Vec::new(),
            claude_authoritative: false,
        })
    }
}
