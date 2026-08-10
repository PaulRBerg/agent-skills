use std::path::{Component, Path, PathBuf};

pub(super) fn resource_target(skill_directory: &Path, reference: &str) -> Option<PathBuf> {
    let relative = Path::new(reference);
    relative
        .components()
        .all(|component| matches!(component, Component::Normal(_) | Component::CurDir))
        .then(|| skill_directory.join(relative))
}
