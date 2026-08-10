//! Claude Code and Codex configuration integration.

mod claude;
mod codex;

pub use claude::{
    ClaudeHooksReport, ClaudeHooksUpdate, HOOK_SPECS, HookSpec, ensure_claude_hooks, inspect_claude_hooks,
};
pub use codex::{
    CODEX_NOTIFY_COMMAND, CodexNotifyReport, CodexNotifyUpdate, inspect_codex_notify, resolve_codex_config_path,
    set_codex_notify, validate_codex_profile_name,
};

/// The state of one integration as observed by an inspector.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntegrationStatus {
    Missing,
    Partial,
    Ok,
    Error,
}

impl IntegrationStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Partial => "partial",
            Self::Ok => "ok",
            Self::Error => "error",
        }
    }
}

impl std::fmt::Display for IntegrationStatus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}
