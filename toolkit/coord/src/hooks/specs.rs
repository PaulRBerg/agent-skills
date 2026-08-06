//! Canonical host-hook definitions shared by installers and runtime checks.

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum Client {
    Codex,
    Claude,
}

impl Client {
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct HookSpec {
    pub(crate) event: &'static str,
    pub(crate) command: &'static str,
    pub(crate) matcher: Option<&'static str>,
    pub(crate) timeout: Option<u64>,
    pub(crate) additional_context_limit: Option<u64>,
    pub(crate) if_filter: Option<&'static str>,
    pub(crate) async_: Option<bool>,
    pub(crate) async_rewake: Option<bool>,
}

const CODEX_HOOK_SPECS: &[HookSpec] = &[
    HookSpec {
        event: "SessionStart",
        command: "ai-coord hook codex",
        matcher: Some("startup|resume|clear"),
        timeout: Some(5),
        additional_context_limit: None,
        if_filter: None,
        async_: None,
        async_rewake: None,
    },
    HookSpec {
        event: "UserPromptSubmit",
        command: "ai-coord hook codex",
        matcher: None,
        timeout: Some(5),
        additional_context_limit: Some(200),
        if_filter: None,
        async_: None,
        async_rewake: None,
    },
    HookSpec {
        event: "Stop",
        command: "ai-coord hook codex",
        matcher: None,
        timeout: Some(5),
        additional_context_limit: None,
        if_filter: None,
        async_: None,
        async_rewake: None,
    },
    HookSpec {
        event: "SessionEnd",
        command: "ai-coord hook codex",
        matcher: None,
        timeout: Some(3),
        additional_context_limit: None,
        if_filter: None,
        async_: None,
        async_rewake: None,
    },
    HookSpec {
        event: "SubagentStart",
        command: "ai-coord hook codex",
        matcher: None,
        timeout: Some(5),
        additional_context_limit: None,
        if_filter: None,
        async_: None,
        async_rewake: None,
    },
    HookSpec {
        event: "SubagentStop",
        command: "ai-coord hook codex",
        matcher: None,
        timeout: Some(5),
        additional_context_limit: None,
        if_filter: None,
        async_: None,
        async_rewake: None,
    },
    HookSpec {
        event: "PostToolUse",
        command: "ai-coord hook codex",
        matcher: None,
        timeout: Some(5),
        additional_context_limit: None,
        if_filter: None,
        async_: None,
        async_rewake: None,
    },
];

const CLAUDE_HOOK_SPECS: &[HookSpec] = &[
    HookSpec {
        event: "SessionStart",
        command: "ai-coord hook claude",
        matcher: None,
        timeout: Some(5),
        additional_context_limit: None,
        if_filter: None,
        async_: None,
        async_rewake: None,
    },
    HookSpec {
        event: "UserPromptSubmit",
        command: "ai-coord hook claude",
        matcher: None,
        timeout: Some(5),
        additional_context_limit: None,
        if_filter: None,
        async_: None,
        async_rewake: None,
    },
    HookSpec {
        event: "Stop",
        command: "ai-coord hook claude",
        matcher: None,
        timeout: Some(5),
        additional_context_limit: None,
        if_filter: None,
        async_: None,
        async_rewake: None,
    },
    HookSpec {
        event: "SessionEnd",
        command: "ai-coord hook claude",
        matcher: None,
        timeout: Some(3),
        additional_context_limit: None,
        if_filter: None,
        async_: None,
        async_rewake: None,
    },
    HookSpec {
        event: "SubagentStart",
        command: "ai-coord hook claude",
        matcher: None,
        timeout: Some(5),
        additional_context_limit: None,
        if_filter: None,
        async_: None,
        async_rewake: None,
    },
    HookSpec {
        event: "SubagentStop",
        command: "ai-coord hook claude",
        matcher: None,
        timeout: Some(5),
        additional_context_limit: None,
        if_filter: None,
        async_: None,
        async_rewake: None,
    },
    HookSpec {
        event: "PostToolUse",
        command: "ai-coord hook claude",
        matcher: Some("ExitPlanMode"),
        timeout: Some(5),
        additional_context_limit: None,
        if_filter: None,
        async_: None,
        async_rewake: None,
    },
    HookSpec {
        event: "PostToolBatch",
        command: "ai-coord hook claude",
        matcher: None,
        timeout: Some(5),
        additional_context_limit: None,
        if_filter: None,
        async_: None,
        async_rewake: None,
    },
    HookSpec {
        event: "PostToolUseFailure",
        command: "ai-coord waker claude",
        matcher: Some("Bash"),
        timeout: Some(3600),
        additional_context_limit: None,
        if_filter: Some("Bash(ai-coord start *)"),
        async_: Some(true),
        async_rewake: Some(true),
    },
];

pub(crate) const fn hook_specs(client: Client) -> &'static [HookSpec] {
    match client {
        Client::Codex => CODEX_HOOK_SPECS,
        Client::Claude => CLAUDE_HOOK_SPECS,
    }
}

#[cfg(test)]
mod tests {
    use super::{Client, hook_specs};

    #[test]
    fn canonical_specs_have_the_expected_host_contract() {
        let codex = hook_specs(Client::Codex);
        let claude = hook_specs(Client::Claude);

        assert_eq!(codex.len(), 7);
        assert_eq!(claude.len(), 9);
        assert_eq!(codex[0].matcher, Some("startup|resume|clear"));
        assert_eq!(codex[1].additional_context_limit, Some(200));
        assert_eq!(claude[6].event, "PostToolUse");
        assert_eq!(claude[6].matcher, Some("ExitPlanMode"));
        assert_eq!(claude[8].if_filter, Some("Bash(ai-coord start *)"));
        assert_eq!(claude[8].async_, Some(true));
        assert_eq!(claude[8].async_rewake, Some(true));
    }
}
