//! Actionable hints appended to clap usage errors, so a guessed flag gets its valid spelling.

use std::ffi::OsString;

use clap::{
    Arg, CommandFactory,
    error::{ContextKind, ContextValue, ErrorKind},
};

use crate::cli::Cli;

/// Returns the hint for a usage error, or `None` for help, version, or errors that already print help.
pub(crate) fn usage_hint(error: &clap::Error, arguments: &[OsString]) -> Option<String> {
    if !error.use_stderr() || error.kind() == ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand {
        return None;
    }
    let mut root = Cli::command();
    root.build();
    let path = subcommand_path(&root, arguments);
    let mut command = &mut root;
    for name in &path {
        command = command.find_subcommand_mut(name)?;
    }
    let mut lines = Vec::new();
    if let Some(ContextValue::String(invalid)) = error.get(ContextKind::InvalidArg) {
        let name = invalid.trim_start_matches('-').split('=').next().unwrap_or_default();
        if let Some(arg) = command.get_arguments().find(|arg| arg.get_long() == Some(name)) {
            lines.push(format!("hint: write `{}`", flag(arg)));
        } else {
            let owners = owners_of(&Cli::command(), name, "ai-coord");
            if !owners.is_empty() {
                let owners = owners.iter().map(|owner| format!("`{owner}`")).collect::<Vec<_>>().join(", ");
                lines.push(format!("hint: `--{name}` is an option of {owners}; place it after that subcommand"));
            }
        }
    }
    if !path.is_empty() {
        lines.push(format!("\n{}", command.render_help()));
    }
    (!lines.is_empty()).then(|| lines.join("\n"))
}

/// Follows positional arguments through the subcommand tree, stopping at the first non-subcommand.
fn subcommand_path(root: &clap::Command, arguments: &[OsString]) -> Vec<String> {
    let mut path = Vec::new();
    let mut current = root;
    for text in arguments.iter().skip(1).filter_map(|argument| argument.to_str()) {
        if text.starts_with('-') {
            continue;
        }
        let Some(next) = current.find_subcommand(text) else { break };
        path.push(next.get_name().to_owned());
        current = next;
    }
    path
}

fn owners_of(command: &clap::Command, long: &str, prefix: &str) -> Vec<String> {
    let mut owners = Vec::new();
    for sub in command.get_subcommands().filter(|sub| !sub.is_hide_set()) {
        let name = format!("{prefix} {}", sub.get_name());
        if sub.get_arguments().any(|arg| arg.get_long() == Some(long)) {
            owners.push(name.clone());
        }
        owners.extend(owners_of(sub, long, &name));
    }
    owners
}

fn flag(arg: &Arg) -> String {
    let long = arg.get_long().unwrap_or_default();
    match arg.get_value_names().and_then(<[_]>::first) {
        Some(value) if arg.get_action().takes_values() => format!("--{long} <{value}>"),
        _ => format!("--{long}"),
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    fn hint(arguments: &[&str]) -> Option<String> {
        let arguments = arguments.iter().map(OsString::from).collect::<Vec<_>>();
        let error = Cli::try_parse_from(&arguments).expect_err("arguments must be invalid");
        usage_hint(&error, &arguments)
    }

    #[test]
    fn bare_option_name_suggests_its_flag_and_subcommand_help() {
        let text = hint(&["ai-coord", "inbox", "ack", "id"]).expect("hint");
        assert!(text.contains("hint: write `--ack <ID>`"), "{text}");
        assert!(text.contains("--ack-all"), "{text}");
    }

    #[test]
    fn misplaced_flag_names_its_owner() {
        let text = hint(&["ai-coord", "--json", "inbox"]).expect("hint");
        assert!(text.contains("`ai-coord status`"), "{text}");
        assert!(text.contains("Usage: ai-coord inbox"), "the intended subcommand's help follows: {text}");
    }

    #[test]
    fn help_and_version_have_no_hint() {
        for arguments in [["ai-coord", "--help"], ["ai-coord", "--version"]] {
            assert_eq!(hint(&arguments), None);
        }
    }
}
