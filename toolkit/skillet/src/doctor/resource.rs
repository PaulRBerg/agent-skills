use std::{
    fs,
    path::{Component, Path, PathBuf},
};

pub(super) fn resource_target(skill_directory: &Path, reference: &str) -> Option<PathBuf> {
    let relative = Path::new(reference);
    if !relative.components().all(|component| matches!(component, Component::Normal(_) | Component::CurDir)) {
        return None;
    }

    let target = skill_directory.join(relative);
    let canonical_skill_directory = fs::canonicalize(skill_directory).ok()?;
    match fs::canonicalize(&target) {
        Ok(canonical_target) => canonical_target.starts_with(&canonical_skill_directory).then_some(target),
        Err(_) if !target.exists() => Some(target),
        Err(_) => None,
    }
}
