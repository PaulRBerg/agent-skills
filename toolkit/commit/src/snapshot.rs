use std::{
    collections::BTreeSet,
    env,
    fs::{self, File, OpenOptions},
    io::ErrorKind,
    path::{Component, Path, PathBuf},
    time::{Duration, SystemTime},
};

use fs4::FileExt;
use tempfile::{Builder, TempDir};

use crate::{
    error::{AppError, Result},
    git::{Repository, copy_file, decode_nul_paths},
    state::Store,
};

const SNAPSHOT_PREFIX: &str = "ai-commit-snapshot-";
const OWNER_LOCK: &str = "lock";
/// A snapshot container this old whose owner lock is free (or was never created) belongs to an
/// interrupted ai-commit process. Owners create and lock it immediately after creating the container.
const STALE_AFTER: Duration = Duration::from_secs(5 * 60);

/// A complete materialization of a prepared candidate, kept outside the repository so upward
/// lookups from it never reach the physical worktree.
pub(crate) struct ValidationSnapshot {
    _container: TempDir,
    worktree: PathBuf,
    validation_index: PathBuf,
    // Declared after `_container` so the directory is removed while the owner lock is still held.
    _owner_lock: File,
}

impl ValidationSnapshot {
    pub(crate) fn materialize(
        repository: &Repository,
        index: &Path,
        candidate_tree: &str,
        validation_index: &Path,
    ) -> Result<Self> {
        let git_dir = repository.git_dir()?;
        let parent = snapshot_parent()?;
        sweep_stale_snapshots(&parent);
        let mut builder = Builder::new();
        builder.prefix(SNAPSHOT_PREFIX);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            builder.permissions(fs::Permissions::from_mode(0o700));
        }
        let container = builder.tempdir_in(&parent).map_err(|error| {
            AppError::retry(format!("cannot create temporary hook worktree under {}: {error}", parent.display()))
        })?;
        let owner_lock = lock_owner(container.path())?;
        let worktree = container.path().join("worktree");
        fs::create_dir(&worktree)
            .map_err(|error| AppError::retry(format!("cannot create temporary hook worktree: {error}")))?;
        if env::var_os("AI_COMMIT_TEST_FAIL_SNAPSHOT_MATERIALIZATION").is_some() {
            return Err(AppError::retry("injected snapshot materialization failure"));
        }
        repository
            .checked_in_worktree(["checkout-index", "--all", "--force"], Some(index), &worktree)
            .map_err(|error| AppError::retry(format!("cannot materialize temporary hook worktree: {error}")))?;
        repository
            .checked_in_worktree(["update-index", "--refresh"], Some(index), &worktree)
            .map_err(|error| AppError::retry(format!("cannot validate temporary hook worktree: {error}")))?;
        let materialized_tree = repository.text(["write-tree"], Some(index))?;
        if materialized_tree != candidate_tree {
            return Err(AppError::operational("temporary hook worktree materialization changed the prepared tree"));
        }
        fs::write(worktree.join(".git"), format!("gitdir: {}\n", git_dir.display()))
            .map_err(|error| AppError::retry(format!("cannot configure temporary hook worktree: {error}")))?;
        project_ignored_directories(repository, index, &worktree)?;
        project_submodules(repository, candidate_tree, &worktree)?;
        copy_file(index, validation_index)?;
        Ok(Self {
            _container: container,
            worktree,
            validation_index: validation_index.to_path_buf(),
            _owner_lock: owner_lock,
        })
    }

    pub(crate) fn root(&self) -> &Path {
        &self.worktree
    }

    pub(crate) fn validation_index(&self) -> &Path {
        &self.validation_index
    }
}

fn snapshot_parent() -> Result<PathBuf> {
    let store = Store::discover()?;
    // Canonical so hooks comparing `pwd -P` with GIT_WORK_TREE see one spelling.
    store.temporary().canonicalize().map_err(|error| {
        AppError::retry(format!("cannot resolve temporary state directory {}: {error}", store.temporary().display()))
    })
}

fn lock_owner(container: &Path) -> Result<File> {
    let path = container.join(OWNER_LOCK);
    let file =
        OpenOptions::new().read(true).write(true).create_new(true).open(&path).map_err(|error| {
            AppError::retry(format!("cannot create snapshot owner lock {}: {error}", path.display()))
        })?;
    FileExt::try_lock(&file)
        .map_err(|error| AppError::retry(format!("cannot lock snapshot owner lock {}: {error:?}", path.display())))?;
    Ok(file)
}

/// Best-effort removal of snapshot containers abandoned by interrupted ai-commit processes. Only real
/// directories named with the snapshot prefix are considered; a live owner keeps its lock held.
fn sweep_stale_snapshots(parent: &Path) {
    let Ok(entries) = fs::read_dir(parent) else {
        return;
    };
    let now = SystemTime::now();
    for entry in entries.flatten() {
        if !entry.file_name().to_string_lossy().starts_with(SNAPSHOT_PREFIX) {
            continue;
        }
        let Ok(metadata) = fs::symlink_metadata(entry.path()) else {
            continue;
        };
        if !metadata.is_dir() {
            continue;
        }
        let old_enough = metadata
            .modified()
            .ok()
            .and_then(|modified| now.duration_since(modified).ok())
            .is_some_and(|age| age >= STALE_AFTER);
        if !old_enough {
            continue;
        }
        let container = entry.path();
        let lock = match OpenOptions::new().read(true).write(true).open(container.join(OWNER_LOCK)) {
            Ok(file) => match FileExt::try_lock(&file) {
                Ok(()) => Some(file),
                Err(_) => continue,
            },
            Err(error) if error.kind() == ErrorKind::NotFound => None,
            Err(_) => continue,
        };
        let _ = fs::remove_dir_all(&container);
        drop(lock);
    }
}

fn project_ignored_directories(repository: &Repository, index: &Path, worktree: &Path) -> Result<()> {
    let untracked = repository.bytes(["ls-files", "--others", "--directory", "-z"], Some(index))?;
    let mut records = Vec::<(String, bool)>::new();
    for record in untracked.split(|byte| *byte == 0).filter(|record| !record.is_empty()) {
        if let Some(directory) = record.strip_suffix(b"/") {
            records.extend(decode_nul_paths(directory)?.into_iter().map(|path| (path, true)));
        } else if let Ok(paths) = decode_nul_paths(record) {
            // Git lists a symlink to a directory without the trailing slash; other untracked files
            // are not projected, so their unsupported names are skipped rather than rejected.
            for path in paths {
                let physical = repository.root.join(&path);
                let is_symlink = fs::symlink_metadata(&physical).is_ok_and(|metadata| metadata.is_symlink());
                if is_symlink && fs::metadata(&physical).is_ok_and(|metadata| metadata.is_dir()) {
                    records.push((path, false));
                }
            }
        }
    }
    records.sort_by(|(left, _), (right, _)| {
        Path::new(left).components().count().cmp(&Path::new(right).components().count()).then_with(|| left.cmp(right))
    });
    let mut roots = Vec::<PathBuf>::new();
    let mut prepared = Vec::<(String, bool, PathBuf, PathBuf)>::new();
    for (directory, real_directory) in records {
        let relative = PathBuf::from(&directory);
        if roots.iter().any(|ancestor| relative.starts_with(ancestor)) {
            continue;
        }
        roots.push(relative.clone());
        let source = repository.root.join(&relative);
        let metadata = match fs::metadata(&source) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == ErrorKind::NotFound => continue,
            Err(error) => {
                return Err(AppError::retry(format!(
                    "cannot inspect ignored local directory {}: {error}",
                    source.display()
                )));
            }
        };
        if !metadata.is_dir() {
            continue;
        }

        let destination = worktree.join(&relative);
        match fs::symlink_metadata(&destination) {
            Ok(_) => continue,
            Err(error) if error.kind() == ErrorKind::NotFound => {}
            Err(error) => {
                return Err(AppError::retry(format!(
                    "cannot inspect prepared local-artifact path {}: {error}",
                    destination.display()
                )));
            }
        }
        let Some(parent) = destination.parent() else {
            continue;
        };
        if !parent.is_dir() {
            continue;
        }
        // A real directory placeholder lets directory-only ignore patterns match; Git treats a
        // symlink as a non-directory, so symlinks are checked without one.
        if real_directory {
            fs::create_dir(&destination).map_err(|error| {
                AppError::retry(format!(
                    "cannot create prepared local-artifact directory {}: {error}",
                    destination.display()
                ))
            })?;
        }
        prepared.push((directory, real_directory, source, destination));
    }

    let candidates = prepared.iter().map(|(relative, ..)| relative.clone()).collect::<Vec<_>>();
    let ignored = repository.ignored_paths_in_worktree(&candidates, index, worktree).map_err(|error| {
        AppError::retry(format!("cannot inspect ignored local directories for prepared validation: {error}"))
    })?;
    let ignored = ignored.into_iter().collect::<BTreeSet<_>>();
    let projection = Projection { physical_root: &repository.root, snapshot_root: worktree };
    for (relative, real_directory, source, destination) in prepared {
        match (ignored.contains(&relative), real_directory) {
            (true, true) => projection.entries(&source, &destination, true)?,
            (true, false) => projection.link(&source, &destination)?,
            (false, true) => fs::remove_dir(&destination).map_err(|error| {
                AppError::retry(format!(
                    "cannot remove unused prepared local-artifact directory {}: {error}",
                    destination.display()
                ))
            })?,
            (false, false) => {}
        }
    }
    Ok(())
}

/// Exposes an initialized submodule whose checked-out HEAD is the candidate's gitlink commit.
/// Entries are projected into the empty gitlink directory (never its `.git`), so Git still sees an
/// unpopulated directory and reports no drift. Other gitlinks stay empty.
fn project_submodules(repository: &Repository, candidate_tree: &str, worktree: &Path) -> Result<()> {
    let gitmodules = worktree.join(".gitmodules");
    if !fs::symlink_metadata(&gitmodules).is_ok_and(|metadata| metadata.is_file()) {
        return Ok(());
    }
    let gitmodules_text = gitmodules.to_string_lossy().into_owned();
    let Ok(listing) = repository
        .bytes(["config", "--file", gitmodules_text.as_str(), "-z", "--get-regexp", r"^submodule\..*\.path$"], None)
    else {
        return Ok(());
    };
    let paths = listing
        .split(|byte| *byte == 0)
        .filter_map(|record| std::str::from_utf8(record).ok())
        .filter_map(|record| record.split_once('\n').map(|(_, path)| path.to_owned()))
        .filter(|path| !path.is_empty())
        .collect::<Vec<_>>();
    if paths.is_empty() {
        return Ok(());
    }
    let entries = repository.tree_file_entries(candidate_tree, &paths)?;
    let projection = Projection { physical_root: &repository.root, snapshot_root: worktree };
    for (path, entry) in entries {
        if entry.kind != "commit" {
            continue;
        }
        let physical = repository.root.join(&path);
        if fs::symlink_metadata(physical.join(".git")).is_err() {
            continue;
        }
        let submodule = Repository { root: physical.clone() };
        let Ok(head) = submodule.text(["rev-parse", "--verify", "--quiet", "HEAD"], None) else {
            continue;
        };
        let destination = worktree.join(&path);
        if head != entry.oid || !fs::symlink_metadata(&destination).is_ok_and(|metadata| metadata.is_dir()) {
            continue;
        }
        projection.submodule_entries(&physical, &destination)?;
    }
    Ok(())
}

struct Projection<'a> {
    physical_root: &'a Path,
    snapshot_root: &'a Path,
}

impl Projection<'_> {
    /// Projects one directory level: symlinks are recreated, `@scope` directories get one more
    /// level, and everything else links to its physical path.
    fn entries(&self, source: &Path, destination: &Path, expand_scopes: bool) -> Result<()> {
        for entry in read_entries(source)? {
            let linked_path = destination.join(entry.file_name());
            let file_type = entry
                .file_type()
                .map_err(|error| AppError::retry(format!("cannot inspect {}: {error}", entry.path().display())))?;
            if file_type.is_symlink() {
                self.link(&entry.path(), &linked_path)?;
            } else if expand_scopes && file_type.is_dir() && entry.file_name().to_string_lossy().starts_with('@') {
                fs::create_dir(&linked_path).map_err(|error| {
                    AppError::retry(format!(
                        "cannot create prepared local-artifact directory {}: {error}",
                        linked_path.display()
                    ))
                })?;
                self.entries(&entry.path(), &linked_path, false)?;
            } else {
                symlink(&entry.path(), &linked_path)?;
            }
        }
        Ok(())
    }

    fn submodule_entries(&self, source: &Path, destination: &Path) -> Result<()> {
        for entry in read_entries(source)? {
            if entry.file_name() == ".git" {
                continue;
            }
            let linked_path = destination.join(entry.file_name());
            if entry.file_type().is_ok_and(|file_type| file_type.is_symlink()) {
                self.link(&entry.path(), &linked_path)?;
            } else {
                symlink(&entry.path(), &linked_path)?;
            }
        }
        Ok(())
    }

    /// Recreates the physical symlink `source` at `destination`. Targets inside the repository
    /// resolve to the same location in the snapshot (relative targets are kept verbatim); targets
    /// outside it resolve to the same physical location.
    fn link(&self, source: &Path, destination: &Path) -> Result<()> {
        let target = fs::read_link(source).map_err(|error| {
            AppError::retry(format!("cannot read ignored local symlink {}: {error}", source.display()))
        })?;
        let resolved = lexical_join(source.parent().unwrap_or(self.physical_root), &target);
        let projected = match resolved.strip_prefix(self.physical_root) {
            Ok(_) if target.is_relative() => target,
            Ok(inside) => self.snapshot_root.join(inside),
            Err(_) => resolved,
        };
        symlink(&projected, destination)
    }
}

fn read_entries(source: &Path) -> Result<Vec<fs::DirEntry>> {
    fs::read_dir(source)
        .and_then(Iterator::collect)
        .map_err(|error| AppError::retry(format!("cannot read ignored local directory {}: {error}", source.display())))
}

fn symlink(target: &Path, linked_path: &Path) -> Result<()> {
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, linked_path).map_err(|error| {
        AppError::retry(format!(
            "cannot expose ignored local artifact at {} for prepared validation: {error}",
            linked_path.display()
        ))
    })?;
    #[cfg(not(unix))]
    let _ = (target, linked_path);
    Ok(())
}

fn lexical_join(base: &Path, target: &Path) -> PathBuf {
    let mut resolved = PathBuf::new();
    for component in base.join(target).components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                resolved.pop();
            }
            other => resolved.push(other),
        }
    }
    resolved
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexical_join_resolves_relative_and_absolute_targets() {
        let base = Path::new("/repo/node_modules/@scope");
        assert_eq!(lexical_join(base, Path::new("../../packages/b")), PathBuf::from("/repo/packages/b"));
        assert_eq!(lexical_join(base, Path::new("./x/../y")), PathBuf::from("/repo/node_modules/@scope/y"));
        assert_eq!(lexical_join(base, Path::new("/elsewhere/lib")), PathBuf::from("/elsewhere/lib"));
    }

    #[test]
    fn sweep_removes_only_abandoned_snapshot_containers() {
        let parent = tempfile::tempdir().unwrap();
        let old = SystemTime::now() - STALE_AFTER - Duration::from_secs(60);
        let make = |name: &str, lock: bool, stale: bool| {
            let path = parent.path().join(name);
            fs::create_dir(&path).unwrap();
            fs::write(path.join("content"), "x").unwrap();
            if lock {
                fs::write(path.join(OWNER_LOCK), "").unwrap();
            }
            if stale {
                File::open(&path).unwrap().set_modified(old).unwrap();
            }
            path
        };
        let abandoned = make(&format!("{SNAPSHOT_PREFIX}abandoned"), true, true);
        let unlocked = make(&format!("{SNAPSHOT_PREFIX}unlocked"), false, true);
        let live = make(&format!("{SNAPSHOT_PREFIX}live"), true, true);
        let held = OpenOptions::new().read(true).write(true).open(live.join(OWNER_LOCK)).unwrap();
        FileExt::try_lock(&held).unwrap();
        let fresh = make(&format!("{SNAPSHOT_PREFIX}fresh"), true, false);
        let unrelated = make("unrelated-stale", true, true);
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("keep"), "x").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(outside.path(), parent.path().join(format!("{SNAPSHOT_PREFIX}link"))).unwrap();

        sweep_stale_snapshots(parent.path());

        assert!(!abandoned.exists());
        assert!(!unlocked.exists());
        assert!(live.join("content").exists());
        assert!(fresh.join("content").exists());
        assert!(unrelated.join("content").exists());
        assert!(outside.path().join("keep").exists());
        drop(held);
    }
}
