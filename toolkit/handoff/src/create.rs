use std::{
    collections::hash_map::RandomState,
    ffi::{OsStr, OsString},
    fs::{self, File},
    hash::BuildHasher,
    io::{Read, Write},
    os::fd::OwnedFd,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

use chrono::Utc;
use rustix::{
    fs::{self as unix_fs, AtFlags, FileType, Mode, OFlags},
    io::Errno,
};

use crate::{
    cli::CreateArgs,
    error::{Error, Result},
    git,
    util::{home, shell_quote},
};

pub(crate) fn run(arguments: CreateArgs) -> Result<()> {
    validate_filename(&arguments.filename)?;
    validate_task(&arguments.task)?;
    let repositories = canonical_repositories(&arguments.repo)?;
    let launch_repository = launch_repository(&repositories, arguments.launch_repo.as_deref())?;
    let before_work_skill = validate_before_work_skill(arguments.before_work_skill.as_deref())?;
    let placement = placement(&repositories, &arguments.filename)?;
    let existing_handoff_directory = validate_physical_parents(&placement.base)?;
    if repositories.len() == 1 && !git::is_ignored(&placement.base, &placement.relative)? {
        return Err(Error::operational(format!(
            "handoff target is not ignored by Git: {}",
            placement.relative.display()
        )));
    }
    if let Some(directory) = existing_handoff_directory {
        ensure_absent(&directory, OsStr::new(&arguments.filename), &placement.target)?;
    }

    if arguments.check {
        println!("target\t{}", placement.target.display());
        println!("launch_repo\t{}", launch_repository.display());
        println!("category\t{}", arguments.category);
        return Ok(());
    }

    let draft_path = arguments.draft.as_deref().expect("clap requires --draft unless --check");
    let draft = validate_draft(draft_path, repositories.len() > 1)?;
    let target_text = utf8_path(&placement.target, "handoff target")?;
    let launch_text = utf8_path(&launch_repository, "launch repository")?;
    let repository_text =
        repositories.iter().map(|repository| utf8_path(repository, "repository root")).collect::<Result<Vec<_>>>()?;
    let before_work_skill_text =
        before_work_skill.as_deref().map(|skill| utf8_path(skill, "before-work skill")).transpose()?;
    let home = home()?;
    let home_text = utf8_path(&home, "home directory")?;
    let category = arguments.category.to_string();
    let contents = compose(&category, launch_text, &repository_text, target_text, &arguments.task, &draft, home_text);
    let command = build_command(
        &category,
        &arguments.task,
        launch_text,
        target_text,
        &placement.relative,
        repositories.len() == 1,
        before_work_skill_text,
    );

    let mut publication = publish(&placement.base, &placement.target, &contents)?;
    if !arguments.no_clipboard {
        copy_and_verify(&command)?;
    }

    println!("handoff\t{target_text}");
    println!("launch_repo\t{launch_text}");
    println!("category\t{category}");
    println!("command\t{command}");
    publication.finish();
    Ok(())
}

struct Placement {
    base: PathBuf,
    target: PathBuf,
    relative: PathBuf,
}

fn placement(repositories: &[PathBuf], filename: &str) -> Result<Placement> {
    let relative = PathBuf::from(".ai").join("task-handoffs").join(filename);
    if repositories.len() == 1 {
        let repository = &repositories[0];
        return Ok(Placement { base: repository.clone(), target: repository.join(&relative), relative });
    }

    let desktop = home()?.join("Desktop");
    let metadata = fs::symlink_metadata(&desktop)
        .map_err(|_| Error::operational(format!("Desktop directory is unavailable: {}", desktop.display())))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(Error::operational(format!("Desktop directory is unavailable: {}", desktop.display())));
    }
    let desktop = desktop
        .canonicalize()
        .map_err(|error| Error::operational(format!("cannot resolve Desktop directory: {error}")))?;
    Ok(Placement { target: desktop.join(&relative), base: desktop, relative })
}

fn canonical_repositories(candidates: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut repositories = Vec::new();
    for candidate in candidates {
        let repository = git::canonical_root(candidate)?;
        if !repositories.contains(&repository) {
            repositories.push(repository);
        }
    }
    if repositories.is_empty() {
        return Err(Error::usage("at least one repository is required"));
    }
    Ok(repositories)
}

fn launch_repository(repositories: &[PathBuf], candidate: Option<&Path>) -> Result<PathBuf> {
    let launch = match candidate {
        Some(candidate) => git::canonical_root(candidate)?,
        None if repositories.len() == 1 => repositories[0].clone(),
        None => return Err(Error::usage("--launch-repo is required for cross-repository handoffs")),
    };
    if !repositories.contains(&launch) {
        return Err(Error::usage(format!("launch repository is not an involved repository: {}", launch.display())));
    }
    Ok(launch)
}

fn validate_filename(filename: &str) -> Result<()> {
    let Some(stem) = filename.strip_suffix(".md") else {
        return Err(Error::usage(format!("invalid handoff filename: {filename}")));
    };
    let valid = stem.as_bytes().first().is_some_and(u8::is_ascii_uppercase) &&
        stem.split('_').all(|part| {
            !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
        });
    if !valid {
        return Err(Error::usage(format!("invalid handoff filename: {filename}")));
    }
    Ok(())
}

fn validate_task(task: &str) -> Result<()> {
    if task.trim().is_empty() {
        return Err(Error::usage("task must not be empty"));
    }
    if task.contains(['\r', '\n']) {
        return Err(Error::usage("task must be a single line"));
    }
    Ok(())
}

fn validate_before_work_skill(candidate: Option<&Path>) -> Result<Option<PathBuf>> {
    let Some(candidate) = candidate else {
        return Ok(None);
    };
    if !candidate.is_absolute() {
        return Err(Error::usage(format!("before-work skill directory must be absolute: {}", candidate.display())));
    }
    let directory = candidate.canonicalize().map_err(|error| {
        Error::operational(format!("cannot resolve before-work skill directory {}: {error}", candidate.display()))
    })?;
    let metadata = fs::metadata(&directory).map_err(|error| {
        Error::operational(format!("cannot inspect before-work skill directory {}: {error}", directory.display()))
    })?;
    if !metadata.is_dir() {
        return Err(Error::usage(format!("before-work skill is not a directory: {}", directory.display())));
    }
    let entrypoint = directory.join("SKILL.md");
    let entrypoint_metadata = fs::metadata(&entrypoint).map_err(|error| {
        Error::operational(format!("before-work skill entrypoint is not readable {}: {error}", entrypoint.display()))
    })?;
    if !entrypoint_metadata.is_file() {
        return Err(Error::usage(format!("before-work skill entrypoint is not a file: {}", entrypoint.display())));
    }
    File::open(&entrypoint).map_err(|error| {
        Error::operational(format!("before-work skill entrypoint is not readable {}: {error}", entrypoint.display()))
    })?;
    Ok(Some(directory))
}

fn validate_draft(path: &Path, cross_repository: bool) -> Result<String> {
    let contents = fs::read_to_string(path)
        .map_err(|error| Error::operational(format!("cannot read draft {}: {error}", path.display())))?;
    if contents.trim().is_empty() {
        return Err(Error::usage(format!("handoff draft is empty: {}", path.display())));
    }
    let first = contents.lines().next().unwrap_or_default();
    if first == "---" {
        return Err(Error::usage("handoff draft must not start with YAML frontmatter"));
    }
    let valid_h1 = first
        .strip_prefix("# ")
        .is_some_and(|rest| rest.chars().next().is_some_and(|character| !character.is_whitespace()));
    if !valid_h1 {
        return Err(Error::usage("handoff draft must start with an H1 heading"));
    }
    let lines = contents.lines().collect::<Vec<_>>();
    for reserved in ["## Handoff category", "## Execution status", "## Handoff cleanup"] {
        if lines.contains(&reserved) {
            return Err(Error::usage(format!("handoff draft contains reserved heading: {reserved}")));
        }
    }
    if cross_repository && !lines.contains(&"## Repository order") {
        return Err(Error::usage("cross-repository draft is missing a Repository order section"));
    }
    Ok(contents)
}

fn compose(
    category: &str,
    launch_repository: &str,
    repositories: &[&str],
    target: &str,
    task: &str,
    draft: &str,
    home: &str,
) -> Vec<u8> {
    let mut output = String::new();
    output.push_str("---\n");
    output.push_str(&format!("category: {}\n", yaml_quote(category)));
    output.push_str(&format!("created: {}\n", yaml_quote(&Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string())));
    output.push_str(&format!("launch_repo: {}\n", yaml_quote(launch_repository)));
    output.push_str("repos:\n");
    for repository in repositories {
        output.push_str(&format!("  - {}\n", yaml_quote(repository)));
    }
    output.push_str(&format!("origin: {}\n", yaml_quote(target)));
    output.push_str(&format!("task: {}\n", yaml_quote(task)));
    output.push_str("---\n");
    output.push_str(draft.trim_end_matches(['\r', '\n']));
    output.push_str("\n\n");
    output.push_str(&footer(category, target, home));
    abbreviate_home_paths(output, home).into_bytes()
}

fn abbreviate_home_paths(contents: String, home: &str) -> String {
    if home == "/" { contents } else { contents.replace(home, "~") }
}

fn yaml_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn footer(category: &str, target: &str, home: &str) -> String {
    format!(
        "## Handoff category\n\n\
Category: `{category}`\n\n\
This handoff is categorized above. Complete the requested task according to its stated outcome, boundaries, authority\n\
constraints, and validation requirements.\n\n\
## Execution status\n\n\
Current status: No task attempt has been recorded.\n\n\
If work stops before successful completion, replace the current status—not append an attempt history—with a concise\n\
record of completed work, remaining work, validation commands and outcomes, the blocker, and the next concrete\n\
action.\n\n\
## Handoff cleanup\n\n\
Archive this handoff only after the requested work is complete and task-scoped validation passes:\n\n\
```sh\n\
ai-handoff archive {}\n\
```\n\n\
A broader required check may remain non-green only when evidence attributes every failure to pre-existing or unrelated\n\
work outside this task's scope. Record each non-green command, its outcome, and that attribution in the final report,\n\
then verify the original path no longer exists. Keep this handoff when work remains, task-scoped validation fails or is\n\
skipped, or any broader failure may have been caused by this task. Archive only this handoff, never\n\
`.ai/task-handoffs/` or any other handoff.\n",
        shell_path(target, home)
    )
}

fn shell_path(path: &str, home: &str) -> String {
    if home != "/" {
        if path == home {
            return "~".to_owned();
        }
        if let Some(relative) = path.strip_prefix(home).and_then(|suffix| suffix.strip_prefix('/')) {
            return format!("~/{}", shell_quote(relative));
        }
    }
    shell_quote(path)
}

fn build_command(
    category: &str,
    task: &str,
    launch_repository: &str,
    target: &str,
    relative: &Path,
    single_repository: bool,
    before_work_skill: Option<&str>,
) -> String {
    let (location, instructions) = if single_repository {
        (
            format!("under {}", relative.display()),
            "Follow its stated outcome, boundaries, authority constraints, and validation requirements.",
        )
    } else {
        (
            format!("at {target}"),
            "Start in the selected first repository and follow its stated repository order, outcome, boundaries, authority constraints, and validation requirements.",
        )
    };
    let mut prompt = format!(
        "A previous agent prepared a {category} task handoff for {task} {location}. Read the handoff, then complete its requested {category} task. {instructions}"
    );
    if let Some(skill) = before_work_skill {
        prompt.push_str(&format!(" Before any task work, load and follow the skill defined at {skill}/SKILL.md."));
    }
    format!("codex -C {} {}", shell_quote(launch_repository), shell_quote(&prompt))
}

fn validate_physical_parents(base: &Path) -> Result<Option<OwnedFd>> {
    let base_directory = open_base_directory(base)?;
    let ai_path = base.join(".ai");
    let Some(ai_directory) = open_existing_directory(&base_directory, OsStr::new(".ai"), &ai_path)? else {
        return Ok(None);
    };
    let handoff_path = ai_path.join("task-handoffs");
    open_existing_directory(&ai_directory, OsStr::new("task-handoffs"), &handoff_path)
}

fn open_base_directory(path: &Path) -> Result<OwnedFd> {
    let directory =
        unix_fs::open(path, OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC, Mode::empty())
            .map_err(|error| {
                Error::operational(format!("cannot inspect handoff parent {}: {error}", path.display()))
            })?;
    verify_directory(&directory, path, "handoff parent")?;
    Ok(directory)
}

fn open_existing_directory(parent: &OwnedFd, name: &OsStr, path: &Path) -> Result<Option<OwnedFd>> {
    match unix_fs::openat(
        parent,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(directory) => {
            verify_directory(&directory, path, "handoff parent")?;
            Ok(Some(directory))
        }
        Err(Errno::NOENT) => Ok(None),
        Err(error) if error == Errno::LOOP || error == Errno::NOTDIR => Err(physical_directory_error(path)),
        Err(error) => Err(Error::operational(format!("cannot inspect handoff parent {}: {error}", path.display()))),
    }
}

fn open_handoff_directory(parent: &OwnedFd, name: &OsStr, path: &Path) -> Result<OwnedFd> {
    let directory = unix_fs::openat(
        parent,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| {
        if error == Errno::LOOP || error == Errno::NOTDIR {
            physical_directory_error(path)
        } else {
            Error::operational(format!("cannot inspect handoff directory {}: {error}", path.display()))
        }
    })?;
    verify_directory(&directory, path, "handoff directory")?;
    Ok(directory)
}

fn verify_directory(directory: &OwnedFd, path: &Path, description: &str) -> Result<()> {
    let status = unix_fs::fstat(directory)
        .map_err(|error| Error::operational(format!("cannot inspect {description} {}: {error}", path.display())))?;
    if FileType::from_raw_mode(status.st_mode) != FileType::Directory {
        return Err(physical_directory_error(path));
    }
    Ok(())
}

fn physical_directory_error(path: &Path) -> Error {
    Error::operational(format!("handoff parent must be a physical directory: {}", path.display()))
}

fn ensure_absent(directory: &OwnedFd, name: &OsStr, path: &Path) -> Result<()> {
    match unix_fs::statat(directory, name, AtFlags::SYMLINK_NOFOLLOW) {
        Err(Errno::NOENT) => Ok(()),
        Err(error) => Err(Error::operational(format!("cannot inspect handoff target {}: {error}", path.display()))),
        Ok(_) => Err(Error::operational(format!("handoff target already exists: {}", path.display()))),
    }
}

struct Publication {
    base_directory: OwnedFd,
    ai_directory: Option<OwnedFd>,
    handoff_directory: Option<OwnedFd>,
    target_name: OsString,
    temporary_name: Option<OsString>,
    target_created: bool,
    ai_created: bool,
    handoff_directory_created: bool,
    finished: bool,
}

impl Publication {
    fn finish(&mut self) {
        self.finished = true;
    }

    fn ai_directory(&self) -> &OwnedFd {
        self.ai_directory.as_ref().expect(".ai directory is open")
    }

    fn handoff_directory(&self) -> &OwnedFd {
        self.handoff_directory.as_ref().expect("handoff directory is open")
    }
}

impl Drop for Publication {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        if let Some(directory) = &self.handoff_directory {
            if let Some(name) = &self.temporary_name {
                let _ = unix_fs::unlinkat(directory, name, AtFlags::empty());
            }
            if self.target_created {
                let _ = unix_fs::unlinkat(directory, &self.target_name, AtFlags::empty());
            }
        }
        if self.handoff_directory_created &&
            let Some(directory) = &self.ai_directory
        {
            let _ = unix_fs::unlinkat(directory, "task-handoffs", AtFlags::REMOVEDIR);
        }
        if self.ai_created {
            let _ = unix_fs::unlinkat(&self.base_directory, ".ai", AtFlags::REMOVEDIR);
        }
    }
}

fn publish(base: &Path, target: &Path, contents: &[u8]) -> Result<Publication> {
    let target_name = target.file_name().expect("target has a filename").to_owned();
    let mut publication = Publication {
        base_directory: open_base_directory(base)?,
        ai_directory: None,
        handoff_directory: None,
        target_name,
        temporary_name: None,
        target_created: false,
        ai_created: false,
        handoff_directory_created: false,
        finished: false,
    };

    let ai_path = base.join(".ai");
    publication.ai_created = create_directory(&publication.base_directory, ".ai", &ai_path)?;
    publication.ai_directory = Some(open_handoff_directory(&publication.base_directory, OsStr::new(".ai"), &ai_path)?);

    let handoff_path = ai_path.join("task-handoffs");
    publication.handoff_directory_created =
        create_directory(publication.ai_directory(), "task-handoffs", &handoff_path)?;
    publication.handoff_directory =
        Some(open_handoff_directory(publication.ai_directory(), OsStr::new("task-handoffs"), &handoff_path)?);

    let (temporary_name, mut temporary) = create_staged_file(publication.handoff_directory())?;
    publication.temporary_name = Some(temporary_name);
    temporary
        .write_all(contents)
        .and_then(|()| temporary.flush())
        .and_then(|()| temporary.sync_all())
        .map_err(|error| Error::operational(format!("cannot write staged handoff: {error}")))?;
    ensure_absent(publication.handoff_directory(), &publication.target_name, target)?;
    unix_fs::linkat(
        publication.handoff_directory(),
        publication.temporary_name.as_ref().expect("staged handoff has a name"),
        publication.handoff_directory(),
        &publication.target_name,
        AtFlags::empty(),
    )
    .map_err(|error| Error::operational(format!("cannot publish handoff without overwriting: {error}")))?;
    publication.target_created = true;

    let published_file = unix_fs::openat(
        publication.handoff_directory(),
        &publication.target_name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| Error::operational(format!("cannot verify published handoff: {error}")))?;
    let mut published = Vec::new();
    File::from(published_file)
        .read_to_end(&mut published)
        .map_err(|error| Error::operational(format!("cannot verify published handoff: {error}")))?;
    if published != contents {
        return Err(Error::operational("published handoff bytes changed during validation"));
    }
    let temporary_name = publication.temporary_name.as_ref().expect("staged handoff has a name").clone();
    unix_fs::unlinkat(publication.handoff_directory(), temporary_name, AtFlags::empty())
        .map_err(|error| Error::operational(format!("cannot remove staged handoff: {error}")))?;
    publication.temporary_name = None;
    Ok(publication)
}

fn create_directory(parent: &OwnedFd, name: &str, path: &Path) -> Result<bool> {
    match unix_fs::mkdirat(parent, name, Mode::RWXU | Mode::RGRP | Mode::XGRP | Mode::ROTH | Mode::XOTH) {
        Ok(()) => Ok(true),
        Err(Errno::EXIST) => Ok(false),
        Err(error) => Err(Error::operational(format!("cannot create handoff directory {}: {error}", path.display()))),
    }
}

fn create_staged_file(directory: &OwnedFd) -> Result<(OsString, File)> {
    static TEMPORARY_COUNTER: AtomicU64 = AtomicU64::new(0);

    for _ in 0..128 {
        let unique = TEMPORARY_COUNTER.fetch_add(1, Ordering::Relaxed);
        let suffix = RandomState::new().hash_one((std::process::id(), unique));
        let name = OsString::from(format!(".ai-handoff.{suffix:016x}"));
        match unix_fs::openat(
            directory,
            &name,
            OFlags::CREATE | OFlags::EXCL | OFlags::WRONLY | OFlags::CLOEXEC | OFlags::NOFOLLOW,
            Mode::RUSR | Mode::WUSR | Mode::RGRP | Mode::ROTH,
        ) {
            Ok(file) => return Ok((name, File::from(file))),
            Err(Errno::EXIST) => {}
            Err(error) => return Err(Error::operational(format!("cannot stage handoff: {error}"))),
        }
    }
    Err(Error::operational("cannot stage handoff: too many temporary filename collisions"))
}

fn copy_and_verify(command: &str) -> Result<()> {
    let mut child =
        Command::new("pbcopy")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| Error::operational(format!("required executable pbcopy is unavailable: {error}")))?;
    child
        .stdin
        .take()
        .expect("piped clipboard stdin")
        .write_all(command.as_bytes())
        .map_err(|error| Error::operational(format!("clipboard copy failed: {error}")))?;
    let copied =
        child.wait_with_output().map_err(|error| Error::operational(format!("clipboard copy failed: {error}")))?;
    if !copied.status.success() {
        return Err(Error::operational("clipboard copy failed"));
    }

    let pasted = Command::new("pbpaste")
        .output()
        .map_err(|error| Error::operational(format!("required executable pbpaste is unavailable: {error}")))?;
    if !pasted.status.success() {
        return Err(Error::operational("clipboard readback failed"));
    }
    if pasted.stdout != command.as_bytes() {
        return Err(Error::operational("clipboard verification failed"));
    }
    Ok(())
}

fn utf8_path<'a>(path: &'a Path, label: &str) -> Result<&'a str> {
    path.to_str().ok_or_else(|| Error::operational(format!("{label} is not valid UTF-8: {}", path.display())))
}
