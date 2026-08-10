use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use tempfile::NamedTempFile;

use crate::error::{AppError, Result};

/// Atomically replaces a file while preserving a final symlink and existing Unix mode.
///
/// The temporary file is created next to the resolved target, so `persist` cannot cross
/// filesystems. A broken final symlink is rejected instead of replacing the link itself.
pub fn replace(path: &Path, contents: impl AsRef<[u8]>) -> Result<()> {
    let target = resolve_target(path)?;
    let parent = target.parent().filter(|value| !value.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    #[cfg(unix)]
    let mode = target.metadata().map(|metadata| mode(&metadata)).unwrap_or(0o600);

    let mut temporary = NamedTempFile::new_in(parent)?;
    temporary.write_all(contents.as_ref())?;
    temporary.as_file_mut().sync_all()?;

    #[cfg(unix)]
    fs::set_permissions(temporary.path(), permissions(mode))?;

    temporary.persist(&target).map_err(|error| AppError::operational(error.error.to_string()))?;
    sync_parent(parent)?;
    Ok(())
}

fn resolve_target(path: &Path) -> Result<PathBuf> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => fs::canonicalize(path)
            .map_err(|error| AppError::operational(format!("cannot resolve symlink {}: {error}", path.display()))),
        Ok(_) => Ok(path.to_path_buf()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(path.to_path_buf()),
        Err(error) => Err(error.into()),
    }
}

#[cfg(unix)]
fn mode(metadata: &fs::Metadata) -> u32 {
    use std::os::unix::fs::MetadataExt;
    metadata.mode() & 0o7777
}

#[cfg(unix)]
fn permissions(mode: u32) -> fs::Permissions {
    use std::os::unix::fs::PermissionsExt;
    fs::Permissions::from_mode(mode)
}

#[cfg(unix)]
fn sync_parent(parent: &Path) -> Result<()> {
    fs::File::open(parent)?.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn sync_parent(_parent: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn creates_and_replaces_a_file() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("config.yaml");

        replace(&path, b"first\n").unwrap();
        replace(&path, b"second\n").unwrap();

        assert_eq!(fs::read(path).unwrap(), b"second\n");
    }

    #[cfg(unix)]
    #[test]
    fn preserves_final_symlink_and_target_mode() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};

        let directory = tempdir().unwrap();
        let target = directory.path().join("target.yaml");
        let link = directory.path().join("config.yaml");
        fs::write(&target, "old").unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o640)).unwrap();
        symlink(&target, &link).unwrap();

        replace(&link, b"new").unwrap();

        assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
        assert_eq!(fs::read_to_string(target.clone()).unwrap(), "new");
        assert_eq!(fs::metadata(target).unwrap().mode() & 0o777, 0o640);
    }

    #[cfg(unix)]
    #[test]
    fn rejects_a_broken_final_symlink() {
        use std::os::unix::fs::symlink;

        let directory = tempdir().unwrap();
        let link = directory.path().join("config.yaml");
        symlink(directory.path().join("missing.yaml"), &link).unwrap();

        let error = replace(&link, b"new").unwrap_err();

        assert!(error.to_string().contains("cannot resolve symlink"));
        assert!(link.symlink_metadata().unwrap().file_type().is_symlink());
    }
}
