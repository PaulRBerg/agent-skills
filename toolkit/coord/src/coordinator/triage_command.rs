use std::{ffi::OsString, path::Path};

pub(super) fn codex_args(repo_root: &Path, state_dir: &Path, run_dir: &Path) -> Vec<OsString> {
    [
        OsString::from("exec"),
        OsString::from("--ephemeral"),
        OsString::from("--ignore-user-config"),
        OsString::from("--model"),
        OsString::from("gpt-5.6-luna"),
        OsString::from("-c"),
        OsString::from("model_reasoning_effort=\"xhigh\""),
        OsString::from("-C"),
        repo_root.as_os_str().to_owned(),
        OsString::from("--sandbox"),
        OsString::from("workspace-write"),
        OsString::from("--add-dir"),
        state_dir.as_os_str().to_owned(),
        OsString::from("--approve-for-me"),
        OsString::from("-c"),
        OsString::from("sandbox_workspace_write.network_access=false"),
        OsString::from("-c"),
        OsString::from("web_search=\"disabled\""),
        OsString::from("-c"),
        OsString::from("agents.enabled=false"),
        OsString::from("--output-schema"),
        run_dir.join("result-schema.json").into_os_string(),
        OsString::from("--output-last-message"),
        run_dir.join("result.json").into_os_string(),
        OsString::from("-"),
    ]
    .into_iter()
    .collect()
}
