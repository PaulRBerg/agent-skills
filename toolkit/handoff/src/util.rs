use std::{
    env, fs,
    path::{Path, PathBuf},
};

use crate::error::{Error, Result};

pub(crate) fn home() -> Result<PathBuf> {
    let home = env::var_os("HOME").ok_or_else(|| Error::operational("HOME is not set"))?;
    let home = PathBuf::from(home);
    if !home.is_absolute() {
        return Err(Error::operational(format!("HOME is not absolute: {}", home.display())));
    }
    Ok(strip_trailing_slashes(&home))
}

/// Drops trailing `/` characters so home-path comparisons see a canonical, slash-free prefix.
/// Never strips below the root.
fn strip_trailing_slashes(path: &Path) -> PathBuf {
    let Some(text) = path.to_str() else {
        return path.to_path_buf();
    };
    let trimmed = text.trim_end_matches('/');
    PathBuf::from(if trimmed.is_empty() { "/" } else { trimmed })
}

pub(crate) fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

pub(crate) fn absolute_regular_handoff(path: &Path) -> Result<PathBuf> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| Error::operational(format!("cannot inspect handoff {}: {error}", path.display())))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(Error::operational(format!("handoff is not a physical regular file: {}", path.display())));
    }
    let extension = path.extension().and_then(|value| value.to_str()).unwrap_or_default();
    if extension != "md" {
        return Err(Error::operational(format!("handoff must be a .md file: {}", path.display())));
    }

    let absolute = path
        .canonicalize()
        .map_err(|error| Error::operational(format!("cannot resolve handoff {}: {error}", path.display())))?;
    let parent = absolute.parent().ok_or_else(|| Error::operational("handoff has no parent directory"))?;
    let ai = parent.parent().ok_or_else(|| Error::operational("handoff is not inside .ai/task-handoffs"))?;
    if parent.file_name().and_then(|value| value.to_str()) != Some("task-handoffs") ||
        ai.file_name().and_then(|value| value.to_str()) != Some(".ai")
    {
        return Err(Error::operational(format!(
            "handoff is not directly inside .ai/task-handoffs: {}",
            path.display()
        )));
    }
    Ok(absolute)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_trailing_slashes_removes_one_or_many_without_going_below_root() {
        assert_eq!(strip_trailing_slashes(Path::new("/Users/prb/")), PathBuf::from("/Users/prb"));
        assert_eq!(strip_trailing_slashes(Path::new("/Users/prb///")), PathBuf::from("/Users/prb"));
        assert_eq!(strip_trailing_slashes(Path::new("/Users/prb")), PathBuf::from("/Users/prb"));
        assert_eq!(strip_trailing_slashes(Path::new("/")), PathBuf::from("/"));
        assert_eq!(strip_trailing_slashes(Path::new("///")), PathBuf::from("/"));
    }
}
