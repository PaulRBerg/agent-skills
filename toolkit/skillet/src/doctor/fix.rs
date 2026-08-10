use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use regex::Regex;

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

pub fn create_metadata(path: &Path, expected: bool) -> io::Result<()> {
    let contents = format!("policy:\n  allow_implicit_invocation: {expected}\n");
    atomic_write(path, contents.as_bytes(), WriteMode::Create)
}

pub fn update_policy(path: &Path, original: &str, expected: bool) -> io::Result<()> {
    let updated = replace_policy_value(original, expected).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "could not locate a unique block-style policy.allow_implicit_invocation boolean",
        )
    })?;
    atomic_write(path, updated.as_bytes(), WriteMode::Replace)
}

fn replace_policy_value(original: &str, expected: bool) -> Option<String> {
    let mut policy_start = None;
    let mut policy_end = original.len();
    let mut offset = 0usize;
    for line in original.split_inclusive('\n') {
        let content = line.trim_end_matches(['\n', '\r']);
        let top_level = !content.is_empty()
            && !content.starts_with([' ', '\t', '#'])
            && content != "---"
            && content != "...";
        if policy_start.is_none() {
            if top_level
                && content.strip_prefix("policy:").is_some_and(|tail| {
                    tail.trim().is_empty() || tail.trim_start().starts_with('#')
                })
            {
                policy_start = Some(offset + line.len());
            }
        } else if top_level {
            policy_end = offset;
            break;
        }
        offset += line.len();
    }
    let start = policy_start?;
    let policy = &original[start..policy_end];
    let pattern = Regex::new(
        r"(?m)^([ \t]+allow_implicit_invocation:[ \t]*)(true|false)([ \t]*(?:#[^\r\n]*)?\r?)$",
    )
    .expect("static policy regex is valid");
    let mut captures = pattern.captures_iter(policy);
    let first = captures.next()?;
    if captures.next().is_some() {
        return None;
    }
    let value = first.get(2)?;
    let absolute_start = start + value.start();
    let absolute_end = start + value.end();
    let mut updated = String::with_capacity(original.len());
    updated.push_str(&original[..absolute_start]);
    updated.push_str(if expected { "true" } else { "false" });
    updated.push_str(&original[absolute_end..]);
    Some(updated)
}

#[derive(Clone, Copy)]
enum WriteMode {
    Create,
    Replace,
}

fn atomic_write(path: &Path, contents: &[u8], mode: WriteMode) -> io::Result<()> {
    let parent = path.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "metadata path has no parent directory")
    })?;
    let created_parent = if parent.exists() {
        if !parent.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::NotADirectory,
                format!("{} is not a directory", parent.display()),
            ));
        }
        false
    } else {
        fs::create_dir(parent)?;
        true
    };

    let result = stage_and_rename(path, parent, contents, mode);
    if result.is_err() && created_parent {
        let _ = fs::remove_dir(parent);
    }
    result
}

fn stage_and_rename(
    path: &Path,
    parent: &Path,
    contents: &[u8],
    mode: WriteMode,
) -> io::Result<()> {
    match mode {
        WriteMode::Create if path.exists() => {
            return Err(io::Error::new(io::ErrorKind::AlreadyExists, "target appeared during fix"));
        }
        WriteMode::Replace if !path.is_file() => {
            return Err(io::Error::new(io::ErrorKind::NotFound, "target disappeared during fix"));
        }
        _ => {}
    }

    let permissions = match mode {
        WriteMode::Replace => Some(fs::metadata(path)?.permissions()),
        WriteMode::Create => None,
    };
    let (temporary, mut file) = create_temporary(parent, path)?;
    let staged = (|| {
        file.write_all(contents)?;
        if let Some(permissions) = permissions {
            file.set_permissions(permissions)?;
        }
        file.sync_all()?;
        drop(file);

        match mode {
            WriteMode::Create if path.exists() => {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "target appeared during fix",
                ));
            }
            WriteMode::Replace if !path.is_file() => {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    "target disappeared during fix",
                ));
            }
            _ => {}
        }
        fs::rename(&temporary, path)
    })();
    if staged.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    staged
}

fn create_temporary(parent: &Path, path: &Path) -> io::Result<(PathBuf, fs::File)> {
    let file_name = path.file_name().unwrap_or_default();
    for _ in 0..100 {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let mut name = OsString::from(".");
        name.push(file_name);
        name.push(format!(".ai-skillet-{}-{sequence}.tmp", std::process::id()));
        let temporary = parent.join(name);
        match OpenOptions::new().write(true).create_new(true).open(&temporary) {
            Ok(file) => return Ok((temporary, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate a unique temporary metadata file",
    ))
}

#[cfg(test)]
mod tests {
    use super::replace_policy_value;

    #[test]
    fn replacement_is_scoped_and_byte_preserving() {
        let source = "interface:\n  allow_implicit_invocation: true\npolicy:\n  other: kept\n  allow_implicit_invocation: false # note\nui:\n  title: Demo\n";
        let updated = replace_policy_value(source, true).unwrap();
        assert_eq!(
            updated,
            "interface:\n  allow_implicit_invocation: true\npolicy:\n  other: kept\n  allow_implicit_invocation: true # note\nui:\n  title: Demo\n"
        );
    }
}
