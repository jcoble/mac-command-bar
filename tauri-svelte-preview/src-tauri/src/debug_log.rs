use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;

static FILE_MIRROR: OnceLock<Mutex<File>> = OnceLock::new();

macro_rules! stderr_log {
    ($($argument:tt)*) => {{
        $crate::debug_log::write_line(format_args!($($argument)*));
    }};
}

pub(crate) use stderr_log;

pub(crate) fn install_file_mirror(path: PathBuf) {
    let Ok(file) = open_file_mirror(&path) else {
        return;
    };
    let mirror = Mutex::new(file);
    if let Err(mirror) = FILE_MIRROR.set(mirror) {
        #[cfg(test)]
        if let (Some(installed), Ok(replacement)) = (FILE_MIRROR.get(), mirror.into_inner()) {
            *installed
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = replacement;
        }
        #[cfg(not(test))]
        let _ = mirror;
    }
}

pub(crate) fn write_line(arguments: fmt::Arguments<'_>) {
    write_line_to(&mut io::stderr().lock(), arguments);
    if let Some(mirror) = FILE_MIRROR.get() {
        write_line_to(
            &mut *mirror
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
            arguments,
        );
    }
}

fn write_line_to(writer: &mut dyn Write, arguments: fmt::Arguments<'_>) {
    let _ = writeln!(writer, "{arguments}");
}

fn open_file_mirror(path: &PathBuf) -> io::Result<File> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    if fs::metadata(path).is_ok_and(|metadata| metadata.len() > MAX_LOG_BYTES) {
        let mut rotated = path.as_os_str().to_owned();
        rotated.push(".1");
        let rotated = PathBuf::from(rotated);
        match fs::remove_file(&rotated) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        fs::rename(path, rotated)?;
    }
    OpenOptions::new().create(true).append(true).open(path)
}

/// The memory hunt's switches (TSK-1299), read from the environment the app was started with:
/// `ASSEMBLY_DEBUG_OFF` names the chat parts to leave out, `ASSEMBLY_DEBUG_SWITCH_SESSIONS` the chats
/// to switch between. A `mark` goes to the backend log, where the measuring script waits for it.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DebugOff {
    off: String,
    switch_sessions: String,
}

#[tauri::command]
pub(crate) fn debug_off(mark: Option<String>) -> DebugOff {
    if let Some(mark) = mark {
        stderr_log!("assembly-debug: {mark}");
    }
    let read = |name| std::env::var(name).unwrap_or_default();
    DebugOff {
        off: read("ASSEMBLY_DEBUG_OFF"),
        switch_sessions: read("ASSEMBLY_DEBUG_SWITCH_SESSIONS"),
    }
}

#[cfg(test)]
mod tests {
    use super::{install_file_mirror, write_line, write_line_to, MAX_LOG_BYTES};
    use std::fs::{self, File};
    use std::io::Write;
    use std::os::fd::FromRawFd;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::path::PathBuf;
    use std::sync::Mutex;

    static FILE_MIRROR_TEST: Mutex<()> = Mutex::new(());

    fn temp_root() -> PathBuf {
        let root = std::env::temp_dir().join(format!("mcb-backend-log-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn writing_to_a_closed_pipe_does_not_panic() {
        let mut pipe_fds = [0; 2];
        assert_eq!(unsafe { libc::pipe(pipe_fds.as_mut_ptr()) }, 0);
        assert_eq!(unsafe { libc::close(pipe_fds[0]) }, 0);
        let mut closed_pipe = unsafe { File::from_raw_fd(pipe_fds[1]) };

        let error = closed_pipe
            .write_all(b"confirm closed pipe")
            .expect_err("writing to the pipe must fail after its reader closes");
        assert_eq!(error.kind(), std::io::ErrorKind::BrokenPipe);

        let result = catch_unwind(AssertUnwindSafe(|| {
            let returned: () = write_line_to(&mut closed_pipe, format_args!("discarded error"));
            returned
        }));

        assert!(result.is_ok(), "the logging helper must discard EPIPE");
    }

    #[test]
    fn installed_mirror_receives_each_stderr_line() {
        let _serial = FILE_MIRROR_TEST.lock().unwrap();
        let root = temp_root();
        let path = root.join("logs/backend.log");
        install_file_mirror(path.clone());

        write_line(format_args!("mirror receipt"));

        assert_eq!(fs::read_to_string(path).unwrap(), "mirror receipt\n");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn oversized_log_rotates_once_before_opening() {
        let _serial = FILE_MIRROR_TEST.lock().unwrap();
        let root = temp_root();
        let path = root.join("logs/backend.log");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        File::create(&path)
            .unwrap()
            .set_len(MAX_LOG_BYTES + 1)
            .unwrap();
        fs::write(path.with_file_name("backend.log.1"), b"previous rotation").unwrap();

        install_file_mirror(path.clone());

        assert_eq!(
            fs::metadata(path.with_file_name("backend.log.1"))
                .unwrap()
                .len(),
            MAX_LOG_BYTES + 1
        );
        assert!(fs::metadata(&path).unwrap().len() < MAX_LOG_BYTES);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn debug_off_returns_the_memory_hunt_switches_from_the_environment() {
        std::env::set_var("ASSEMBLY_DEBUG_OFF", "tools,markdown");
        std::env::set_var("ASSEMBLY_DEBUG_SWITCH_SESSIONS", "a1,b2");
        let read = super::debug_off(Some("ready".to_string()));
        assert_eq!(read.off, "tools,markdown");
        assert_eq!(read.switch_sessions, "a1,b2");
        std::env::remove_var("ASSEMBLY_DEBUG_OFF");
        std::env::remove_var("ASSEMBLY_DEBUG_SWITCH_SESSIONS");
        let read = super::debug_off(None);
        assert_eq!((read.off.as_str(), read.switch_sessions.as_str()), ("", ""));
    }
}
