use std::{
    fmt::Write as _,
    fs::{self, File, Metadata},
    io::{BufReader, Read},
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

use crate::{
    error::Error,
    exclusions::{agent_state_path, directory_name_is_excluded},
};

const COPY_BUFFER_BYTES: usize = 1024 * 1024;

pub fn sha256_file(path: &Path) -> Result<String, Error> {
    let file = File::open(path).map_err(|error| Error::io("hash", path, error))?;
    let mut reader = BufReader::with_capacity(COPY_BUFFER_BYTES, file);
    let mut digest = Sha256::new();
    let mut buffer = vec![0; COPY_BUFFER_BYTES];
    loop {
        let read = reader.read(&mut buffer).map_err(|error| Error::io("hash", path, error))?;
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    Ok(finalize_hex(digest))
}

pub fn sha256_tree(directory: &Path) -> Result<String, Error> {
    let mut entries = Vec::new();
    collect_entries(directory, Path::new(""), &mut entries)?;
    entries.sort_by(|left, right| encoded_path(&left.0).cmp(encoded_path(&right.0)));

    let mut digest = Sha256::new();
    let mut buffer = vec![0; COPY_BUFFER_BYTES];
    for (relative, path, metadata) in entries {
        hash_field(&mut digest, encoded_path(&relative));
        let file_type = metadata.file_type();
        let entry_type = if file_type.is_file() {
            b"file".as_slice()
        } else if file_type.is_dir() {
            b"directory".as_slice()
        } else if file_type.is_symlink() {
            b"symlink".as_slice()
        } else {
            b"other".as_slice()
        };
        hash_field(&mut digest, entry_type);

        let executable = if file_type.is_file() { executable_bits(&metadata) } else { 0 };
        hash_field(&mut digest, format!("{executable:03o}").as_bytes());

        if file_type.is_file() {
            hash_field(&mut digest, &metadata.len().to_be_bytes());
            let file = File::open(&path).map_err(|error| Error::io("hash", &path, error))?;
            let mut reader = BufReader::with_capacity(COPY_BUFFER_BYTES, file);
            loop {
                let read = reader.read(&mut buffer).map_err(|error| Error::io("hash", &path, error))?;
                if read == 0 {
                    break;
                }
                digest.update(&buffer[..read]);
            }
        } else if file_type.is_symlink() {
            let target = fs::read_link(&path).map_err(|error| Error::io("read symlink while hashing", &path, error))?;
            hash_field(&mut digest, target.as_os_str().as_encoded_bytes());
        }
    }
    Ok(finalize_hex(digest))
}

fn finalize_hex(digest: Sha256) -> String {
    let bytes = digest.finalize();
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        write!(output, "{byte:02x}").expect("writing to a String cannot fail");
    }
    output
}

fn collect_entries(
    directory: &Path,
    relative_directory: &Path,
    entries: &mut Vec<(PathBuf, PathBuf, Metadata)>,
) -> Result<(), Error> {
    let current = directory.join(relative_directory);
    let children = fs::read_dir(&current).map_err(|error| Error::io("read skill tree", &current, error))?;
    for child in children {
        let child = child.map_err(|error| Error::io("read skill tree", &current, error))?;
        let relative = relative_directory.join(child.file_name());
        if tree_path_is_ignored(&relative) {
            continue;
        }
        let path = child.path();
        let metadata =
            fs::symlink_metadata(&path).map_err(|error| Error::io("inspect skill tree entry", &path, error))?;
        let is_directory = metadata.file_type().is_dir();
        entries.push((relative.clone(), path, metadata));
        if is_directory {
            collect_entries(directory, &relative, entries)?;
        }
    }
    Ok(())
}

fn tree_path_is_ignored(relative: &Path) -> bool {
    relative.components().any(|component| directory_name_is_excluded(component.as_os_str())) ||
        agent_state_path(relative)
}

fn encoded_path(path: &Path) -> &[u8] {
    path.as_os_str().as_encoded_bytes()
}

fn hash_field(digest: &mut Sha256, value: &[u8]) {
    digest.update((value.len() as u64).to_be_bytes());
    digest.update(value);
}

#[cfg(unix)]
fn executable_bits(metadata: &Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;

    metadata.permissions().mode() & 0o111
}

#[cfg(not(unix))]
fn executable_bits(_metadata: &Metadata) -> u32 {
    0
}
