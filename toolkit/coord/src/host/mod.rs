mod git;
mod identity;
mod process;
mod providers;

pub(crate) use git::*;
pub(crate) use identity::*;
pub(crate) use process::*;
pub(crate) use providers::*;

pub(crate) fn path_text(path: &std::path::Path) -> crate::error::Result<String> {
    path.to_str().map(str::to_owned).ok_or_else(|| crate::error::AppError::usage("path is not valid UTF-8"))
}

pub(crate) fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").filter(|value| !value.is_empty()).map(std::path::PathBuf::from)
}
