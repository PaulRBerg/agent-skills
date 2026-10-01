mod types;

use crate::error::Result;

pub(crate) use types::*;

pub(crate) const fn client_name(client: Client) -> &'static str {
    match client {
        Client::Codex => "codex",
        Client::Claude => "claude",
    }
}

pub(crate) fn sanitize(text: &str, limit: usize) -> String {
    if limit == 0 {
        return String::new();
    }
    let value = text
        .chars()
        .map(|character| if character.is_control() { ' ' } else { character })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if value.chars().count() <= limit {
        value
    } else {
        let mut result = value.chars().take(limit - 1).collect::<String>();
        result.push('…');
        result
    }
}

pub(crate) fn terminal_field(value: &str) -> String {
    value.chars().map(|character| if character.is_control() { ' ' } else { character }).collect()
}

pub(crate) trait ProcessProbe: Send + Sync {
    fn fingerprint(&self, pid: u32) -> Result<ProcessFingerprint>;
    fn liveness(&self, fingerprint: &ProcessFingerprint) -> ProcessLiveness;
}

#[cfg(test)]
mod tests {
    use super::sanitize;

    #[test]
    fn sanitize_strips_controls_collapses_whitespace_and_limits_characters() {
        assert_eq!(sanitize("alpha\u{1b} beta", 80), "alpha beta");
        assert_eq!(sanitize("  alpha\t\n beta  ", 80), "alpha beta");
        assert_eq!(sanitize("alpha beta gamma", 11), "alpha beta…");
        assert_eq!(sanitize("text", 0), "");
    }
}
