#![allow(dead_code)]

use assert_cmd::Command;
use std::fs;
use std::path::{Path, PathBuf};

pub fn ai_skillet() -> Command {
    Command::cargo_bin("ai-skillet").expect("binary should be built for integration tests")
}

pub fn write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("fixture parent should be created");
    }
    fs::write(path, contents).expect("fixture should be written");
}

pub fn skill(name: &str, extra: &str) -> String {
    format!("---\nname: {name}\n{extra}description: {name} description\n---\n# {name}\n")
}

pub fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(path)
}
