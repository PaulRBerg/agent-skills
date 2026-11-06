//! Bounded child-process runner shared by the Git helpers and provider inventory.
//!
//! One monotonic deadline covers stdin delivery, child completion, and the
//! stdout/stderr drain. Every pipe is served by an owned worker thread so the
//! supervising thread never blocks on a pipe. On Unix the child leads its own
//! process group, so cleanup also reaches descendants that inherited a pipe
//! and would otherwise keep a reader blocked after the child exited.

use std::{
    io::{self, Read, Write},
    process::{Child, Command, ExitStatus, Stdio},
    sync::mpsc::{self, Receiver, RecvTimeoutError, Sender},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use wait_timeout::ChildExt;

const MAX_COMMAND_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
/// Longest wait between checks for an early worker failure while the child still runs.
const WORKER_POLL: Duration = Duration::from_millis(20);
/// How long cleanup waits for IO workers after the process group is killed. A
/// descendant that escaped the group can hold a pipe open; its worker is then
/// detached instead of blocking the caller.
const WORKER_DRAIN_GRACE: Duration = Duration::from_secs(1);

#[derive(Debug)]
pub(crate) struct CommandOutput {
    pub(crate) status: ExitStatus,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
}

/// Run a child with bounded wall time while draining both output pipes.
pub(crate) fn run_output_timeout(command: &mut Command, timeout: Duration) -> io::Result<CommandOutput> {
    run_output_with_input_timeout(command, None, timeout)
}

pub(super) fn run_output_with_input_timeout(
    command: &mut Command,
    input: Option<&[u8]>,
    timeout: Duration,
) -> io::Result<CommandOutput> {
    let deadline = Deadline::start(timeout);
    if input.is_some() {
        command.stdin(Stdio::piped());
    }
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    configure_process_group(command);
    let mut child = command.spawn()?;
    let mut workers = Workers::new();
    let result = workers.spawn_io(&mut child, input).and_then(|()| supervise(&mut child, &mut workers, &deadline));
    if result.is_err() {
        terminate_child(&mut child);
    }
    workers.join(WORKER_DRAIN_GRACE);
    result
}

struct Deadline {
    started: Instant,
    timeout: Duration,
}

impl Deadline {
    fn start(timeout: Duration) -> Self {
        Self { started: Instant::now(), timeout }
    }

    fn remaining(&self) -> Duration {
        self.timeout.saturating_sub(self.started.elapsed())
    }

    fn error(&self) -> io::Error {
        io::Error::new(io::ErrorKind::TimedOut, format!("command timed out after {} seconds", self.timeout.as_secs()))
    }
}

enum Event {
    Stdin(io::Result<()>),
    Stdout(io::Result<Vec<u8>>),
    Stderr(io::Result<Vec<u8>>),
}

struct Workers {
    handles: Vec<JoinHandle<()>>,
    sender: Sender<Event>,
    events: Receiver<Event>,
    outstanding: usize,
}

impl Workers {
    fn new() -> Self {
        let (sender, events) = mpsc::channel();
        Self { handles: Vec::new(), sender, events, outstanding: 0 }
    }

    fn spawn_io(&mut self, child: &mut Child, input: Option<&[u8]>) -> io::Result<()> {
        let mut stdout = child.stdout.take().expect("piped stdout");
        let mut stderr = child.stderr.take().expect("piped stderr");
        self.spawn(move || Event::Stdout(read_bounded_output(&mut stdout, MAX_COMMAND_OUTPUT_BYTES)))?;
        self.spawn(move || Event::Stderr(read_bounded_output(&mut stderr, MAX_COMMAND_OUTPUT_BYTES)))?;
        if let Some(input) = input {
            let mut stdin = child.stdin.take().expect("piped stdin");
            let input = input.to_vec();
            self.spawn(move || {
                let result = stdin.write_all(&input);
                // Close the pipe explicitly so a child reading to EOF can finish.
                drop(stdin);
                Event::Stdin(result)
            })?;
        }
        Ok(())
    }

    fn spawn(&mut self, work: impl FnOnce() -> Event + Send + 'static) -> io::Result<()> {
        let sender = self.sender.clone();
        let handle = thread::Builder::new().name("ai-coord-output".to_owned()).spawn(move || {
            let _ = sender.send(work());
        })?;
        self.handles.push(handle);
        self.outstanding += 1;
        Ok(())
    }

    /// Receive the next worker result, waiting at most `wait`. `Ok(None)` means
    /// nothing arrived in time or every worker has already reported.
    fn next(&mut self, wait: Duration) -> io::Result<Option<Event>> {
        match self.events.recv_timeout(wait) {
            Ok(event) => {
                self.outstanding -= 1;
                Ok(Some(event))
            }
            Err(RecvTimeoutError::Timeout) => Ok(None),
            Err(RecvTimeoutError::Disconnected) if self.outstanding == 0 => Ok(None),
            Err(RecvTimeoutError::Disconnected) => Err(io::Error::other("command output reader stopped")),
        }
    }

    /// Wait up to `grace` for outstanding results, then join every worker that
    /// finished. A worker still blocked on a pipe is detached, never joined.
    fn join(self, grace: Duration) {
        let Self { handles, sender, events, mut outstanding } = self;
        drop(sender);
        let deadline = Deadline::start(grace);
        while outstanding > 0 {
            match events.recv_timeout(deadline.remaining()) {
                Ok(_) => outstanding -= 1,
                Err(_) => break,
            }
        }
        for handle in handles {
            if outstanding == 0 || handle.is_finished() {
                let _ = handle.join();
            }
        }
    }
}

#[derive(Default)]
struct Collected {
    status: Option<ExitStatus>,
    stdout: Option<Vec<u8>>,
    stderr: Option<Vec<u8>>,
}

impl Collected {
    fn apply(&mut self, event: Event) -> io::Result<()> {
        match event {
            Event::Stdin(result) => result?,
            Event::Stdout(result) => self.stdout = Some(result?),
            Event::Stderr(result) => self.stderr = Some(result?),
        }
        Ok(())
    }
}

fn supervise(child: &mut Child, workers: &mut Workers, deadline: &Deadline) -> io::Result<CommandOutput> {
    let mut collected = Collected::default();
    loop {
        if let (Some(status), 0) = (collected.status, workers.outstanding) {
            return Ok(CommandOutput {
                status,
                stdout: collected.stdout.take().unwrap_or_default(),
                stderr: collected.stderr.take().unwrap_or_default(),
            });
        }
        let remaining = deadline.remaining();
        if remaining.is_zero() {
            return Err(deadline.error());
        }
        if collected.status.is_none() {
            // Surface an early writer or reader failure without waiting for the child.
            if let Some(event) = workers.next(Duration::ZERO)? {
                collected.apply(event)?;
                continue;
            }
            collected.status = child.wait_timeout(remaining.min(WORKER_POLL))?;
        } else if let Some(event) = workers.next(remaining)? {
            collected.apply(event)?;
        }
    }
}

fn read_bounded_output(reader: &mut impl Read, limit: usize) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(limit.saturating_add(1) as u64).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "command output exceeded size limit"));
    }
    Ok(bytes)
}

#[cfg(unix)]
fn configure_process_group(command: &mut Command) {
    use std::os::unix::process::CommandExt;

    command.process_group(0);
}

#[cfg(not(unix))]
fn configure_process_group(_: &mut Command) {}

/// Kill the child and, on Unix, the process group it leads, then reap the child.
///
/// The child is its own group leader, so the negative pid never addresses the
/// parent's group. The kernel keeps a pid reserved while a process group with
/// that id has members, so signaling the group after the leader was reaped
/// still reaches only its descendants.
fn terminate_child(child: &mut Child) {
    #[cfg(unix)]
    if let Ok(group_id) = i32::try_from(child.id()) {
        let _ = nix::sys::signal::kill(nix::unistd::Pid::from_raw(-group_id), nix::sys::signal::Signal::SIGKILL);
    }
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[test]
    fn command_output_reader_enforces_its_byte_limit() {
        let error = read_bounded_output(&mut Cursor::new(b"abcd"), 3).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }

    #[cfg(unix)]
    mod unix {
        use std::{fs, path::PathBuf};

        use nix::{errno::Errno, sys::signal, unistd::Pid};
        use tempfile::TempDir;

        use super::*;

        const ASSERTION_BOUND: Duration = Duration::from_secs(2);
        const INPUT_BYTES: usize = 1024 * 1024;

        /// Records a pid written by a test child and kills that process and its
        /// group on drop so a failed assertion never leaks a sleeping child.
        struct PidFile {
            path: PathBuf,
            _temp: TempDir,
        }

        impl PidFile {
            fn new() -> Self {
                let temp = TempDir::new().unwrap();
                Self { path: temp.path().join("pid"), _temp: temp }
            }

            fn recorded(&self) -> Option<i32> {
                fs::read_to_string(&self.path).ok()?.trim().parse().ok()
            }

            fn pid(&self) -> i32 {
                self.recorded().expect("test child recorded its pid")
            }
        }

        impl Drop for PidFile {
            fn drop(&mut self) {
                if let Some(pid) = self.recorded() {
                    let _ = signal::kill(Pid::from_raw(-pid), signal::Signal::SIGKILL);
                    let _ = signal::kill(Pid::from_raw(pid), signal::Signal::SIGKILL);
                }
            }
        }

        fn shell(script: &str, pid_file: &PidFile) -> Command {
            let mut command = Command::new("/bin/sh");
            command.arg("-c").arg(script).arg(&pid_file.path);
            command
        }

        fn process_is_gone(pid: i32) -> bool {
            let pid = Pid::from_raw(pid);
            (0..200).any(|_| match signal::kill(pid, None) {
                Err(Errno::ESRCH) => true,
                _ => {
                    thread::sleep(Duration::from_millis(10));
                    false
                }
            })
        }

        #[test]
        fn blocked_stdin_delivery_respects_the_timeout_and_kills_the_child() {
            let pid_file = PidFile::new();
            let input = vec![b'x'; INPUT_BYTES];
            let started = Instant::now();

            let error = run_output_with_input_timeout(
                &mut shell("echo $$ >\"$0\"; exec sleep 30", &pid_file),
                Some(&input),
                Duration::from_millis(50),
            )
            .unwrap_err();

            assert_eq!(error.kind(), io::ErrorKind::TimedOut);
            assert!(started.elapsed() < ASSERTION_BOUND);
            assert!(process_is_gone(pid_file.pid()), "timed-out child survived cleanup");
        }

        #[test]
        fn descendant_holding_output_pipes_is_killed_after_the_child_exits() {
            let pid_file = PidFile::new();
            let started = Instant::now();

            let error =
                run_output_timeout(&mut shell("sleep 30 & echo $! >\"$0\"", &pid_file), Duration::from_millis(50))
                    .unwrap_err();

            assert_eq!(error.kind(), io::ErrorKind::TimedOut);
            assert!(started.elapsed() < ASSERTION_BOUND);
            assert!(process_is_gone(pid_file.pid()), "descendant survived process-group cleanup");
        }

        #[test]
        fn immediate_write_failure_terminates_the_child_before_the_deadline() {
            let pid_file = PidFile::new();
            let input = vec![b'x'; INPUT_BYTES];
            let started = Instant::now();

            let error = run_output_with_input_timeout(
                &mut shell("echo $$ >\"$0\"; exec 0<&-; exec sleep 30", &pid_file),
                Some(&input),
                Duration::from_secs(30),
            )
            .unwrap_err();

            assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
            assert!(started.elapsed() < ASSERTION_BOUND);
            assert!(process_is_gone(pid_file.pid()), "child survived write-failure cleanup");
        }

        #[test]
        fn output_limit_failure_terminates_a_still_running_child() {
            let pid_file = PidFile::new();
            let script = format!("echo $$ >\"$0\"; head -c {} /dev/zero; exec sleep 30", MAX_COMMAND_OUTPUT_BYTES + 1);
            let started = Instant::now();

            let error = run_output_timeout(&mut shell(&script, &pid_file), Duration::from_secs(30)).unwrap_err();

            assert_eq!(error.kind(), io::ErrorKind::InvalidData);
            assert!(started.elapsed() < ASSERTION_BOUND);
            assert!(process_is_gone(pid_file.pid()), "child survived output-limit cleanup");
        }

        #[test]
        fn large_input_and_both_outputs_flow_concurrently() {
            let pid_file = PidFile::new();
            let input = (0..INPUT_BYTES).map(|index| index as u8).collect::<Vec<_>>();

            let output = run_output_with_input_timeout(
                &mut shell(&format!("cat; head -c {INPUT_BYTES} /dev/zero >&2"), &pid_file),
                Some(&input),
                Duration::from_secs(30),
            )
            .unwrap();

            assert!(output.status.success());
            assert_eq!(output.stdout, input);
            assert_eq!(output.stderr.len(), INPUT_BYTES);
        }

        #[test]
        fn preserves_exit_status_and_output_of_finished_children() {
            let pid_file = PidFile::new();

            let failure = run_output_timeout(
                &mut shell("printf out; printf err >&2; exit 3", &pid_file),
                Duration::from_secs(30),
            )
            .unwrap();
            assert_eq!(failure.status.code(), Some(3));
            assert_eq!(failure.stdout, b"out");
            assert_eq!(failure.stderr, b"err");

            let success =
                run_output_with_input_timeout(&mut shell("cat", &pid_file), Some(b"echoed"), Duration::from_secs(30))
                    .unwrap();
            assert!(success.status.success());
            assert_eq!(success.stdout, b"echoed");
            assert!(success.stderr.is_empty());
        }
    }
}
