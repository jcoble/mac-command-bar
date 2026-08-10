use std::fmt;
use std::io::{self, Write};

macro_rules! stderr_log {
    ($($argument:tt)*) => {{
        $crate::debug_log::write_line(format_args!($($argument)*));
    }};
}

pub(crate) use stderr_log;

pub(crate) fn write_line(arguments: fmt::Arguments<'_>) {
    write_line_to(&mut io::stderr().lock(), arguments);
}

fn write_line_to(writer: &mut dyn Write, arguments: fmt::Arguments<'_>) {
    let _ = writeln!(writer, "{arguments}");
}

#[cfg(test)]
mod tests {
    use super::write_line_to;
    use std::fs::File;
    use std::io::Write;
    use std::os::fd::FromRawFd;
    use std::panic::{catch_unwind, AssertUnwindSafe};

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
}
