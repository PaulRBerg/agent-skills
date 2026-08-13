#![allow(dead_code)]

use assert_cmd::{Command as AssertCommand, prelude::OutputAssertExt};
use predicates::prelude::*;
use std::{
    ffi::OsStr,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use tempfile::TempDir;

pub struct Harness {
    _temporary: TempDir,
    pub root: PathBuf,
    pub repo: PathBuf,
    pub home: PathBuf,
    pub state: PathBuf,
    pub shim: PathBuf,
    pub search_path: String,
}

impl Harness {
    pub fn new(name: &str) -> Self {
        let temporary = tempfile::Builder::new().prefix(&format!("ai-commit-{name}-")).tempdir().unwrap();
        let root = temporary.path().to_path_buf();
        let repo = root.join("repo");
        let home = root.join("home");
        let state = root.join("state");
        let shim = root.join("shim");
        fs::create_dir_all(&repo).unwrap();
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&shim).unwrap();
        write_executable(&shim.join("ai-coord"), "#!/bin/sh\nexit 1\n");
        let inherited = std::env::var("PATH").unwrap_or_default();
        let search_path = format!("{}:{inherited}", shim.display());
        let harness = Self { _temporary: temporary, root, repo, home, state, shim, search_path };
        harness.git(["init", "--quiet"]);
        harness.git(["config", "user.name", "AI Commit Test"]);
        harness.git(["config", "user.email", "ai-commit@example.com"]);
        harness.git(["config", "commit.gpgsign", "false"]);
        harness.git(["config", "core.hooksPath", ".git/hooks"]);
        harness
    }

    pub fn write(&self, relative: &str, contents: &str) {
        let path = self.repo.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    pub fn read(&self, relative: &str) -> String {
        fs::read_to_string(self.repo.join(relative)).unwrap()
    }

    pub fn commit_all(&self, message: &str) {
        self.git(["add", "-A"]);
        self.git(["commit", "--quiet", "-m", message]);
    }

    pub fn git<I, S>(&self, args: I) -> String
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        git_at(&self.repo, &self.home, args)
    }

    pub fn git_output<I, S>(&self, args: I) -> Output
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        git_output_at(&self.repo, &self.home, args)
    }

    pub fn command<I, S>(&self, args: I) -> Output
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.command_with_env(args, std::iter::empty::<(&str, &str)>())
    }

    pub fn command_with_env<I, S, E, K, V>(&self, args: I, environment: E) -> Output
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
        E: IntoIterator<Item = (K, V)>,
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        let mut command = AssertCommand::cargo_bin("ai-commit").expect("binary should be built for integration tests");
        command
            .current_dir(&self.repo)
            .env("HOME", &self.home)
            .env("AI_COMMIT_STATE_DIR", &self.state)
            .env("XDG_CONFIG_HOME", self.root.join("config"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("AI_COMMIT_CONFIG")
            .env_remove("AI_COMMIT_TEST_COORD_TIMEOUT_MS")
            .env_remove("AI_COMMIT_TEST_FAIL_AFTER_REF_UPDATE")
            .env("PATH", &self.search_path)
            .args(args);
        command.envs(environment).output().unwrap()
    }

    pub fn command_at<I, S>(&self, directory: &Path, args: I) -> Output
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut command = AssertCommand::cargo_bin("ai-commit").expect("binary should be built for integration tests");
        command
            .current_dir(directory)
            .env("HOME", &self.home)
            .env("AI_COMMIT_STATE_DIR", &self.state)
            .env("XDG_CONFIG_HOME", self.root.join("config"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("AI_COMMIT_CONFIG")
            .env_remove("AI_COMMIT_TEST_COORD_TIMEOUT_MS")
            .env_remove("AI_COMMIT_TEST_FAIL_AFTER_REF_UPDATE")
            .env("PATH", &self.search_path)
            .args(args);
        command.output().unwrap()
    }

    pub fn git_input<I, S>(&self, args: I, input: &[u8]) -> String
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let mut child = Command::new("git")
            .arg("-C")
            .arg(&self.repo)
            .env("HOME", &self.home)
            .env("XDG_CONFIG_HOME", self.home.join(".config"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(input).unwrap();
        let output = assert_success(child.wait_with_output().unwrap(), "git");
        String::from_utf8(output.stdout).unwrap().trim_end().to_owned()
    }

    pub fn success<I, S>(&self, args: I) -> Output
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        assert_success(self.command(args), "ai-commit")
    }

    pub fn prepare(&self, paths: &[&str]) -> (String, String) {
        let mut arguments = vec!["prepare", "--porcelain", "--"];
        arguments.extend_from_slice(paths);
        let output = self.success(arguments);
        let stdout = String::from_utf8(output.stdout).unwrap();
        let id = stdout.lines().find_map(|line| line.strip_prefix("PREPARED\t")).expect("PREPARED record").to_owned();
        (id, stdout)
    }

    pub fn transaction_json(&self, id: &str) -> PathBuf {
        self.state.join("transactions").join(format!("{id}.json"))
    }
}

pub fn git_at<I, S>(repository: &Path, home: &Path, args: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = assert_success(git_output_at(repository, home, args), "git");
    String::from_utf8(output.stdout).unwrap().trim_end().to_owned()
}

pub fn git_output_at<I, S>(repository: &Path, home: &Path, args: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    Command::new("git")
        .arg("-C")
        .arg(repository)
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join(".config"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .args(args)
        .output()
        .unwrap()
}

pub fn write_executable(path: &Path, source: &str) {
    fs::write(path, source).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

pub fn exit_code(output: &Output) -> i32 {
    output.status.code().expect("process exit code")
}

fn assert_success(output: Output, command: &'static str) -> Output {
    let assertion = output.assert().append_context("command", command).code(predicate::eq(0));
    assertion.get_output().clone()
}
