use std::{fmt, sync::Arc};

use sha2::{Digest, Sha256};

use crate::{
    domain::{Identity, ProcessFingerprint, ProcessLiveness, ProcessProbe},
    error::{AppError, Result},
};

const MAX_ANCESTORS: usize = 16;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LivenessObservation {
    pub(crate) identity: Identity,
    pub(crate) expected_fingerprint: Option<ProcessFingerprint>,
    pub(crate) liveness: ProcessLiveness,
}

/// Probe session processes without consulting or updating provider inventory caches.
pub(crate) fn process_sweep<P, I>(probe: &P, sessions: I) -> Vec<LivenessObservation>
where
    P: ProcessProbe + ?Sized,
    I: IntoIterator<Item = (Identity, Option<ProcessFingerprint>)>,
{
    sessions
        .into_iter()
        .map(|(identity, expected_fingerprint)| {
            let liveness =
                expected_fingerprint.as_ref().map_or(ProcessLiveness::Unknown, |value| probe.liveness(value));
            LivenessObservation { identity, expected_fingerprint, liveness }
        })
        .collect()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NativeState {
    Alive,
    Dead,
    Unknown,
}

#[derive(Clone, Debug)]
struct NativeProcess {
    pid: u32,
    parent_pid: u32,
    state: NativeState,
    start_marker: String,
    codex_match: bool,
    claude_match: bool,
}

#[derive(Clone, Debug)]
enum InspectionError {
    Missing,
    Permission(String),
    Other(String),
}

impl fmt::Display for InspectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing => formatter.write_str("process does not exist"),
            Self::Permission(detail) | Self::Other(detail) => formatter.write_str(detail),
        }
    }
}

trait ProcessBackend: Send + Sync {
    fn boot_marker(&self) -> std::result::Result<String, InspectionError>;
    fn inspect(&self, pid: u32, identify_host: bool) -> std::result::Result<NativeProcess, InspectionError>;
}

/// OS-native process inspection for macOS and Linux.
///
/// Fingerprint tokens are intentionally opaque. They bind the OS boot identity to
/// the kernel's process start marker, so both PID reuse and a reboot invalidate an
/// old fingerprint.
#[derive(Clone)]
pub(crate) struct NativeProcessProbe {
    backend: Arc<dyn ProcessBackend>,
    boot_marker: std::result::Result<String, InspectionError>,
}

impl fmt::Debug for NativeProcessProbe {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeProcessProbe")
            .field("boot_marker_available", &self.boot_marker.is_ok())
            .finish_non_exhaustive()
    }
}

impl Default for NativeProcessProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeProcessProbe {
    pub(crate) fn new() -> Self {
        let backend: Arc<dyn ProcessBackend> = Arc::new(NativeBackend);
        let boot_marker = backend.boot_marker();
        Self { backend, boot_marker }
    }

    #[cfg(test)]
    fn with_backend(backend: Arc<dyn ProcessBackend>) -> Self {
        let boot_marker = backend.boot_marker();
        Self { backend, boot_marker }
    }

    fn token(&self, process: &NativeProcess) -> std::result::Result<String, InspectionError> {
        let boot = self.boot_marker.as_ref().map_err(Clone::clone)?;
        let mut digest = Sha256::new();
        digest.update(b"ai-coord-process-v1\0");
        digest.update(boot.as_bytes());
        digest.update(b"\0");
        digest.update(process.start_marker.as_bytes());
        Ok(format!("v1:{}", hex_bytes(&digest.finalize())))
    }

    fn fingerprint_process(&self, process: &NativeProcess) -> std::result::Result<ProcessFingerprint, InspectionError> {
        Ok(ProcessFingerprint { pid: process.pid, start_token: Some(self.token(process)?) })
    }

    pub(crate) fn ancestors(&self, start_pid: u32) -> Vec<ProcessFingerprint> {
        let mut pid = start_pid;
        let mut result = Vec::new();
        for _ in 0..MAX_ANCESTORS {
            if pid <= 1 {
                break;
            }
            let Ok(process) = self.backend.inspect(pid, false) else {
                break;
            };
            let parent = process.parent_pid;
            if let Ok(fingerprint) = self.fingerprint_process(&process) {
                result.push(fingerprint);
            }
            if parent == 0 || parent == pid {
                break;
            }
            pid = parent;
        }
        result
    }

    /// Capture the actual Codex or Claude host ancestor, skipping hook shells.
    /// Command-line data used for matching is discarded inside the OS backend.
    pub(crate) fn host_ancestor(
        &self,
        client: crate::domain::Client,
        start_pid: u32,
    ) -> Result<Option<ProcessFingerprint>> {
        let mut pid = start_pid;
        for _ in 0..MAX_ANCESTORS {
            if pid <= 1 {
                break;
            }
            let process = match self.backend.inspect(pid, true) {
                Ok(process) => process,
                Err(InspectionError::Missing) => return Ok(None),
                Err(error) => {
                    return Err(AppError::operational(format!("could not inspect process ancestor {pid}: {error}")));
                }
            };
            let is_match = match client {
                crate::domain::Client::Codex => process.codex_match,
                crate::domain::Client::Claude => process.claude_match,
            };
            if is_match {
                return self.fingerprint_process(&process).map(Some).map_err(|error| {
                    AppError::operational(format!("could not fingerprint host process {pid}: {error}"))
                });
            }
            if process.parent_pid == 0 || process.parent_pid == pid {
                break;
            }
            pid = process.parent_pid;
        }
        Ok(None)
    }
}

impl ProcessProbe for NativeProcessProbe {
    fn fingerprint(&self, pid: u32) -> Result<ProcessFingerprint> {
        if pid == 0 {
            return Err(AppError::operational("process ID must be positive"));
        }
        let process = self
            .backend
            .inspect(pid, false)
            .map_err(|error| AppError::operational(format!("could not inspect process {pid}: {error}")))?;
        if process.state != NativeState::Alive {
            return Err(AppError::operational(format!("process {pid} is not alive")));
        }
        self.fingerprint_process(&process)
            .map_err(|error| AppError::operational(format!("could not fingerprint process {pid}: {error}")))
    }

    fn liveness(&self, fingerprint: &ProcessFingerprint) -> ProcessLiveness {
        if fingerprint.pid == 0 {
            return ProcessLiveness::Dead;
        }
        let process = match self.backend.inspect(fingerprint.pid, false) {
            Ok(process) => process,
            Err(InspectionError::Missing) => return ProcessLiveness::Dead,
            Err(InspectionError::Permission(_) | InspectionError::Other(_)) => {
                return ProcessLiveness::Unknown;
            }
        };
        match process.state {
            NativeState::Dead => return ProcessLiveness::Dead,
            NativeState::Unknown => return ProcessLiveness::Unknown,
            NativeState::Alive => {}
        }
        let Some(expected) = fingerprint.start_token.as_deref() else {
            return ProcessLiveness::Unknown;
        };
        match self.token(&process) {
            Ok(actual) if actual == expected => ProcessLiveness::Alive,
            Ok(_) => ProcessLiveness::Dead,
            Err(_) => ProcessLiveness::Unknown,
        }
    }
}

fn host_matches(name: &str, client: &str) -> bool {
    let normalized = name.trim_matches('\0').trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return false;
    }
    let basename = normalized.rsplit(['/', '\\']).next().unwrap_or(normalized.as_str()).trim_end_matches(".exe");
    if basename == client {
        return true;
    }
    let stem = basename.strip_suffix(".js").or_else(|| basename.strip_suffix(".mjs")).unwrap_or(basename);
    stem == client ||
        normalized.split(['/', '\\']).any(|part| {
            part == client ||
                part.starts_with(&format!("{client}@")) ||
                (client == "claude" && part.starts_with("claude-code"))
        })
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

#[cfg(target_os = "linux")]
#[derive(Debug)]
struct NativeBackend;

#[cfg(target_os = "linux")]
impl ProcessBackend for NativeBackend {
    fn boot_marker(&self) -> std::result::Result<String, InspectionError> {
        std::fs::read_to_string("/proc/sys/kernel/random/boot_id")
            .map(|value| value.trim().to_owned())
            .map_err(classify_io)
            .and_then(|value| {
                if value.is_empty() {
                    Err(InspectionError::Other("kernel boot ID is empty".to_owned()))
                } else {
                    Ok(value)
                }
            })
    }

    fn inspect(&self, pid: u32, identify_host: bool) -> std::result::Result<NativeProcess, InspectionError> {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).map_err(classify_io)?;
        let close = stat.rfind(')').ok_or_else(|| InspectionError::Other("malformed /proc process stat".to_owned()))?;
        let fields: Vec<&str> = stat[close + 1..].split_whitespace().collect();
        if fields.len() <= 19 {
            return Err(InspectionError::Other("short /proc process stat".to_owned()));
        }
        let state = match fields[0].as_bytes().first().copied() {
            Some(b'Z' | b'X' | b'x') => NativeState::Dead,
            Some(b'R' | b'S' | b'D' | b'I' | b'T' | b't' | b'W') => NativeState::Alive,
            Some(_) => NativeState::Unknown,
            None => return Err(InspectionError::Other("missing /proc process state".to_owned())),
        };
        let parent_pid =
            fields[1].parse::<u32>().map_err(|_| InspectionError::Other("invalid /proc parent PID".to_owned()))?;
        let start_marker = fields[19].to_owned();

        let mut names = Vec::new();
        if identify_host {
            if let Ok(comm) = std::fs::read_to_string(format!("/proc/{pid}/comm")) {
                names.push(comm);
            }
            if let Ok(executable) = std::fs::read_link(format!("/proc/{pid}/exe")) {
                names.push(executable.to_string_lossy().into_owned());
            }
            if let Ok(command_line) = std::fs::read(format!("/proc/{pid}/cmdline")) {
                names.extend(
                    command_line
                        .split(|byte| *byte == 0)
                        .filter(|value| !value.is_empty())
                        .filter_map(|value| std::str::from_utf8(value).ok())
                        .take(3)
                        .map(str::to_owned),
                );
            }
        }
        Ok(NativeProcess {
            pid,
            parent_pid,
            state,
            start_marker,
            codex_match: names.iter().any(|name| host_matches(name, "codex")),
            claude_match: names.iter().any(|name| host_matches(name, "claude")),
        })
    }
}

#[cfg(target_os = "linux")]
fn classify_io(error: std::io::Error) -> InspectionError {
    match error.kind() {
        std::io::ErrorKind::NotFound => InspectionError::Missing,
        std::io::ErrorKind::PermissionDenied => InspectionError::Permission(error.to_string()),
        _ => InspectionError::Other(error.to_string()),
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use std::{ffi::CStr, mem::MaybeUninit};

    use super::*;

    const PROC_PIDTBSDINFO: i32 = 3;
    const PROC_PIDPATHINFO_MAXSIZE: usize = 4096;
    const SZOMB: u32 = 5;
    const SIDL: u32 = 1;
    const SRUN: u32 = 2;
    const SSLEEP: u32 = 3;
    const SSTOP: u32 = 4;

    #[repr(C)]
    struct ProcBsdInfo {
        pbi_flags: u32,
        pbi_status: u32,
        pbi_xstatus: u32,
        pbi_pid: u32,
        pbi_ppid: u32,
        pbi_uid: libc::uid_t,
        pbi_gid: libc::gid_t,
        pbi_ruid: libc::uid_t,
        pbi_rgid: libc::gid_t,
        pbi_svuid: libc::uid_t,
        pbi_svgid: libc::gid_t,
        rfu_1: u32,
        pbi_comm: [libc::c_char; 16],
        pbi_name: [libc::c_char; 32],
        pbi_nfiles: u32,
        pbi_pgid: u32,
        pbi_pjobc: u32,
        e_tdev: u32,
        e_tpgid: u32,
        pbi_nice: i32,
        pbi_start_tvsec: u64,
        pbi_start_tvusec: u64,
    }

    unsafe extern "C" {
        fn proc_pidinfo(
            pid: libc::c_int,
            flavor: libc::c_int,
            arg: u64,
            buffer: *mut libc::c_void,
            buffer_size: libc::c_int,
        ) -> libc::c_int;
        fn proc_pidpath(pid: libc::c_int, buffer: *mut libc::c_void, buffer_size: u32) -> libc::c_int;
    }

    #[derive(Debug)]
    pub(super) struct NativeBackend;

    impl ProcessBackend for NativeBackend {
        fn boot_marker(&self) -> std::result::Result<String, InspectionError> {
            sysctl_string(c"kern.bootsessionuuid")
        }

        fn inspect(&self, pid: u32, identify_host: bool) -> std::result::Result<NativeProcess, InspectionError> {
            let mut info = MaybeUninit::<ProcBsdInfo>::zeroed();
            // SAFETY: `info` points to a correctly sized writable `proc_bsdinfo` buffer.
            let read = unsafe {
                proc_pidinfo(
                    pid as libc::c_int,
                    PROC_PIDTBSDINFO,
                    0,
                    info.as_mut_ptr().cast(),
                    size_of::<ProcBsdInfo>() as libc::c_int,
                )
            };
            if read != size_of::<ProcBsdInfo>() as libc::c_int {
                return Err(last_inspection_error());
            }
            // SAFETY: `proc_pidinfo` filled the entire structure above.
            let info = unsafe { info.assume_init() };
            let state = match info.pbi_status {
                SZOMB => NativeState::Dead,
                SIDL | SRUN | SSLEEP | SSTOP => NativeState::Alive,
                _ => NativeState::Unknown,
            };
            let mut names = Vec::new();
            if identify_host {
                names.push(c_char_array(&info.pbi_comm));
                names.push(c_char_array(&info.pbi_name));
                let mut executable = [0_u8; PROC_PIDPATHINFO_MAXSIZE];
                // SAFETY: the byte array is writable and its supplied size is exact.
                let length = unsafe {
                    proc_pidpath(pid as libc::c_int, executable.as_mut_ptr().cast(), executable.len() as u32)
                };
                if length > 0 {
                    names.push(String::from_utf8_lossy(&executable[..length as usize]).into_owned());
                }
                if let Ok(arguments) = process_arguments(pid) {
                    names.extend(arguments.into_iter().take(3));
                }
            }
            Ok(NativeProcess {
                pid,
                parent_pid: info.pbi_ppid,
                state,
                start_marker: format!("{}:{}", info.pbi_start_tvsec, info.pbi_start_tvusec),
                codex_match: names.iter().any(|name| host_matches(name, "codex")),
                claude_match: names.iter().any(|name| host_matches(name, "claude")),
            })
        }
    }

    fn c_char_array<const N: usize>(value: &[libc::c_char; N]) -> String {
        let bytes: Vec<u8> = value.iter().map(|byte| *byte as u8).take_while(|byte| *byte != 0).collect();
        String::from_utf8_lossy(&bytes).into_owned()
    }

    fn sysctl_string(name: &CStr) -> std::result::Result<String, InspectionError> {
        let mut length = 0_usize;
        // SAFETY: this first call requests the required output length only.
        if unsafe { libc::sysctlbyname(name.as_ptr(), std::ptr::null_mut(), &mut length, std::ptr::null_mut(), 0) } != 0
        {
            return Err(last_inspection_error());
        }
        let mut bytes = vec![0_u8; length];
        // SAFETY: `bytes` has the size returned by the preceding sysctl call.
        if unsafe { libc::sysctlbyname(name.as_ptr(), bytes.as_mut_ptr().cast(), &mut length, std::ptr::null_mut(), 0) } !=
            0
        {
            return Err(last_inspection_error());
        }
        bytes.truncate(length);
        let value = String::from_utf8_lossy(&bytes).trim_matches('\0').trim().to_owned();
        if value.is_empty() {
            Err(InspectionError::Other("kernel boot session ID is empty".to_owned()))
        } else {
            Ok(value)
        }
    }

    fn process_arguments(pid: u32) -> std::result::Result<Vec<String>, InspectionError> {
        let mut mib = [libc::CTL_KERN, libc::KERN_PROCARGS2, pid as libc::c_int];
        let mut length = 0_usize;
        // SAFETY: the MIB is valid and this first call requests only the result length.
        if unsafe {
            libc::sysctl(mib.as_mut_ptr(), mib.len() as u32, std::ptr::null_mut(), &mut length, std::ptr::null_mut(), 0)
        } != 0
        {
            return Err(last_inspection_error());
        }
        let mut bytes = vec![0_u8; length];
        // SAFETY: `bytes` has the size returned by the preceding sysctl call.
        if unsafe {
            libc::sysctl(
                mib.as_mut_ptr(),
                mib.len() as u32,
                bytes.as_mut_ptr().cast(),
                &mut length,
                std::ptr::null_mut(),
                0,
            )
        } != 0
        {
            return Err(last_inspection_error());
        }
        bytes.truncate(length);
        if bytes.len() < size_of::<libc::c_int>() {
            return Err(InspectionError::Other("short process argument data".to_owned()));
        }
        let mut argc_bytes = [0_u8; size_of::<libc::c_int>()];
        argc_bytes.copy_from_slice(&bytes[..size_of::<libc::c_int>()]);
        let argc = i32::from_ne_bytes(argc_bytes);
        if argc < 0 {
            return Err(InspectionError::Other("invalid process argument count".to_owned()));
        }
        let mut offset = size_of::<libc::c_int>();
        // Skip the executable path and the padding before argv[0].
        while offset < bytes.len() && bytes[offset] != 0 {
            offset += 1;
        }
        while offset < bytes.len() && bytes[offset] == 0 {
            offset += 1;
        }
        let mut arguments = Vec::with_capacity(argc as usize);
        for _ in 0..argc {
            if offset >= bytes.len() {
                break;
            }
            let end = bytes[offset..].iter().position(|byte| *byte == 0).map_or(bytes.len(), |index| offset + index);
            arguments.push(String::from_utf8_lossy(&bytes[offset..end]).into_owned());
            offset = end.saturating_add(1);
        }
        Ok(arguments)
    }

    fn last_inspection_error() -> InspectionError {
        let error = std::io::Error::last_os_error();
        match error.raw_os_error() {
            Some(libc::ESRCH) => InspectionError::Missing,
            Some(libc::EPERM | libc::EACCES) => InspectionError::Permission(error.to_string()),
            _ => InspectionError::Other(error.to_string()),
        }
    }
}

#[cfg(target_os = "macos")]
use macos::NativeBackend;

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
#[derive(Debug)]
struct NativeBackend;

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
impl ProcessBackend for NativeBackend {
    fn boot_marker(&self) -> std::result::Result<String, InspectionError> {
        Err(InspectionError::Other("process probing is supported only on macOS and Linux".to_owned()))
    }

    fn inspect(&self, _pid: u32, _identify_host: bool) -> std::result::Result<NativeProcess, InspectionError> {
        Err(InspectionError::Other("process probing is supported only on macOS and Linux".to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        process::Command,
        sync::{Arc, Mutex},
    };

    use super::*;
    use crate::domain::Client;

    #[derive(Debug)]
    struct FakeBackend {
        boot: std::result::Result<String, InspectionError>,
        processes: Mutex<HashMap<u32, std::result::Result<NativeProcess, InspectionError>>>,
    }

    impl ProcessBackend for FakeBackend {
        fn boot_marker(&self) -> std::result::Result<String, InspectionError> {
            self.boot.clone()
        }

        fn inspect(&self, pid: u32, _identify_host: bool) -> std::result::Result<NativeProcess, InspectionError> {
            self.processes.lock().unwrap().get(&pid).cloned().unwrap_or(Err(InspectionError::Missing))
        }
    }

    fn fake_process(pid: u32, parent_pid: u32, state: NativeState, marker: &str) -> NativeProcess {
        NativeProcess {
            pid,
            parent_pid,
            state,
            start_marker: marker.to_owned(),
            codex_match: false,
            claude_match: false,
        }
    }

    fn fake_probe(
        rows: impl IntoIterator<Item = (u32, std::result::Result<NativeProcess, InspectionError>)>,
    ) -> NativeProcessProbe {
        NativeProcessProbe::with_backend(Arc::new(FakeBackend {
            boot: Ok("boot-a".to_owned()),
            processes: Mutex::new(rows.into_iter().collect()),
        }))
    }

    #[test]
    fn classifies_live_dead_unknown_and_stale_tokens() {
        let probe = fake_probe([
            (10, Ok(fake_process(10, 1, NativeState::Alive, "one"))),
            (11, Ok(fake_process(11, 1, NativeState::Dead, "two"))),
            (12, Err(InspectionError::Permission("denied".to_owned()))),
        ]);
        let live = probe.fingerprint(10).unwrap();
        let stale = ProcessFingerprint { pid: 10, start_token: Some("v1:stale".to_owned()) };
        let no_token = ProcessFingerprint { pid: 10, start_token: None };

        assert_eq!(probe.liveness(&live), ProcessLiveness::Alive);
        assert_eq!(probe.liveness(&stale), ProcessLiveness::Dead);
        assert_eq!(probe.liveness(&no_token), ProcessLiveness::Unknown);
        assert_eq!(
            probe.liveness(&ProcessFingerprint { pid: 11, start_token: Some("anything".to_owned()) }),
            ProcessLiveness::Dead
        );
        assert_eq!(
            probe.liveness(&ProcessFingerprint { pid: 12, start_token: Some("anything".to_owned()) }),
            ProcessLiveness::Unknown
        );
        assert_eq!(
            probe.liveness(&ProcessFingerprint { pid: 13, start_token: Some("anything".to_owned()) }),
            ProcessLiveness::Dead
        );
    }

    #[test]
    fn captures_matching_host_above_shell_without_cross_client_match() {
        let mut shell = fake_process(30, 20, NativeState::Alive, "shell");
        let mut codex = fake_process(20, 10, NativeState::Alive, "codex");
        codex.codex_match = true;
        let mut claude = fake_process(10, 1, NativeState::Alive, "claude");
        claude.claude_match = true;
        shell.codex_match = false;
        let probe = fake_probe([(30, Ok(shell)), (20, Ok(codex)), (10, Ok(claude))]);

        let codex = probe.host_ancestor(Client::Codex, 30).unwrap().unwrap();
        let claude = probe.host_ancestor(Client::Claude, 30).unwrap().unwrap();
        assert_eq!(codex.pid, 20);
        assert_eq!(claude.pid, 10);
    }

    #[test]
    fn process_sweep_preserves_identity_and_expected_evidence() {
        let probe = fake_probe([(10, Ok(fake_process(10, 1, NativeState::Alive, "one")))]);
        let fingerprint = probe.fingerprint(10).unwrap();
        let identity = Identity { client: Client::Codex, session_id: "session".to_owned() };
        let observations =
            process_sweep(&probe, [(identity.clone(), Some(fingerprint.clone())), (identity.clone(), None)]);

        assert_eq!(observations[0].expected_fingerprint, Some(fingerprint));
        assert_eq!(observations[0].liveness, ProcessLiveness::Alive);
        assert_eq!(observations[1].identity, identity);
        assert_eq!(observations[1].liveness, ProcessLiveness::Unknown);
    }

    #[test]
    fn native_probe_observes_current_child_exit() {
        let probe = NativeProcessProbe::new();
        let mut child = Command::new("sleep").arg("30").spawn().unwrap();
        let fingerprint = probe.fingerprint(child.id()).unwrap();
        assert_eq!(probe.liveness(&fingerprint), ProcessLiveness::Alive);

        child.kill().unwrap();
        child.wait().unwrap();
        assert_eq!(probe.liveness(&fingerprint), ProcessLiveness::Dead);
    }

    #[cfg(unix)]
    #[test]
    fn native_probe_treats_stopped_process_as_alive() {
        use nix::{
            sys::signal::{Signal, kill},
            unistd::Pid,
        };

        let probe = NativeProcessProbe::new();
        let mut child = Command::new("sleep").arg("30").spawn().unwrap();
        let pid = Pid::from_raw(child.id() as i32);
        let fingerprint = probe.fingerprint(child.id()).unwrap();
        kill(pid, Signal::SIGSTOP).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(25));
        assert_eq!(probe.liveness(&fingerprint), ProcessLiveness::Alive);

        kill(pid, Signal::SIGCONT).unwrap();
        child.kill().unwrap();
        child.wait().unwrap();
    }

    #[test]
    fn host_name_matching_is_exact_enough_for_wrappers() {
        assert!(host_matches("/usr/local/bin/codex", "codex"));
        assert!(host_matches("/packages/claude@1.2.3/cli.js", "claude"));
        assert!(!host_matches("ai-coord", "codex"));
        assert!(!host_matches("codex-helper", "codex"));
    }
}
