mod git;
mod identity;
mod process;
mod providers;

pub(crate) use git::*;
pub(crate) use identity::*;
pub(crate) use process::*;
pub(crate) use providers::*;

fn hex_bytes(bytes: &[u8]) -> String {
    hex::encode(bytes)
}

pub(crate) fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").filter(|value| !value.is_empty()).map(std::path::PathBuf::from)
}
