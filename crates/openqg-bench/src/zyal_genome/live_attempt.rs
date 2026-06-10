//! Self-contained subprocess LLM transport (V6 transitional).
//!
//! Extracted from the V3 `jailgun_live.rs`/`live_call.rs` stack so the jekko proposer keeps
//! working through the V3 purge. The prompt goes to the child's stdin; stdout/stderr are drained
//! on reader threads; the child runs in its own session (`setsid`) so a timeout kill cannot reach
//! siblings. Scheduled for deletion when the router-native proposer (direct HTTP) lands.

use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{bail, Result};
use serde_json::{json, Value};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

#[cfg(unix)]
const SIGKILL_NUM: i32 = 9;

#[cfg(unix)]
extern "C" {
    fn setsid() -> i32;
    fn kill(pid: i32, sig: i32) -> i32;
}

/// One subprocess LLM call's outcome.
#[allow(dead_code)] // exit_code/metadata are diagnostic fields read by future transports
pub(crate) struct LiveAttempt {
    pub(crate) status: String,
    pub(crate) exit_code: Option<i32>,
    pub(crate) elapsed_seconds: f64,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) error: Option<String>,
    pub(crate) metadata: Value,
}

impl LiveAttempt {
    pub(crate) fn status(&self) -> &str {
        &self.status
    }
    pub(crate) fn stdout(&self) -> &str {
        &self.stdout
    }
    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    pub(crate) fn elapsed_seconds(&self) -> f64 {
        self.elapsed_seconds
    }
    pub(crate) fn stderr_excerpt(&self) -> &str {
        &self.stderr
    }
}

fn spawn_pipe_reader<R>(mut reader: R) -> mpsc::Receiver<String>
where
    R: Read + Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut buffer = String::new();
        let _ = reader.read_to_string(&mut buffer);
        let _ = tx.send(buffer);
    });
    rx
}

fn collect_pipe(rx: Option<mpsc::Receiver<String>>, timeout: Duration) -> Option<String> {
    rx.and_then(|rx| rx.recv_timeout(timeout).ok())
}

fn kill_live_process_tree(child_pid: u32) {
    #[cfg(unix)]
    // SAFETY: kill(2) is async-signal-safe and takes only integer arguments; sending SIGKILL to
    // the process group and pid cannot create memory-safety hazards in this process.
    unsafe {
        let pid = child_pid as i32;
        let _ = kill(-pid, SIGKILL_NUM);
        let _ = kill(pid, SIGKILL_NUM);
    }
    #[cfg(not(unix))]
    {
        let _ = child_pid;
    }
}

/// Run one subprocess LLM call: prompt via stdin, response on stdout, hard timeout with a
/// process-group kill.
#[allow(dead_code)]
pub(crate) fn run_live_call_attempt(
    command: &[String],
    prompt: &str,
    timeout_seconds: u64,
    attempt: usize,
    started_at: &str,
) -> Result<LiveAttempt> {
    run_live_call_attempt_env(command, prompt, timeout_seconds, attempt, started_at, &[])
}

/// As [`run_live_call_attempt`], but sets extra environment variables on the spawned process —
/// used to forward `JEKKO_RUN_QUALITY_BAND` so a call routes to a jnoccio quality band.
pub(crate) fn run_live_call_attempt_env(
    command: &[String],
    prompt: &str,
    timeout_seconds: u64,
    attempt: usize,
    _started_at: &str,
    extra_env: &[(String, String)],
) -> Result<LiveAttempt> {
    if command.is_empty() {
        bail!("live command is empty");
    }
    let start = Instant::now();
    let mut cmd = Command::new(&command[0]);
    cmd.args(&command[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in extra_env {
        cmd.env(key, value);
    }
    #[cfg(unix)]
    // SAFETY: pre_exec runs in the forked child before exec; setsid() is async-signal-safe and
    // the closure performs no allocation or non-reentrant work beyond the single libc call.
    unsafe {
        cmd.pre_exec(|| {
            if setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = cmd.spawn().map_err(|err| anyhow::anyhow!(err))?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(prompt.as_bytes());
    }
    let stdout_rx = child.stdout.take().map(spawn_pipe_reader);
    let stderr_rx = child.stderr.take().map(spawn_pipe_reader);
    let mut exit_code = None;
    let status;
    let mut error = None;
    let timed_out;
    loop {
        if let Some(exit) = child.try_wait()? {
            exit_code = exit.code();
            status = if exit.success() { "ok" } else { "failed" }.to_string();
            timed_out = false;
            break;
        }
        if start.elapsed() >= Duration::from_secs(timeout_seconds) {
            kill_live_process_tree(child.id());
            let _ = child.wait();
            status = "timeout".to_string();
            error = Some(format!(
                "timeout after {timeout_seconds}s on attempt {attempt}"
            ));
            timed_out = true;
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    let pipe_wait = if timed_out {
        Duration::from_secs(1)
    } else {
        Duration::from_secs(5)
    };
    // Explicit fallbacks: a drained pipe after a timeout is an expected state, not an error —
    // the placeholder text documents the gap in the captured stream.
    let stdout = match collect_pipe(stdout_rx, pipe_wait) {
        Some(s) => s,
        None => "[stdout unavailable after timeout]\n".to_string(),
    };
    let stderr = match collect_pipe(stderr_rx, pipe_wait) {
        Some(s) => s,
        None => "[stderr unavailable after timeout]\n".to_string(),
    };
    Ok(LiveAttempt {
        status,
        exit_code,
        elapsed_seconds: start.elapsed().as_secs_f64(),
        stdout,
        stderr,
        error,
        metadata: json!({}),
    })
}
