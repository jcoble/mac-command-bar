use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Emitter, Runtime};

pub use crate::agent_conversation::protocol::ToolTerminalIdentity;

pub const TERMINAL_OUTPUT_EVENT: &str = "terminal_output";
/// Per-session scrollback ring held by the backend. 16 MB is enough to survive a
/// genuinely long agent run (256 KB was ~2 minutes of a chatty build), and it is
/// the backend that has to hold it: a hidden view holds nothing, so a re-attach
/// can only replay what lives here.
const TERMINAL_SCROLLBACK_MAX_BYTES: usize = 16 * 1024 * 1024;
/// Hysteresis floor. Once the cap is exceeded, ONE drain takes the buffer down to
/// this — 75% of the cap — instead of shaving off exactly the overflow. Trimming
/// to the cap would make every subsequent 8 KB read memmove the whole 16 MB; this
/// way the O(n) drain amortizes over ~4 MB of output.
const TERMINAL_SCROLLBACK_TRIM_TO_BYTES: usize = TERMINAL_SCROLLBACK_MAX_BYTES / 4 * 3;

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TerminalKind {
    UserPty,
    AgentTool,
    RunConfiguration,
    BrowserAutomation,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalStartRequest {
    pub cwd: String,
    pub shell: Option<String>,
    pub cols: Option<u16>,
    pub rows: Option<u16>,
    pub owned_id: Option<String>,
    /// Give this a command line and the session runs that one command and exits, so the
    /// caller gets a real exit code. Leave it out and the session is the interactive
    /// shell it has always been.
    pub command: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSessionInfo {
    pub session_id: String,
    pub cwd: String,
    pub shell: String,
    pub cols: u16,
    pub rows: u16,
    pub pid: Option<u32>,
    pub started_at: u128,
    pub exited: bool,
    pub exit_code: Option<u32>,
    pub signal: Option<String>,
    pub kind: TerminalKind,
    /// The Command Bar-owned identity supplied when this terminal was started.
    /// This is additive so older consumers can continue to render the session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owned_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_terminal_identity: Option<ToolTerminalIdentity>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalOutputEvent {
    pub session_id: String,
    pub data: String,
    pub terminated: bool,
    pub exit_code: Option<u32>,
    pub signal: Option<String>,
}

#[derive(Clone, Default)]
pub struct TerminalRegistry {
    inner: Arc<Mutex<HashMap<String, TerminalSessionHandle>>>,
}

struct TerminalSessionHandle {
    info: TerminalSessionInfo,
    master: Box<dyn MasterPty + Send>,
    writer: Mutex<Box<dyn Write + Send>>,
    killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
    scrollback: Arc<Mutex<String>>,
}

pub fn start_terminal_session<R: Runtime>(
    app: tauri::AppHandle<R>,
    registry: &TerminalRegistry,
    request: TerminalStartRequest,
) -> Result<TerminalSessionInfo, String> {
    start_terminal_session_typed(app, registry, request, TerminalKind::UserPty, None)
}

fn start_terminal_session_typed<R: Runtime>(
    app: tauri::AppHandle<R>,
    registry: &TerminalRegistry,
    request: TerminalStartRequest,
    kind: TerminalKind,
    tool_terminal_identity: Option<ToolTerminalIdentity>,
) -> Result<TerminalSessionInfo, String> {
    let cwd = terminal_cwd_from_request(&request.cwd)?;
    let shell = terminal_shell_from_request(request.shell);
    let run_command = terminal_command_from_request(request.command);
    let size = terminal_size_from_request(request.cols, request.rows);
    let session_id = tool_terminal_identity
        .as_ref()
        .map(|identity| identity.terminal_id.clone())
        .unwrap_or_else(new_terminal_session_id);
    let started_at = timestamp_millis();

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(size)
        .map_err(|error| format!("Could not open terminal pty: {error}"))?;
    let mut command = CommandBuilder::new(&shell);
    for arg in terminal_shell_args(&shell, run_command.as_deref()) {
        command.arg(arg);
    }
    command.cwd(&cwd);
    command.env("TERM", "xterm-256color");
    command.env("COLORTERM", "truecolor");
    command.env("TERM_PROGRAM", crate::product_identity::TERM_PROGRAM);
    command.env("CLICOLOR", "1");
    command.env("CLICOLOR_FORCE", "1");
    command.env("FORCE_COLOR", "3");
    command.env("COLORFGBG", "15;0");
    command.env_remove("NO_COLOR");
    // Correlates the spawned agent process (and any hook files it writes) back to
    // the CommandBar-owned session. Borrowed from CMUX's CMUX_WORKSPACE_ID.
    if let Some(owned_id) = request
        .owned_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        command.env("COMMANDBAR_SESSION_ID", owned_id);
    }

    let child = pair
        .slave
        .spawn_command(command)
        .map_err(|error| format!("Could not start terminal shell: {error}"))?;
    let pid = child.process_id();
    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|error| format!("Could not clone terminal reader: {error}"))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|error| format!("Could not take terminal writer: {error}"))?;
    let killer = child.clone_killer();
    let scrollback = Arc::new(Mutex::new(String::new()));
    drop(pair.slave);

    let info = TerminalSessionInfo {
        session_id: session_id.clone(),
        cwd: cwd.display().to_string(),
        shell,
        cols: size.cols,
        rows: size.rows,
        pid,
        started_at,
        exited: false,
        exit_code: None,
        signal: None,
        kind,
        owned_id: request.owned_id,
        tool_terminal_identity,
    };

    registry.insert(
        session_id.clone(),
        TerminalSessionHandle {
            info: info.clone(),
            master: pair.master,
            writer: Mutex::new(writer),
            killer: Mutex::new(killer),
            scrollback: Arc::clone(&scrollback),
        },
    )?;

    spawn_terminal_reader(
        app.clone(),
        session_id.clone(),
        Arc::clone(&scrollback),
        reader,
    );
    spawn_terminal_waiter(app, registry.clone(), session_id, child);

    Ok(info)
}

pub fn list_terminal_sessions(
    registry: &TerminalRegistry,
) -> Result<Vec<TerminalSessionInfo>, String> {
    let sessions = registry
        .inner
        .lock()
        .map_err(|_| "Terminal session registry is unavailable".to_string())?;
    Ok(sessions
        .values()
        .map(|session| session.info.clone())
        .collect())
}

pub fn read_terminal_session_scrollback(
    registry: &TerminalRegistry,
    session_id: &str,
) -> Result<Option<String>, String> {
    let Some(session_id) = normalize_terminal_session_id(session_id) else {
        return Ok(None);
    };
    let scrollback = {
        let sessions = registry
            .inner
            .lock()
            .map_err(|_| "Terminal session registry is unavailable".to_string())?;
        let Some(session) = sessions.get(session_id) else {
            return Ok(None);
        };
        Arc::clone(&session.scrollback)
    };
    let scrollback = scrollback
        .lock()
        .map_err(|_| "Terminal scrollback is unavailable".to_string())?;
    Ok(Some(scrollback.clone()))
}

pub fn write_terminal_session(
    registry: &TerminalRegistry,
    session_id: &str,
    data: &str,
) -> Result<bool, String> {
    let Some(session_id) = normalize_terminal_session_id(session_id) else {
        return Ok(false);
    };
    let sessions = registry
        .inner
        .lock()
        .map_err(|_| "Terminal session registry is unavailable".to_string())?;
    let Some(session) = sessions.get(session_id) else {
        return Ok(false);
    };
    let mut writer = session
        .writer
        .lock()
        .map_err(|_| "Terminal writer is unavailable".to_string())?;
    writer
        .write_all(data.as_bytes())
        .and_then(|_| writer.flush())
        .map_err(|error| format!("Could not write terminal input: {error}"))?;
    Ok(true)
}

pub fn resize_terminal_session(
    registry: &TerminalRegistry,
    session_id: &str,
    cols: Option<u16>,
    rows: Option<u16>,
) -> Result<bool, String> {
    let Some(session_id) = normalize_terminal_session_id(session_id) else {
        return Ok(false);
    };
    let mut sessions = registry
        .inner
        .lock()
        .map_err(|_| "Terminal session registry is unavailable".to_string())?;
    let Some(session) = sessions.get_mut(session_id) else {
        return Ok(false);
    };
    let size = terminal_size_from_request(cols, rows);
    session
        .master
        .resize(size)
        .map_err(|error| format!("Could not resize terminal: {error}"))?;
    session.info.cols = size.cols;
    session.info.rows = size.rows;
    Ok(true)
}

pub fn close_terminal_session(
    registry: &TerminalRegistry,
    session_id: &str,
) -> Result<bool, String> {
    let Some(session_id) = normalize_terminal_session_id(session_id) else {
        return Ok(false);
    };
    registry.kill_and_remove(session_id)
}

impl TerminalRegistry {
    fn insert(&self, session_id: String, handle: TerminalSessionHandle) -> Result<(), String> {
        let mut sessions = self
            .inner
            .lock()
            .map_err(|_| "Terminal session registry is unavailable".to_string())?;
        sessions.insert(session_id, handle);
        Ok(())
    }

    /// Kill the child, THEN unregister it — both under one registry lock.
    ///
    /// The ordering is the point. Removing first and killing after meant a failed
    /// kill returned `Err` on a session that no longer existed: the child was
    /// still running but its id was gone, so nothing could list it, close it
    /// again, or reap it. Killing first means the only way a handle leaves the
    /// registry is a kill that actually succeeded (or a tombstone, which has no
    /// child left to kill — see `spawn_terminal_waiter`). On failure the caller
    /// gets the `Err` AND the session stays closable.
    fn kill_and_remove(&self, session_id: &str) -> Result<bool, String> {
        let mut sessions = self
            .inner
            .lock()
            .map_err(|_| "Terminal session registry is unavailable".to_string())?;
        let Some(handle) = sessions.get(session_id) else {
            return Ok(false);
        };
        if !handle.info.exited {
            let mut killer = handle
                .killer
                .lock()
                .map_err(|_| "Terminal killer lock poisoned".to_string())?;
            if let Err(error) = killer.kill() {
                return Err(format!("Failed to kill terminal session: {error}"));
            }
        }
        sessions.remove(session_id);
        Ok(true)
    }

    fn mark_exited(
        &self,
        session_id: &str,
        exit_code: Option<u32>,
        signal: Option<String>,
    ) -> Result<bool, String> {
        let mut sessions = self
            .inner
            .lock()
            .map_err(|_| "Terminal registry lock poisoned".to_string())?;
        match sessions.get_mut(session_id) {
            Some(handle) => {
                handle.info.exited = true;
                handle.info.exit_code = exit_code;
                handle.info.signal = signal;
                Ok(true)
            }
            None => Ok(false),
        }
    }
}

fn spawn_terminal_reader<R: Runtime>(
    app: tauri::AppHandle<R>,
    session_id: String,
    scrollback: Arc<Mutex<String>>,
    mut reader: Box<dyn Read + Send>,
) {
    std::thread::spawn(move || {
        let mut buffer = [0_u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(read_count) => {
                    let data = String::from_utf8_lossy(&buffer[..read_count]).to_string();
                    if let Ok(mut stored_scrollback) = scrollback.lock() {
                        append_terminal_scrollback(&mut stored_scrollback, &data);
                    }
                    let _ = app.emit(
                        TERMINAL_OUTPUT_EVENT,
                        TerminalOutputEvent {
                            session_id: session_id.clone(),
                            data,
                            terminated: false,
                            exit_code: None,
                            signal: None,
                        },
                    );
                }
                Err(_) => break,
            }
        }
    });
}

fn append_terminal_scrollback(scrollback: &mut String, data: &str) {
    if data.is_empty() {
        return;
    }

    scrollback.push_str(data);
    if scrollback.len() <= TERMINAL_SCROLLBACK_MAX_BYTES {
        return;
    }

    // Jump STRAIGHT to the target byte index and nudge forward at most 3 bytes
    // onto a UTF-8 char boundary. The old code walked `char_indices()` from the
    // front of the whole buffer on every over-cap append — O(cap) per 8 KB read,
    // which at a 16 MB cap is not survivable. `is_char_boundary` is O(1), and
    // nudging forward can only ever shorten the result, so the post-trim length
    // is always <= TERMINAL_SCROLLBACK_TRIM_TO_BYTES.
    let mut drain_end = scrollback.len() - TERMINAL_SCROLLBACK_TRIM_TO_BYTES;
    while !scrollback.is_char_boundary(drain_end) {
        drain_end += 1;
    }
    scrollback.drain(..drain_end);
}

fn spawn_terminal_waiter<R: Runtime>(
    app: tauri::AppHandle<R>,
    registry: TerminalRegistry,
    session_id: String,
    mut child: Box<dyn portable_pty::Child + Send + Sync>,
) {
    std::thread::spawn(move || {
        let status = child.wait().ok();
        let exit_code = status.as_ref().map(|value| value.exit_code());
        let signal = status
            .as_ref()
            .and_then(|value| value.signal().map(ToString::to_string));
        // Keep the session as a tombstone: the rail shows "finished — read final
        // output", and the scrollback stays readable. Only an explicit close purges it.
        let _ = registry.mark_exited(&session_id, exit_code, signal.clone());
        let _ = app.emit(
            TERMINAL_OUTPUT_EVENT,
            TerminalOutputEvent {
                session_id,
                data: String::new(),
                terminated: true,
                exit_code,
                signal,
            },
        );
    });
}

fn terminal_size_from_request(cols: Option<u16>, rows: Option<u16>) -> PtySize {
    PtySize {
        cols: cols.unwrap_or(96).clamp(20, 300),
        rows: rows.unwrap_or(28).clamp(4, 100),
        pixel_width: 0,
        pixel_height: 0,
    }
}

/// A command made of nothing but spaces is the same as asking for no command, so the
/// session falls back to the ordinary interactive shell instead of running an empty line.
fn terminal_command_from_request(command: Option<String>) -> Option<String> {
    command
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

/// The flags the shell is started with.
///
/// Every session asks for a LOGIN shell so the embedded terminal loads the user's full
/// environment (PATH from ~/.zprofile, /etc/zprofile path_helper, and Homebrew/pnpm/nvm/
/// cargo shims) exactly like Terminal.app / iTerm / Warp. A plain PTY shell is interactive
/// but NOT a login shell, so login-only PATH entries are missing and agent CLIs
/// (codex/claude/gemini) fail with "command not found" when a conversation tries to resume.
/// Only shells known to accept `-l` are given it.
///
/// With a command to run, `-c <command>` is added: the shell runs that one line and exits,
/// which is how a stack run gets a real exit code instead of a prompt sitting there.
fn terminal_shell_args(shell: &str, run_command: Option<&str>) -> Vec<String> {
    let mut args = Vec::new();

    let is_login_capable = std::path::Path::new(shell)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|shell_name| {
            matches!(shell_name, "zsh" | "bash" | "sh" | "dash" | "ksh" | "fish")
        });
    if is_login_capable {
        args.push("-l".to_string());
    }

    if let Some(run_command) = run_command {
        args.push("-c".to_string());
        args.push(run_command.to_string());
    }

    args
}

fn terminal_shell_from_request(shell: Option<String>) -> String {
    shell
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(default_terminal_shell)
}

fn terminal_cwd_from_request(cwd: &str) -> Result<PathBuf, String> {
    let path = if cwd.trim().is_empty() {
        std::env::current_dir()
            .map_err(|error| format!("Could not read current directory: {error}"))?
    } else {
        PathBuf::from(cwd.trim())
    };
    let metadata = std::fs::metadata(&path)
        .map_err(|error| format!("Could not read terminal cwd metadata: {error}"))?;
    if !metadata.is_dir() {
        return Err("Terminal cwd is not a directory".to_string());
    }
    std::fs::canonicalize(&path)
        .map_err(|error| format!("Could not canonicalize terminal cwd: {error}"))
}

fn normalize_terminal_session_id(session_id: &str) -> Option<&str> {
    let session_id = session_id.trim();
    if session_id.is_empty() {
        None
    } else {
        Some(session_id)
    }
}

fn default_terminal_shell() -> String {
    std::env::var("SHELL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            #[cfg(windows)]
            {
                "cmd.exe".to_string()
            }
            #[cfg(not(windows))]
            {
                "/bin/zsh".to_string()
            }
        })
}

static TERMINAL_SESSION_COUNTER: AtomicU64 = AtomicU64::new(0);

fn new_terminal_session_id() -> String {
    let seq = TERMINAL_SESSION_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("term-{}-{}-{seq}", std::process::id(), timestamp_millis())
}

fn timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_identity_terminal_uses_assembly() {
        assert_eq!(crate::product_identity::TERM_PROGRAM, "Assembly");
    }
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn terminal_without_a_command_still_starts_an_interactive_login_shell() {
        assert_eq!(
            terminal_shell_args("/bin/zsh", None),
            vec!["-l".to_string()]
        );
        assert_eq!(
            terminal_shell_args("/bin/bash", None),
            vec!["-l".to_string()]
        );
        // A shell we do not recognise is started exactly as it was before: no extra flags.
        assert!(terminal_shell_args("/usr/local/bin/nu", None).is_empty());
    }

    #[test]
    fn terminal_with_a_command_runs_it_in_a_login_shell_and_then_exits() {
        assert_eq!(
            terminal_shell_args("/bin/zsh", Some("pnpm test")),
            vec!["-l".to_string(), "-c".to_string(), "pnpm test".to_string()]
        );
        // An unrecognised shell still gets the command, just without the login flag.
        assert_eq!(
            terminal_shell_args("/usr/local/bin/nu", Some("pnpm test")),
            vec!["-c".to_string(), "pnpm test".to_string()]
        );
    }

    #[test]
    fn terminal_treats_a_blank_command_as_no_command_at_all() {
        assert_eq!(terminal_command_from_request(None), None);
        assert_eq!(terminal_command_from_request(Some("   ".to_string())), None);
        assert_eq!(
            terminal_command_from_request(Some("  pnpm test  ".to_string())),
            Some("pnpm test".to_string())
        );
    }

    #[test]
    fn terminal_size_clamps_to_safe_bounds() {
        let low = terminal_size_from_request(Some(1), Some(1));
        assert_eq!(low.cols, 20);
        assert_eq!(low.rows, 4);

        let high = terminal_size_from_request(Some(999), Some(999));
        assert_eq!(high.cols, 300);
        assert_eq!(high.rows, 100);
    }

    #[test]
    fn terminal_shell_uses_request_or_default() {
        assert_eq!(
            terminal_shell_from_request(Some("  /bin/zsh  ".to_string())),
            "/bin/zsh"
        );
        assert!(!terminal_shell_from_request(Some("   ".to_string())).is_empty());
        assert!(!terminal_shell_from_request(None).is_empty());
    }

    #[test]
    fn terminal_scrollback_stays_bounded_and_utf8_safe() {
        let mut scrollback = String::new();
        append_terminal_scrollback(&mut scrollback, "hello");
        append_terminal_scrollback(&mut scrollback, " 世界");

        assert_eq!(scrollback, "hello 世界", "nothing is trimmed below the cap");

        append_terminal_scrollback(
            &mut scrollback,
            &"x".repeat(TERMINAL_SCROLLBACK_MAX_BYTES + 1024),
        );

        // Hysteresis: one over-cap append drains all the way down to the 75%
        // floor, so the next few MB of output cost no drain at all.
        assert!(scrollback.len() <= TERMINAL_SCROLLBACK_TRIM_TO_BYTES);
        assert!(scrollback.is_char_boundary(0));
        assert!(scrollback.ends_with('x'));
    }

    #[test]
    fn terminal_scrollback_never_trims_below_the_cap() {
        let mut scrollback = String::new();
        append_terminal_scrollback(
            &mut scrollback,
            &"y".repeat(TERMINAL_SCROLLBACK_MAX_BYTES - 8),
        );
        append_terminal_scrollback(&mut scrollback, "12345678");
        assert_eq!(
            scrollback.len(),
            TERMINAL_SCROLLBACK_MAX_BYTES,
            "landing exactly ON the cap is not an over-cap append"
        );

        append_terminal_scrollback(&mut scrollback, "9");
        assert!(scrollback.len() <= TERMINAL_SCROLLBACK_TRIM_TO_BYTES);
        assert!(
            scrollback.ends_with("123456789"),
            "the TAIL is what survives"
        );
    }

    #[test]
    fn terminal_scrollback_trim_lands_on_a_char_boundary() {
        // Every byte of the buffer is inside a 3-byte char, and the 1..=3 byte
        // tail shifts the raw drain index through all three residues mod 3 — so
        // one of these iterations targets a byte that is NOT a char boundary.
        // `String::drain` panics on a non-boundary, and `starts_with('世')`
        // catches a boundary that is merely valid but wrong.
        for tail_bytes in 1..=3 {
            let mut scrollback = "世".repeat(TERMINAL_SCROLLBACK_MAX_BYTES / 3 + 16);
            append_terminal_scrollback(&mut scrollback, &"a".repeat(tail_bytes));

            assert!(scrollback.len() <= TERMINAL_SCROLLBACK_TRIM_TO_BYTES);
            assert!(scrollback.starts_with('世'), "trim split a multibyte char");
            assert!(scrollback.ends_with('a'));
        }
    }

    #[test]
    fn terminal_cwd_trims_canonicalizes_and_rejects_invalid_paths() {
        let root = unique_terminal_test_root();
        let nested = root.join("workspace");
        std::fs::create_dir_all(&nested).unwrap();
        let file_path = root.join("not-a-directory.txt");
        std::fs::write(&file_path, "not a directory").unwrap();

        let requested = format!("  {}/../workspace/  ", nested.display());
        let resolved = terminal_cwd_from_request(&requested).unwrap();
        assert_eq!(resolved, std::fs::canonicalize(&nested).unwrap());

        let file_error = terminal_cwd_from_request(&file_path.display().to_string()).unwrap_err();
        assert!(file_error.contains("Terminal cwd is not a directory"));

        let missing_error =
            terminal_cwd_from_request(&root.join("missing").display().to_string()).unwrap_err();
        assert!(missing_error.contains("Could not read terminal cwd metadata"));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_terminal_session_ids_are_noops() {
        let registry = TerminalRegistry::default();

        assert_eq!(
            read_terminal_session_scrollback(&registry, "missing-terminal").unwrap(),
            None
        );
        assert!(!write_terminal_session(&registry, "missing-terminal", "echo nope\n").unwrap());
        assert!(
            !resize_terminal_session(&registry, "missing-terminal", Some(100), Some(32)).unwrap()
        );
        assert!(!close_terminal_session(&registry, "missing-terminal").unwrap());

        assert_eq!(
            read_terminal_session_scrollback(&registry, "   ").unwrap(),
            None
        );
        assert!(!write_terminal_session(&registry, "   ", "echo nope\n").unwrap());
        assert!(!resize_terminal_session(&registry, "   ", Some(100), Some(32)).unwrap());
        assert!(!close_terminal_session(&registry, "   ").unwrap());
    }

    #[test]
    fn agent_tool_terminals_have_distinct_typed_identities() {
        let first = ToolTerminalIdentity {
            owned_id: "owned-a".into(),
            turn_id: "turn-a".into(),
            tool_call_id: "tool-a".into(),
            terminal_id: "terminal-a".into(),
        };
        let second = ToolTerminalIdentity {
            owned_id: "owned-a".into(),
            turn_id: "turn-a".into(),
            tool_call_id: "tool-b".into(),
            terminal_id: "terminal-b".into(),
        };
        assert_ne!(first, second);
        assert_eq!(TerminalKind::AgentTool, TerminalKind::AgentTool);
    }

    #[cfg(not(windows))]
    #[test]
    fn terminal_session_lifecycle_accepts_padded_ids_writes_resizes_reads_and_closes_native_pty() {
        let app = tauri::test::mock_app();
        let registry = TerminalRegistry::default();
        let expected_cwd = std::fs::canonicalize(std::env::temp_dir())
            .expect("temp dir should canonicalize")
            .display()
            .to_string();
        let session = start_terminal_session(
            app.handle().clone(),
            &registry,
            TerminalStartRequest {
                cwd: std::env::temp_dir().display().to_string(),
                shell: Some("/bin/sh".to_string()),
                cols: Some(80),
                rows: Some(20),
                owned_id: None,
                command: None,
            },
        )
        .expect("terminal session should start");
        let copied_session_id = format!("  {}\n", session.session_id);

        let result = (|| {
            assert_eq!(session.cwd, expected_cwd);
            assert_eq!(session.shell, "/bin/sh");
            assert_eq!(session.cols, 80);
            assert_eq!(session.rows, 20);
            assert!(session.pid.is_some());
            assert!(list_terminal_sessions(&registry)
                .expect("terminal sessions should list")
                .iter()
                .any(|listed| listed.session_id == session.session_id));

            assert!(write_terminal_session(
                &registry,
                &copied_session_id,
                "printf 'mcb-terminal-ready\\n'\n"
            )
            .expect("terminal input should write"));
            assert!(
                resize_terminal_session(&registry, &copied_session_id, Some(100), Some(32))
                    .expect("terminal session should resize")
            );
            let resized_session = list_terminal_sessions(&registry)
                .expect("terminal sessions should list after resize")
                .into_iter()
                .find(|listed| listed.session_id == session.session_id)
                .expect("resized terminal session should still be listed");
            assert_eq!(resized_session.cols, 100);
            assert_eq!(resized_session.rows, 32);

            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                let scrollback = read_terminal_session_scrollback(&registry, &copied_session_id)
                    .expect("terminal scrollback should read")
                    .unwrap_or_default();
                if scrollback.contains("mcb-terminal-ready") {
                    break Ok(());
                }
                if Instant::now() >= deadline {
                    break Err(format!(
                        "terminal scrollback did not receive sentinel; got {scrollback:?}"
                    ));
                }
                thread::sleep(Duration::from_millis(25));
            }
        })();

        let closed = close_terminal_session(&registry, &copied_session_id)
            .expect("terminal session should close");
        assert!(closed);
        assert!(
            list_terminal_sessions(&registry)
                .expect("terminal sessions should list after close")
                .is_empty(),
            "closed terminal session should be removed from the registry"
        );

        result.expect("terminal session lifecycle should complete");
    }

    fn unique_terminal_test_root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "mcb-terminal-test-{}-{}",
            std::process::id(),
            timestamp_millis()
        ))
    }
}
