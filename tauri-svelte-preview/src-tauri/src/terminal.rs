use portable_pty::{native_pty_system, ChildKiller, CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{Emitter, Runtime};

pub const TERMINAL_OUTPUT_EVENT: &str = "terminal_output";
const TERMINAL_SCROLLBACK_MAX_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalStartRequest {
    pub cwd: String,
    pub shell: Option<String>,
    pub cols: Option<u16>,
    pub rows: Option<u16>,
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
    let cwd = terminal_cwd_from_request(&request.cwd)?;
    let shell = terminal_shell_from_request(request.shell);
    let size = terminal_size_from_request(request.cols, request.rows);
    let session_id = new_terminal_session_id();
    let started_at = timestamp_millis();

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(size)
        .map_err(|error| format!("Could not open terminal pty: {error}"))?;
    let mut command = CommandBuilder::new(&shell);
    command.cwd(&cwd);
    command.env("TERM", "xterm-256color");
    command.env("COLORTERM", "truecolor");

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
    let sessions = registry
        .inner
        .lock()
        .map_err(|_| "Terminal session registry is unavailable".to_string())?;
    let Some(session) = sessions.get(session_id) else {
        return Ok(false);
    };
    session
        .master
        .resize(terminal_size_from_request(cols, rows))
        .map_err(|error| format!("Could not resize terminal: {error}"))?;
    Ok(true)
}

pub fn close_terminal_session(
    registry: &TerminalRegistry,
    session_id: &str,
) -> Result<bool, String> {
    let Some(session_id) = normalize_terminal_session_id(session_id) else {
        return Ok(false);
    };
    let Some(session) = registry.remove(session_id)? else {
        return Ok(false);
    };
    let mut killer = session
        .killer
        .lock()
        .map_err(|_| "Terminal process killer is unavailable".to_string())?;
    killer
        .kill()
        .map_err(|error| format!("Could not close terminal process: {error}"))?;
    Ok(true)
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

    fn remove(&self, session_id: &str) -> Result<Option<TerminalSessionHandle>, String> {
        let mut sessions = self
            .inner
            .lock()
            .map_err(|_| "Terminal session registry is unavailable".to_string())?;
        Ok(sessions.remove(session_id))
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

    let excess_bytes = scrollback.len() - TERMINAL_SCROLLBACK_MAX_BYTES;
    let trim_index = scrollback
        .char_indices()
        .map(|(index, _)| index)
        .find(|index| *index >= excess_bytes)
        .unwrap_or(scrollback.len());
    scrollback.drain(..trim_index);
}

fn spawn_terminal_waiter<R: Runtime>(
    app: tauri::AppHandle<R>,
    registry: TerminalRegistry,
    session_id: String,
    mut child: Box<dyn portable_pty::Child + Send + Sync>,
) {
    std::thread::spawn(move || {
        let status = child.wait().ok();
        let _ = registry.remove(&session_id);
        let _ = app.emit(
            TERMINAL_OUTPUT_EVENT,
            TerminalOutputEvent {
                session_id,
                data: String::new(),
                terminated: true,
                exit_code: status.as_ref().map(|value| value.exit_code()),
                signal: status
                    .as_ref()
                    .and_then(|value| value.signal().map(ToString::to_string)),
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

fn new_terminal_session_id() -> String {
    format!("term-{}-{}", std::process::id(), timestamp_millis())
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
    use std::thread;
    use std::time::{Duration, Instant};

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

        assert!(scrollback.ends_with(" 世界"));
        assert!(scrollback.len() <= TERMINAL_SCROLLBACK_MAX_BYTES);

        append_terminal_scrollback(
            &mut scrollback,
            &"x".repeat(TERMINAL_SCROLLBACK_MAX_BYTES + 1024),
        );

        assert!(scrollback.len() <= TERMINAL_SCROLLBACK_MAX_BYTES);
        assert!(scrollback.is_char_boundary(0));
        assert!(scrollback.ends_with('x'));
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
        assert!(!resize_terminal_session(&registry, "missing-terminal", Some(100), Some(32))
            .unwrap());
        assert!(!close_terminal_session(&registry, "missing-terminal").unwrap());

        assert_eq!(read_terminal_session_scrollback(&registry, "   ").unwrap(), None);
        assert!(!write_terminal_session(&registry, "   ", "echo nope\n").unwrap());
        assert!(!resize_terminal_session(&registry, "   ", Some(100), Some(32)).unwrap());
        assert!(!close_terminal_session(&registry, "   ").unwrap());
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
