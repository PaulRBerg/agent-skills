use std::{
    collections::{BTreeSet, HashMap, HashSet},
    env, fs,
    io::Read,
    path::{Component, Path},
    process::{Command, Stdio},
    time::Duration,
};

use rand::random;
use tempfile::Builder;
use wait_timeout::ChildExt;

use crate::{
    cli::{DiffMode, PrepareArgs},
    config,
    error::{AppError, Result},
    git::{
        Repository, clear_repository_environment, copy_file, decode_nul_paths, git_error, literal_pathspec,
        validate_safe_path_text,
    },
    rules,
    state::{MessageFormat, Store, Transaction, TransactionStatus, now_seconds},
};

#[derive(Clone, Debug)]
struct Baseline {
    path: String,
    oid: String,
    automatic: bool,
}

pub(crate) const PATH_BATCH_SIZE: usize = 32;
const AI_COORD_BASELINE_TIMEOUT: Duration = Duration::from_secs(5);
const AI_COORD_TRAILER_TIMEOUT: Duration = Duration::from_millis(750);

pub fn run(args: PrepareArgs, store: &Store) -> Result<()> {
    Repository::ensure_default_index_env()?;
    validate_mode(&args)?;
    let repository = Repository::discover()?;
    repository.ensure_idle()?;
    let branch = repository.branch()?;
    let head = repository.head_oid()?;
    let unborn = head.is_none();
    let base_head = match head {
        Some(head) => head,
        None => repository.empty_tree()?,
    };
    let intended_paths = normalize_inputs(&repository, &args.paths)?;
    let baselines = parse_baselines(&repository, &args, &intended_paths)?;
    let repository_config = config::load(&repository.root, message_format_override(&args))?;

    let temporary = Builder::new().prefix("prepare-").tempdir_in(store.temporary())?;
    let shared_index = repository.git_path("index")?;
    let shared_copy = temporary.path().join("shared-index");
    if shared_index.is_file() {
        copy_file(&shared_index, &shared_copy)?;
    } else {
        repository.checked(["read-tree", &base_head], Some(&shared_copy))?;
    }
    let shared_index_tree = repository.text(["write-tree"], Some(&shared_copy))?;

    let prepared_index = temporary.path().join("prepared-index");
    if args.staged {
        if shared_index.is_file() {
            copy_file(&shared_index, &prepared_index)?;
        } else {
            repository.checked(["read-tree", &base_head], Some(&prepared_index))?;
        }
    } else {
        repository.checked(["read-tree", &base_head], Some(&prepared_index))?;
        stage_worktree(&repository, &prepared_index, &base_head, args.all, &intended_paths)?;
    }
    let worktree_tree = repository.text(["write-tree"], Some(&prepared_index))?;
    apply_baselines(&repository, temporary.path(), &prepared_index, &base_head, &worktree_tree, &baselines)?;
    let prepared_tree = repository.text(["write-tree"], Some(&prepared_index))?;
    let paths = changed_paths(&repository, &base_head, &prepared_tree)?;
    if paths.is_empty() {
        return Err(AppError::usage("prepared transaction has no changes"));
    }

    let name_status = repository.text(
        [
            "-c",
            "core.quotePath=false",
            "diff",
            "--no-color",
            "--no-ext-diff",
            "--no-textconv",
            "--name-status",
            "--no-renames",
            &base_head,
            &prepared_tree,
            "--",
        ],
        None,
    )?;
    let shortstat = repository.text(
        ["diff", "--no-color", "--no-ext-diff", "--no-textconv", "--shortstat", &base_head, &prepared_tree, "--"],
        None,
    )?;
    let full_diff = if args.diff == DiffMode::Full {
        Some(repository.text_lossy(
            [
                "-c",
                "core.quotePath=false",
                "diff",
                "--no-color",
                "--no-ext-diff",
                "--no-textconv",
                &base_head,
                &prepared_tree,
                "--",
            ],
            None,
        )?)
    } else {
        None
    };
    let (full_diff, diff_truncations) = match full_diff {
        Some(diff) => {
            let (kept, truncations) = truncate_full_diff(&diff, DIFF_FILE_LINE_LIMIT);
            (Some(kept), truncations)
        }
        None => (None, Vec::new()),
    };

    let id = allocate_id(store)?;
    let trailer = bounded_trailer(&repository.root);
    let transaction = Transaction {
        id: id.clone(),
        repository_root: repository.root.clone(),
        branch,
        base_head,
        unborn,
        prepared_tree: prepared_tree.clone(),
        shared_index_tree,
        message_format: repository_config.message_format,
        validation_command: repository_config.validation_command,
        trailer,
        paths,
        name_status,
        shortstat,
        created_at: now_seconds(),
        status: TransactionStatus::Prepared,
        pending_commit: None,
        commit_oid: None,
        hook_added: Vec::new(),
        index_lock_token: None,
        reconciled: false,
        push_requested: false,
        terminal_at: None,
    };

    let transaction_ref = transaction.reference();
    let base_ref = transaction.base_reference();
    let index_ref = transaction.index_reference();
    repository.create_refs(&[
        (&transaction_ref, &prepared_tree),
        (&base_ref, &transaction.base_head),
        (&index_ref, &transaction.shared_index_tree),
    ])?;
    if let Err(error) = store.save(&transaction) {
        let _ = repository.delete_refs(&transaction.references());
        return Err(error);
    }
    print_prepared(&transaction, args.porcelain, full_diff.as_deref(), &diff_truncations, &baselines);
    Ok(())
}

fn validate_mode(args: &PrepareArgs) -> Result<()> {
    if !args.all && !args.staged && args.paths.is_empty() {
        return Err(AppError::usage("prepare requires explicit paths unless --all or --staged is used"));
    }
    if (args.all || args.staged) && !args.paths.is_empty() {
        return Err(AppError::usage("--all and --staged do not accept explicit paths"));
    }
    if args.staged && !args.exclude_baselines.is_empty() {
        return Err(AppError::usage("--exclude-baseline is incompatible with --staged"));
    }
    Ok(())
}

fn message_format_override(args: &PrepareArgs) -> Option<MessageFormat> {
    if args.natural {
        Some(MessageFormat::Natural)
    } else if args.conventional {
        Some(MessageFormat::Conventional)
    } else {
        None
    }
}

fn normalize_inputs(repository: &Repository, inputs: &[String]) -> Result<Vec<String>> {
    let cwd = env::current_dir()?.canonicalize()?;
    let cwd_relative = cwd
        .strip_prefix(&repository.root)
        .map_err(|_| AppError::usage("current directory resolves outside the repository"))?;
    let mut normalized = Vec::with_capacity(inputs.len());
    let mut seen = HashSet::new();
    for input in inputs {
        validate_safe_path_text(input)?;
        let input_path = Path::new(input);
        let relative = if input_path.is_absolute() {
            input_path
                .strip_prefix(&repository.root)
                .map_err(|_| AppError::usage(format!("path is outside the repository: {input}")))?
                .to_path_buf()
        } else {
            cwd_relative.join(input_path)
        };
        let path = normalize_relative(&relative, input)?;
        if seen.insert(path.clone()) {
            normalized.push(path);
        }
    }
    Ok(normalized)
}

fn normalize_relative(path: &Path, original: &str) -> Result<String> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => {
                let value =
                    value.to_str().ok_or_else(|| AppError::usage("non-UTF-8 repository paths are not supported"))?;
                if value == ".git" {
                    return Err(AppError::usage(format!("Git metadata paths are not allowed: {original}")));
                }
                if value.chars().any(char::is_control) {
                    return Err(AppError::usage("repository paths may not contain control characters"));
                }
                parts.push(value);
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if parts.pop().is_none() {
                    return Err(AppError::usage(format!("path is outside the repository: {original}")));
                }
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(AppError::usage(format!("invalid repository path: {original}")));
            }
        }
    }
    if parts.is_empty() {
        return Err(AppError::usage(format!("invalid repository path: {original}")));
    }
    Ok(parts.join("/"))
}

fn stage_worktree(repository: &Repository, index: &Path, base_head: &str, all: bool, paths: &[String]) -> Result<()> {
    let pathspecs = if all { Vec::new() } else { paths.iter().map(|path| literal_pathspec(path)).collect::<Vec<_>>() };
    let mut head_paths = BTreeSet::new();
    for pathspecs in pathspec_batches(&pathspecs) {
        let mut arguments = vec![
            "ls-tree".to_owned(),
            "-r".to_owned(),
            "-z".to_owned(),
            "--name-only".to_owned(),
            base_head.to_owned(),
            "--".to_owned(),
        ];
        arguments.extend(pathspecs.iter().cloned());
        let bytes = repository.bytes(arguments, None)?;
        head_paths.extend(decode_nul_paths(&bytes)?);
    }
    let mut worktree_candidates = BTreeSet::new();
    let mut skip_worktree = BTreeSet::new();
    for pathspecs in pathspec_batches(&pathspecs) {
        let mut candidates_arguments = vec![
            "-c".to_owned(),
            "core.ignorecase=false".to_owned(),
            "ls-files".to_owned(),
            "-z".to_owned(),
            "--cached".to_owned(),
            "--others".to_owned(),
            "--exclude-standard".to_owned(),
            "--".to_owned(),
        ];
        candidates_arguments.extend(pathspecs.iter().cloned());
        let bytes = repository.bytes(candidates_arguments, None)?;
        worktree_candidates.extend(decode_nul_paths(&bytes)?);

        let mut skip_arguments = vec![
            "-c".to_owned(),
            "core.ignorecase=false".to_owned(),
            "ls-files".to_owned(),
            "-t".to_owned(),
            "-z".to_owned(),
            "--".to_owned(),
        ];
        skip_arguments.extend(pathspecs.iter().cloned());
        let bytes = repository.bytes(skip_arguments, None)?;
        skip_worktree.extend(decode_skip_worktree_paths(&bytes)?);
    }
    let mut directory_cache = HashMap::new();
    let mut worktree_paths = Vec::new();
    for path in worktree_candidates {
        if exact_path_exists(&repository.root, Path::new(&path), &mut directory_cache)? {
            worktree_paths.push(path);
        }
    }
    let existing = worktree_paths.iter().cloned().collect::<HashSet<_>>();
    for path in skip_worktree {
        if !existing.contains(&path) {
            head_paths.remove(&path);
        }
    }
    if head_paths.is_empty() && worktree_paths.is_empty() {
        return Err(AppError::usage("intended paths do not match tracked or worktree files"));
    }

    let head_paths = head_paths.into_iter().collect::<Vec<_>>();
    for paths in head_paths.chunks(PATH_BATCH_SIZE) {
        let mut arguments = vec!["update-index".to_owned(), "--force-remove".to_owned(), "--".to_owned()];
        arguments.extend(paths.iter().cloned());
        repository.checked(arguments, Some(index))?;
    }
    for paths in worktree_paths.chunks(PATH_BATCH_SIZE) {
        let mut arguments = vec!["add".to_owned(), "--force".to_owned(), "--".to_owned()];
        arguments.extend(paths.iter().map(|path| literal_pathspec(path)));
        let output = repository.raw(arguments, Some(index))?;
        if !output.status.success() {
            let message = git_error(output).message;
            return Err(AppError::usage(format!("cannot snapshot intended paths: {message}")));
        }
    }
    Ok(())
}

fn pathspec_batches(pathspecs: &[String]) -> Vec<&[String]> {
    if pathspecs.is_empty() { vec![pathspecs] } else { pathspecs.chunks(PATH_BATCH_SIZE).collect() }
}

fn decode_skip_worktree_paths(bytes: &[u8]) -> Result<Vec<String>> {
    bytes
        .split(|byte| *byte == 0)
        .filter(|record| record.starts_with(b"S "))
        .map(|record| {
            let path = std::str::from_utf8(&record[2..])
                .map_err(|_| AppError::usage("non-UTF-8 repository paths are not supported"))?
                .to_owned();
            validate_safe_path_text(&path)?;
            Ok(path)
        })
        .collect()
}

fn exact_path_exists(
    root: &Path,
    path: &Path,
    directory_cache: &mut HashMap<std::path::PathBuf, HashSet<std::ffi::OsString>>,
) -> Result<bool> {
    let mut current = root.to_path_buf();
    for component in path.components() {
        let Component::Normal(expected) = component else {
            return Ok(false);
        };
        let entries = if let Some(entries) = directory_cache.get(&current) {
            entries
        } else {
            let directory = match fs::read_dir(&current) {
                Ok(directory) => directory,
                Err(error)
                    if matches!(error.kind(), std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory) =>
                {
                    return Ok(false);
                }
                Err(error) => return Err(error.into()),
            };
            let entries = directory
                .map(|entry| entry.map(|entry| entry.file_name()))
                .collect::<std::result::Result<HashSet<_>, _>>()?;
            directory_cache.insert(current.clone(), entries);
            directory_cache.get(&current).expect("inserted directory cache")
        };
        if !entries.contains(expected) {
            return Ok(false);
        }
        current.push(expected);
    }
    Ok(true)
}

fn parse_baselines(repository: &Repository, args: &PrepareArgs, intended_paths: &[String]) -> Result<Vec<Baseline>> {
    let mut baselines = Vec::new();
    let mut seen = HashSet::new();
    for specification in &args.exclude_baselines {
        let (raw_path, oid) =
            specification.rsplit_once('=').ok_or_else(|| AppError::usage("--exclude-baseline requires PATH=OID"))?;
        if raw_path.is_empty() || oid.is_empty() {
            return Err(AppError::usage("--exclude-baseline requires non-empty PATH=OID"));
        }
        let path = normalize_inputs(repository, &[raw_path.to_owned()])?.remove(0);
        if !args.all && !intended_paths.contains(&path) {
            return Err(AppError::usage(format!("baseline path is not among intended paths: {path}")));
        }
        if !seen.insert(path.clone()) {
            return Err(AppError::usage(format!("duplicate baseline path: {path}")));
        }
        if !matches!(oid.len(), 40 | 64) || !oid.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(AppError::usage(format!("invalid baseline blob OID for path {path}: {oid}")));
        }
        let output = repository.raw(["cat-file", "-t", oid], None)?;
        if !output.status.success() {
            return Err(AppError::usage(format!("invalid baseline blob OID for path {path}: {oid}")));
        }
        let kind = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        if kind != "blob" {
            return Err(AppError::usage(format!("baseline OID is not a blob for path {path}: {oid}")));
        }
        let oid = repository.text(["rev-parse", "--verify", &format!("{oid}^{{blob}}")], None)?;
        baselines.push(Baseline { path, oid, automatic: false });
    }
    if !args.staged && !args.no_auto_baseline {
        for (raw_path, raw_oid) in bounded_baselines(&repository.root) {
            if Path::new(&raw_path).is_absolute() {
                continue;
            }
            let Ok(path) = normalize_relative(Path::new(&raw_path), &raw_path) else {
                continue;
            };
            if seen.contains(&path) || (!args.all && !intended_paths.contains(&path)) {
                continue;
            }
            if !matches!(raw_oid.len(), 40 | 64) || !raw_oid.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                continue;
            }
            let output = match repository.raw(["cat-file", "-t", &raw_oid], None) {
                Ok(output) if output.status.success() => output,
                _ => continue,
            };
            if String::from_utf8_lossy(&output.stdout).trim() != "blob" {
                continue;
            }
            let Ok(oid) = repository.text(["rev-parse", "--verify", &format!("{raw_oid}^{{blob}}")], None) else {
                continue;
            };
            seen.insert(path.clone());
            baselines.push(Baseline { path, oid, automatic: true });
        }
    }
    Ok(baselines)
}

fn bounded_baselines(repository_root: &Path) -> Vec<(String, String)> {
    let Some(bytes) = bounded_ai_coord(repository_root, "baseline", 64 * 1024, AI_COORD_BASELINE_TIMEOUT) else {
        return Vec::new();
    };
    let Ok(text) = String::from_utf8(bytes) else {
        return Vec::new();
    };
    text.lines()
        .filter_map(|line| {
            let (path, oid) = line.split_once('\t')?;
            if path.is_empty() || oid.is_empty() || oid.contains('\t') {
                return None;
            }
            Some((path.to_owned(), oid.to_owned()))
        })
        .collect()
}

fn apply_baselines(
    repository: &Repository,
    temporary: &Path,
    prepared_index: &Path,
    base_head: &str,
    worktree_tree: &str,
    baselines: &[Baseline],
) -> Result<()> {
    for (number, baseline) in baselines.iter().enumerate() {
        let baseline_index = temporary.join(format!("baseline-index-{number}"));
        repository.checked(["read-tree", worktree_tree], Some(&baseline_index))?;
        let baseline_entry = repository
            .tree_entry(base_head, &baseline.path)?
            .or(repository.tree_entry(worktree_tree, &baseline.path)?)
            .ok_or_else(|| {
                AppError::usage(format!("baseline path is not a file in HEAD or the worktree: {}", baseline.path))
            })?;
        if baseline_entry.kind != "blob" {
            return Err(AppError::usage(format!("baseline path is not a file: {}", baseline.path)));
        }
        let mode = baseline_entry.mode;
        repository.checked(
            ["update-index", "--add", "--cacheinfo", &mode, &baseline.oid, &baseline.path],
            Some(&baseline_index),
        )?;
        let baseline_tree = repository.text(["write-tree"], Some(&baseline_index))?;

        repository.checked(["update-index", "--force-remove", "--", &baseline.path], Some(prepared_index))?;
        if let Some(entry) = repository.tree_entry(base_head, &baseline.path)? {
            repository.checked(
                ["update-index", "--add", "--cacheinfo", &entry.mode, &entry.oid, &baseline.path],
                Some(prepared_index),
            )?;
        }
        let pathspec = literal_pathspec(&baseline.path);
        let patch = repository.bytes(
            ["diff", "--binary", "--no-ext-diff", "--no-textconv", &baseline_tree, worktree_tree, "--", &pathspec],
            None,
        )?;
        if patch.is_empty() {
            continue;
        }
        let check = repository.with_input(
            ["apply", "--cached", "--check", "--whitespace=nowarn", "-"],
            &patch,
            Some(prepared_index),
        )?;
        if !check.status.success() {
            return Err(AppError::usage(format!(
                "baseline changes do not apply cleanly to prepared HEAD for path: {}",
                baseline.path
            )));
        }
        let applied =
            repository.with_input(["apply", "--cached", "--whitespace=nowarn", "-"], &patch, Some(prepared_index))?;
        if !applied.status.success() {
            return Err(git_error(applied));
        }
    }
    Ok(())
}

fn changed_paths(repository: &Repository, base_head: &str, prepared_tree: &str) -> Result<Vec<String>> {
    let bytes = repository.bytes(
        ["diff", "--no-ext-diff", "--no-textconv", "--name-only", "--no-renames", "-z", base_head, prepared_tree, "--"],
        None,
    )?;
    decode_nul_paths(&bytes)
}

fn allocate_id(store: &Store) -> Result<String> {
    for _ in 0..128 {
        let id = format!("{:016x}", random::<u64>());
        if !store.transaction_path(&id)?.exists() {
            return Ok(id);
        }
    }
    Err(AppError::operational("cannot allocate a unique transaction ID"))
}

fn bounded_trailer(repository_root: &Path) -> Option<String> {
    let bytes = bounded_ai_coord(repository_root, "trailer", 512, AI_COORD_TRAILER_TIMEOUT)?;
    let text = String::from_utf8(bytes).ok()?;
    let line = text.trim_end_matches(['\r', '\n']);
    if line.is_empty() ||
        line.len() > 256 ||
        line.chars().any(char::is_control) ||
        !line.starts_with("Agent-Session: ") ||
        line.len() == "Agent-Session: ".len() ||
        line.trim() != line
    {
        return None;
    }
    Some(line.to_owned())
}

fn bounded_ai_coord(repository_root: &Path, subcommand: &str, limit: u64, timeout: Duration) -> Option<Vec<u8>> {
    let timeout = env::var("AI_COMMIT_TEST_COORD_TIMEOUT_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|milliseconds| *milliseconds > 0)
        .map(Duration::from_millis)
        .unwrap_or(timeout);
    let mut command = Command::new("ai-coord");
    clear_repository_environment(&mut command);
    let mut child = command
        .arg(subcommand)
        .current_dir(repository_root)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.take(limit + 1).read_to_end(&mut bytes).ok()?;
        Some(bytes)
    });
    let status = match child.wait_timeout(timeout).ok()? {
        Some(status) => status,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
    };
    let bytes = reader.join().ok()??;
    if !status.success() {
        return None;
    }
    if bytes.len() as u64 > limit {
        return None;
    }
    Some(bytes)
}

const DIFF_FILE_LINE_LIMIT: usize = 400;

// Display-only cap: over-limit per-file sections are cut with a disclosure record;
// the prepared tree, name-status, shortstat, and paths stay complete.
fn truncate_full_diff(diff: &str, limit: usize) -> (String, Vec<(String, usize)>) {
    let mut output = String::with_capacity(diff.len());
    let mut truncations: Vec<(String, usize)> = Vec::new();
    let mut path = String::new();
    let mut kept = 0usize;
    let mut omitted = 0usize;
    // `+++ b/<path>` headers are only trustworthy before a file's first hunk: an added line whose
    // content itself begins with `++ b/` renders as a `+++ b/...` line inside a hunk and must not
    // be mistaken for the next file-header path update.
    let mut in_header = false;
    for line in diff.lines() {
        if line.starts_with("diff --git ") {
            if omitted > 0 {
                truncations.push((path.clone(), omitted));
            }
            path = diff_git_header_path(line).unwrap_or_default();
            kept = 0;
            omitted = 0;
            in_header = true;
        } else if in_header {
            if line.starts_with("@@") {
                in_header = false;
            } else if let Some(new_path) = header_plus_path(line) {
                path = new_path;
            }
        }
        if kept < limit {
            output.push_str(line);
            output.push('\n');
            kept += 1;
        } else {
            omitted += 1;
        }
    }
    if omitted > 0 {
        truncations.push((path, omitted));
    }
    if output.ends_with('\n') {
        output.pop();
    }
    (output, truncations)
}

/// Extracts the `a/`-side path from a `diff --git a/<path> b/<path>` header line, unquoting it if
/// Git quoted it (paths containing `"`, `\`, or control characters are quoted even with
/// `core.quotePath=false`, which only exempts non-ASCII bytes).
fn diff_git_header_path(line: &str) -> Option<String> {
    let rest = line.strip_prefix("diff --git ")?;
    if rest.starts_with('"') {
        let (path, _remainder) = take_quoted_git_path(rest)?;
        return path.strip_prefix("a/").map(str::to_owned);
    }
    let after_a = rest.strip_prefix("a/")?;
    Some(after_a.split(" b/").next().unwrap_or_default().to_owned())
}

/// Extracts the `b/`-side path from a `+++ b/<path>` (or quoted) header line, or `None` for
/// `+++ /dev/null`.
fn header_plus_path(line: &str) -> Option<String> {
    let rest = line.strip_prefix("+++ ")?;
    if rest.starts_with('"') {
        let (path, _remainder) = take_quoted_git_path(rest)?;
        return path.strip_prefix("b/").map(str::to_owned);
    }
    rest.strip_prefix("b/").map(str::to_owned)
}

/// Splits off one Git-quoted path token (the leading `"` is expected to have been checked by the
/// caller and is consumed here) and returns its unescaped value along with the remainder of the
/// input after the closing quote.
fn take_quoted_git_path(input: &str) -> Option<(String, &str)> {
    let rest = input.strip_prefix('"')?;
    let bytes = rest.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'"' => {
                let raw = &rest[..index];
                let remainder = &rest[index + 1..];
                return unquote_git_path(raw).map(|path| (path, remainder));
            }
            b'\\' => {
                index += 1;
                if index >= bytes.len() {
                    return None;
                }
                if bytes[index].is_ascii_digit() {
                    let mut consumed = 0;
                    while consumed < 3 && bytes.get(index).is_some_and(u8::is_ascii_digit) {
                        index += 1;
                        consumed += 1;
                    }
                } else {
                    index += 1;
                }
            }
            _ => index += 1,
        }
    }
    None
}

/// Decodes Git's C-style quoting (`quote_c_style`): `\"`, `\\`, the named control escapes, and
/// `\NNN` octal byte escapes.
fn unquote_git_path(quoted: &str) -> Option<String> {
    let bytes = quoted.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'\\' {
            out.push(bytes[index]);
            index += 1;
            continue;
        }
        index += 1;
        let escape = *bytes.get(index)?;
        if (b'0'..=b'7').contains(&escape) {
            let mut value = 0u32;
            let mut consumed = 0;
            while consumed < 3 {
                match bytes.get(index) {
                    Some(&digit) if (b'0'..=b'7').contains(&digit) => {
                        value = value * 8 + u32::from(digit - b'0');
                        index += 1;
                        consumed += 1;
                    }
                    _ => break,
                }
            }
            out.push(value as u8);
            continue;
        }
        let decoded = match escape {
            b'"' => b'"',
            b'\\' => b'\\',
            b'a' => 0x07,
            b'b' => 0x08,
            b'f' => 0x0c,
            b'n' => b'\n',
            b'r' => b'\r',
            b't' => b'\t',
            b'v' => 0x0b,
            _ => return None,
        };
        out.push(decoded);
        index += 1;
    }
    String::from_utf8(out).ok()
}

fn print_prepared(
    transaction: &Transaction,
    porcelain: bool,
    full_diff: Option<&str>,
    diff_truncations: &[(String, usize)],
    baselines: &[Baseline],
) {
    if porcelain {
        println!("PREPARED\t{}", transaction.id);
        println!("FORMAT\t{}", transaction.message_format.label());
        for line in rules::for_format(transaction.message_format).lines() {
            println!("RULE\t{}", escape_tsv(line));
        }
        if let Some(trailer) = &transaction.trailer {
            println!("TRAILER\t{}", escape_tsv(trailer));
        }
        for baseline in baselines.iter().filter(|baseline| baseline.automatic) {
            println!("AUTO_BASELINE\t{}\t{}", escape_tsv(&baseline.path), baseline.oid);
        }
        println!("BRANCH\t{}", escape_tsv(&transaction.branch));
        for line in transaction.name_status.lines() {
            println!("CHANGE\t{}", escape_tsv(line));
        }
        println!("SHORTSTAT\t{}", escape_tsv(&transaction.shortstat));
        if let Some(diff) = full_diff {
            for line in diff.lines() {
                println!("DIFF\t{}", escape_tsv(line));
            }
            for (path, omitted) in diff_truncations {
                println!("DIFF_TRUNCATED\t{}\t{omitted}", escape_tsv(path));
            }
        }
        for path in &transaction.paths {
            println!("PATH\t{}", escape_tsv(path));
        }
        return;
    }

    println!("PREPARED {}", transaction.id);
    println!("\n## message format\n{}", transaction.message_format.label());
    println!("\n## message format rules\n{}", rules::for_format(transaction.message_format).trim_end());
    if let Some(trailer) = &transaction.trailer {
        println!("\n## trailer\n{trailer}");
    }
    let automatic = baselines.iter().filter(|baseline| baseline.automatic).collect::<Vec<_>>();
    if !automatic.is_empty() {
        println!("\n## auto-applied baselines");
        for baseline in automatic {
            println!("{}={}", baseline.path, baseline.oid);
        }
    }
    println!("\n## branch\n{}", transaction.branch);
    println!("\n## name-status\n{}", transaction.name_status);
    println!("\n## shortstat\n{}", transaction.shortstat);
    if let Some(diff) = full_diff {
        println!("\n## diff\n{diff}");
        for (path, omitted) in diff_truncations {
            println!("DIFF_TRUNCATED {path} ({omitted} more lines)");
        }
    }
    println!("\n## commit paths");
    for path in &transaction.paths {
        println!("{path}");
    }
}

fn escape_tsv(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\t', "\\t").replace('\r', "\\r").replace('\n', "\\n")
}
