use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use chrono::Utc;
use tempfile::Builder;

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
    validate_physical_parents(&placement.base)?;
    ensure_absent(&placement.target)?;

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
        if !git::is_ignored(repository, &relative)? {
            return Err(Error::operational(format!("handoff target is not ignored by Git: {}", relative.display())));
        }
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

fn validate_physical_parents(base: &Path) -> Result<()> {
    for path in [base.join(".ai"), base.join(".ai/task-handoffs")] {
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Ok(_) => {
                return Err(Error::operational(format!(
                    "handoff parent must be a physical directory: {}",
                    path.display()
                )));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(Error::operational(format!("cannot inspect handoff parent {}: {error}", path.display())));
            }
        }
    }
    Ok(())
}

fn ensure_absent(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(Error::operational(format!("cannot inspect handoff target {}: {error}", path.display()))),
        Ok(_) => Err(Error::operational(format!("handoff target already exists: {}", path.display()))),
    }
}

struct Publication {
    target: PathBuf,
    target_created: bool,
    created_directories: Vec<PathBuf>,
    finished: bool,
}

impl Publication {
    fn finish(&mut self) {
        self.finished = true;
    }
}

impl Drop for Publication {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        if self.target_created {
            let _ = fs::remove_file(&self.target);
        }
        for directory in self.created_directories.iter().rev() {
            let _ = fs::remove_dir(directory);
        }
    }
}

fn publish(base: &Path, target: &Path, contents: &[u8]) -> Result<Publication> {
    let mut publication = Publication {
        target: target.to_path_buf(),
        target_created: false,
        created_directories: Vec::new(),
        finished: false,
    };
    for directory in [base.join(".ai"), base.join(".ai/task-handoffs")] {
        match fs::create_dir(&directory) {
            Ok(()) => publication.created_directories.push(directory),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                let metadata = fs::symlink_metadata(&directory).map_err(|inspect_error| {
                    Error::operational(format!(
                        "cannot inspect handoff directory {}: {inspect_error}",
                        directory.display()
                    ))
                })?;
                if !metadata.is_dir() || metadata.file_type().is_symlink() {
                    return Err(Error::operational(format!(
                        "handoff parent must be a physical directory: {}",
                        directory.display()
                    )));
                }
            }
            Err(error) => {
                return Err(Error::operational(format!(
                    "cannot create handoff directory {}: {error}",
                    directory.display()
                )));
            }
        }
    }

    let target_directory = target.parent().expect("target has a parent");
    let mut temporary = Builder::new()
        .prefix(".ai-handoff.")
        .tempfile_in(target_directory)
        .map_err(|error| Error::operational(format!("cannot stage handoff: {error}")))?;
    temporary
        .write_all(contents)
        .and_then(|()| temporary.flush())
        .and_then(|()| temporary.as_file().sync_all())
        .map_err(|error| Error::operational(format!("cannot write staged handoff: {error}")))?;
    ensure_absent(target)?;
    fs::hard_link(temporary.path(), target)
        .map_err(|error| Error::operational(format!("cannot publish handoff without overwriting: {error}")))?;
    publication.target_created = true;

    let mut published = Vec::new();
    File::open(target)
        .and_then(|mut file| file.read_to_end(&mut published))
        .map_err(|error| Error::operational(format!("cannot verify published handoff: {error}")))?;
    if published != contents {
        return Err(Error::operational("published handoff bytes changed during validation"));
    }
    temporary.close().map_err(|error| Error::operational(format!("cannot remove staged handoff: {error}")))?;
    Ok(publication)
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
