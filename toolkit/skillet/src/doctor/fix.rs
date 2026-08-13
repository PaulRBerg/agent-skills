use std::{
    fs,
    io::{self, Write},
    path::Path,
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use regex::Regex;
#[cfg(test)]
use tempfile::NamedTempFile;

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
        let top_level =
            !content.is_empty() && !content.starts_with([' ', '\t', '#']) && content != "---" && content != "...";
        if policy_start.is_none() {
            if top_level &&
                content
                    .strip_prefix("policy:")
                    .is_some_and(|tail| tail.trim().is_empty() || tail.trim_start().starts_with('#'))
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
    let pattern = Regex::new(r"(?m)^([ \t]+allow_implicit_invocation:[ \t]*)(true|false)([ \t]*(?:#[^\r\n]*)?\r?)$")
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
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "metadata path has no parent directory"))?;
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

    let result = stage_and_persist(path, parent, contents, mode);
    if result.is_err() && created_parent {
        let _ = fs::remove_dir(parent);
    }
    result
}

fn stage_and_persist(path: &Path, parent: &Path, contents: &[u8], mode: WriteMode) -> io::Result<()> {
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
    let mut builder = tempfile::Builder::new();
    #[cfg(unix)]
    builder.permissions(fs::Permissions::from_mode(0o666));
    let mut temporary = builder.tempfile_in(parent)?;
    (|| {
        temporary.write_all(contents)?;
        if let Some(permissions) = permissions {
            temporary.as_file().set_permissions(permissions)?;
        }
        temporary.as_file().sync_all()?;

        match mode {
            WriteMode::Create if path.exists() => {
                return Err(io::Error::new(io::ErrorKind::AlreadyExists, "target appeared during fix"));
            }
            WriteMode::Replace if !path.is_file() => {
                return Err(io::Error::new(io::ErrorKind::NotFound, "target disappeared during fix"));
            }
            _ => {}
        }
        match mode {
            WriteMode::Create => temporary.persist_noclobber(path).map(|_| ()).map_err(|error| error.error),
            WriteMode::Replace => temporary.persist(path).map(|_| ()).map_err(|error| error.error),
        }
    })()
}

#[cfg(test)]
fn publish_staged(temporary: &Path, path: &Path, mode: WriteMode) -> io::Result<()> {
    let file = fs::OpenOptions::new().read(true).write(true).open(temporary)?;
    let temporary = NamedTempFile::from_parts(file, tempfile::TempPath::try_from_path(temporary)?);
    match mode {
        WriteMode::Create => match temporary.persist_noclobber(path) {
            Ok(_) => Ok(()),
            Err(error) => {
                let source = error.error;
                let _ = error.file.keep();
                Err(source)
            }
        },
        WriteMode::Replace => temporary.persist(path).map(|_| ()).map_err(|error| error.error),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{WriteMode, publish_staged, replace_policy_value};

    #[test]
    fn replacement_is_scoped_and_byte_preserving() {
        let source = "interface:\n  allow_implicit_invocation: true\npolicy:\n  other: kept\n  allow_implicit_invocation: false # note\nui:\n  title: Demo\n";
        let updated = replace_policy_value(source, true).unwrap();
        assert_eq!(
            updated,
            "interface:\n  allow_implicit_invocation: true\npolicy:\n  other: kept\n  allow_implicit_invocation: true # note\nui:\n  title: Demo\n"
        );
    }

    #[test]
    fn create_publish_never_replaces_an_existing_target() {
        let directory = tempdir().unwrap();
        let temporary = directory.path().join("staged");
        let target = directory.path().join("openai.yaml");
        fs::write(&temporary, "new\n").unwrap();
        fs::write(&target, "existing\n").unwrap();

        let error = publish_staged(&temporary, &target, WriteMode::Create).unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read_to_string(&target).unwrap(), "existing\n");
        assert_eq!(fs::read_to_string(&temporary).unwrap(), "new\n");
    }
}
