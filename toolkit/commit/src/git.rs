use std::{
    collections::HashMap,
    env,
    ffi::OsStr,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
};

use crate::error::{AppError, Result};

#[derive(Clone, Debug)]
pub struct Repository {
    pub root: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreeEntry {
    pub mode: String,
    pub kind: String,
    pub oid: String,
}

impl Repository {
    pub fn discover() -> Result<Self> {
        let cwd = env::current_dir()?;
        let root = command_text(Command::new("git").arg("-C").arg(&cwd).args(["rev-parse", "--show-toplevel"]))
            .map_err(|_| AppError::usage("current directory is not inside a Git working tree"))?;
        let root_text = root;
        let root = PathBuf::from(&root_text).canonicalize().map_err(|error| {
            AppError::operational(format!("cannot resolve repository root {}: {error}", root_text.trim()))
        })?;
        Ok(Self { root })
    }

    pub fn from_root(root: &Path) -> Result<Self> {
        let root = root.canonicalize().map_err(|error| {
            AppError::operational(format!("cannot resolve repository root {}: {error}", root.display()))
        })?;
        let top_level =
            command_text(Command::new("git").arg("-C").arg(&root).args(["rev-parse", "--show-toplevel"]))
                .map_err(|_| AppError::usage(format!("transaction repository is unavailable: {}", root.display())))?;
        let discovered_root = PathBuf::from(top_level).canonicalize()?;
        if discovered_root != root {
            return Err(AppError::usage(format!(
                "transaction path is no longer a repository root: {}",
                root.display()
            )));
        }
        Ok(Self { root })
    }

    pub fn ensure_default_index_env() -> Result<()> {
        if env::var_os("GIT_INDEX_FILE").is_some() {
            return Err(AppError::usage("GIT_INDEX_FILE is already set; refusing to inherit an alternate index"));
        }
        Ok(())
    }

    pub fn branch(&self) -> Result<String> {
        self.text(["symbolic-ref", "--quiet", "--short", "HEAD"], None)
            .map_err(|_| AppError::usage("detached HEAD is not supported"))
    }

    pub fn head(&self) -> Result<String> {
        self.text(["rev-parse", "--verify", "HEAD"], None)
            .map_err(|_| AppError::usage("repository must have an existing HEAD commit"))
    }

    pub fn ensure_idle(&self) -> Result<()> {
        let markers = ["MERGE_HEAD", "CHERRY_PICK_HEAD", "REVERT_HEAD", "REBASE_HEAD"];
        for marker in markers {
            if self.git_path(marker)?.exists() {
                return Err(AppError::usage(format!("repository operation in progress: {marker}")));
            }
        }
        for directory in ["rebase-apply", "rebase-merge", "sequencer"] {
            if self.git_path(directory)?.exists() {
                return Err(AppError::usage(format!("repository operation in progress: {directory}")));
            }
        }
        Ok(())
    }

    pub fn git_path(&self, name: &str) -> Result<PathBuf> {
        let value = self.text(["rev-parse", "--git-path", name], None)?;
        let path = PathBuf::from(value);
        Ok(if path.is_absolute() { path } else { self.root.join(path) })
    }

    pub fn text<I, S>(&self, args: I, index: Option<&Path>) -> Result<String>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = self.raw(args, index)?;
        output_text(output)
    }

    pub fn bytes<I, S>(&self, args: I, index: Option<&Path>) -> Result<Vec<u8>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = self.raw(args, index)?;
        if output.status.success() { Ok(output.stdout) } else { Err(git_error(output)) }
    }

    pub fn checked<I, S>(&self, args: I, index: Option<&Path>) -> Result<()>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = self.raw(args, index)?;
        if output.status.success() { Ok(()) } else { Err(git_error(output)) }
    }

    pub fn raw<I, S>(&self, args: I, index: Option<&Path>) -> Result<Output>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut command = self.command(index);
        command.args(args);
        command.output().map_err(|error| AppError::operational(format!("cannot execute git: {error}")))
    }

    pub fn with_input<I, S>(&self, args: I, input: &[u8], index: Option<&Path>) -> Result<Output>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut command = self.command(index);
        command.args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
        let mut child =
            command.spawn().map_err(|error| AppError::operational(format!("cannot execute git: {error}")))?;
        child.stdin.take().expect("piped stdin").write_all(input)?;
        child.wait_with_output().map_err(AppError::from)
    }

    pub fn command(&self, index: Option<&Path>) -> Command {
        let mut command = Command::new("git");
        command.arg("-C").arg(&self.root).env_remove("GIT_INDEX_FILE");
        if let Some(index) = index {
            command.env("GIT_INDEX_FILE", index);
        }
        command
    }

    pub fn tree_entry(&self, tree: &str, path: &str) -> Result<Option<TreeEntry>> {
        let pathspec = literal_pathspec(path);
        let output = self.bytes(["ls-tree", "-z", "--full-tree", tree, "--", &pathspec], None)?;
        if output.is_empty() {
            return Ok(None);
        }
        let record = output.split(|byte| *byte == 0).next().unwrap_or_default();
        let metadata = record.split(|byte| *byte == b'\t').next().unwrap_or_default();
        let metadata =
            std::str::from_utf8(metadata).map_err(|_| AppError::operational("Git returned a non-UTF-8 tree entry"))?;
        let mut fields = metadata.split_ascii_whitespace();
        let mode = fields.next().ok_or_else(|| AppError::operational("malformed Git tree entry"))?.to_owned();
        let kind = fields.next().ok_or_else(|| AppError::operational("malformed Git tree entry"))?.to_owned();
        let oid = fields.next().ok_or_else(|| AppError::operational("malformed Git tree entry"))?.to_owned();
        Ok(Some(TreeEntry { mode, kind, oid }))
    }

    pub fn tree_file_entries(&self, tree: &str, paths: &[String]) -> Result<HashMap<String, TreeEntry>> {
        let mut entries = HashMap::new();
        for paths in paths.chunks(32) {
            let mut arguments = vec![
                "ls-tree".to_owned(),
                "-r".to_owned(),
                "-z".to_owned(),
                "--full-tree".to_owned(),
                tree.to_owned(),
                "--".to_owned(),
            ];
            arguments.extend(paths.iter().map(|path| literal_pathspec(path)));
            for record in self.bytes(arguments, None)?.split(|byte| *byte == 0).filter(|record| !record.is_empty()) {
                let Some(tab) = record.iter().position(|byte| *byte == b'\t') else {
                    return Err(AppError::operational("malformed Git tree entry"));
                };
                let metadata = std::str::from_utf8(&record[..tab])
                    .map_err(|_| AppError::operational("Git returned a non-UTF-8 tree entry"))?;
                let path = std::str::from_utf8(&record[tab + 1..])
                    .map_err(|_| AppError::usage("non-UTF-8 repository paths are not supported"))?
                    .to_owned();
                validate_safe_path_text(&path)?;
                let mut fields = metadata.split_ascii_whitespace();
                let mode = fields.next().ok_or_else(|| AppError::operational("malformed Git tree entry"))?.to_owned();
                let kind = fields.next().ok_or_else(|| AppError::operational("malformed Git tree entry"))?.to_owned();
                let oid = fields.next().ok_or_else(|| AppError::operational("malformed Git tree entry"))?.to_owned();
                entries.insert(path, TreeEntry { mode, kind, oid });
            }
        }
        Ok(entries)
    }

    pub fn create_refs(&self, refs: &[(&str, &str)]) -> Result<()> {
        let mut input = String::from("start\n");
        for (reference, oid) in refs {
            input.push_str(&format!("create {reference} {oid}\n"));
        }
        input.push_str("prepare\ncommit\n");
        let output = self.with_input(["update-ref", "--stdin"], input.as_bytes(), None)?;
        if output.status.success() { Ok(()) } else { Err(git_error(output)) }
    }

    pub fn delete_refs(&self, refs: &[String]) -> Result<()> {
        let mut input = String::from("start\n");
        for reference in refs {
            input.push_str(&format!("delete {reference}\n"));
        }
        input.push_str("prepare\ncommit\n");
        let output = self.with_input(["update-ref", "--stdin"], input.as_bytes(), None)?;
        if output.status.success() { Ok(()) } else { Err(git_error(output)) }
    }

    pub fn update_refs(&self, refs: &[(&str, &str, &str)]) -> Result<()> {
        let mut input = String::from("start\n");
        for (reference, new_oid, old_oid) in refs {
            input.push_str(&format!("update {reference} {new_oid} {old_oid}\n"));
        }
        input.push_str("prepare\ncommit\n");
        let output = self.with_input(["update-ref", "--stdin"], input.as_bytes(), None)?;
        if output.status.success() { Ok(()) } else { Err(git_error(output)) }
    }
}

pub fn literal_pathspec(path: &str) -> String {
    format!(":(top,literal){path}")
}

pub fn output_text(output: Output) -> Result<String> {
    if !output.status.success() {
        return Err(git_error(output));
    }
    let text = String::from_utf8(output.stdout).map_err(|_| AppError::operational("Git returned non-UTF-8 output"))?;
    Ok(text.trim_end_matches(['\r', '\n']).to_owned())
}

pub fn git_error(output: Output) -> AppError {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let detail = if stderr.is_empty() { stdout } else { stderr };
    AppError::operational(if detail.is_empty() { "git command failed".to_owned() } else { detail })
}

pub fn decode_nul_paths(bytes: &[u8]) -> Result<Vec<String>> {
    bytes
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
        .map(|record| {
            let path = std::str::from_utf8(record)
                .map_err(|_| AppError::usage("non-UTF-8 repository paths are not supported"))?
                .to_owned();
            validate_safe_path_text(&path)?;
            Ok(path)
        })
        .collect()
}

pub fn validate_safe_path_text(path: &str) -> Result<()> {
    if path.is_empty() || path.chars().any(char::is_control) {
        return Err(AppError::usage("repository paths may not be empty or contain control characters"));
    }
    Ok(())
}

pub fn copy_file(source: &Path, destination: &Path) -> Result<()> {
    fs::copy(source, destination)
        .map(|_| ())
        .map_err(|error| AppError::operational(format!("cannot copy Git index {}: {error}", source.display())))
}

fn command_text(command: &mut Command) -> std::result::Result<String, String> {
    let output = command.output().map_err(|error| error.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim_end_matches(['\r', '\n']).to_owned())
        .map_err(|error| error.to_string())
}
