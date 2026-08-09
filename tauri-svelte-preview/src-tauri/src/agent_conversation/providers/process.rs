use std::collections::VecDeque;
use std::mem::ManuallyDrop;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::ptr;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use super::super::protocol::AgentProviderManifest;

const STDERR_LINE_CAP: usize = 200;

pub fn validated_conversation_cwd(value: &str) -> Result<PathBuf, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("Conversation project folder is required".to_string());
    }
    let path = PathBuf::from(value)
        .canonicalize()
        .map_err(|_| "Conversation project folder does not exist".to_string())?;
    if !path.is_dir() {
        return Err("Conversation project folder is not a directory".to_string());
    }
    Ok(path)
}

#[derive(Clone, Debug)]
pub struct BoundedStderr {
    capacity: usize,
    lines: Arc<Mutex<VecDeque<String>>>,
}

impl BoundedStderr {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            lines: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub fn push(&self, line: String) {
        if let Ok(mut lines) = self.lines.lock() {
            lines.push_back(line);
            while lines.len() > self.capacity {
                lines.pop_front();
            }
        }
    }

    pub fn snapshot(&self) -> Vec<String> {
        self.lines
            .lock()
            .map(|lines| lines.iter().cloned().collect())
            .unwrap_or_default()
    }
}

pub struct SidecarProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: Lines<BufReader<ChildStdout>>,
    stderr: BoundedStderr,
}

pub struct SidecarReadHalf {
    stdout: Lines<BufReader<ChildStdout>>,
    stderr: BoundedStderr,
}

pub struct SidecarWriteHalf {
    stdin: ChildStdin,
}

pub struct SidecarProcessHandle {
    child: Child,
    stderr: BoundedStderr,
}

impl SidecarProcess {
    pub fn spawn(
        manifest: &AgentProviderManifest,
        cwd: &Path,
        owned_id: &str,
    ) -> Result<Self, String> {
        let mut command = Command::new(&manifest.executable);
        command
            .args(&manifest.args)
            .current_dir(cwd)
            .env("COMMANDBAR_SESSION_ID", owned_id)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        command.as_std_mut().process_group(0);
        let mut child = command
            .spawn()
            .map_err(|error| format!("Could not start {}: {error}", manifest.display_name))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "ACP sidecar did not provide stdin".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "ACP sidecar did not provide stdout".to_string())?;
        let stderr_stream = child
            .stderr
            .take()
            .ok_or_else(|| "ACP sidecar did not provide stderr".to_string())?;
        let stderr = BoundedStderr::new(STDERR_LINE_CAP);
        let stderr_capture = stderr.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr_stream).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                stderr_capture.push(line);
            }
        });
        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout).lines(),
            stderr,
        })
    }

    pub async fn write_json(&mut self, value: &serde_json::Value) -> Result<(), String> {
        let mut encoded = serde_json::to_vec(value).map_err(|error| error.to_string())?;
        encoded.push(b'\n');
        self.stdin
            .write_all(&encoded)
            .await
            .map_err(|error| format!("Could not write ACP request: {error}"))?;
        self.stdin
            .flush()
            .await
            .map_err(|error| format!("Could not flush ACP request: {error}"))
    }

    pub async fn next_json(&mut self) -> Result<serde_json::Value, String> {
        loop {
            let line = self
                .stdout
                .next_line()
                .await
                .map_err(|error| format!("Could not read ACP response: {error}"))?
                .ok_or_else(|| {
                    format!("ACP sidecar exited: {}", self.stderr.snapshot().join(" | "))
                })?;
            if let Ok(value) = serde_json::from_str(&line) {
                return Ok(value);
            }
        }
    }

    pub fn split(self) -> (SidecarReadHalf, SidecarWriteHalf, SidecarProcessHandle) {
        let mut process = ManuallyDrop::new(self);
        // The process is deliberately transferred to independent transport
        // owners; ManuallyDrop prevents the original Drop impl from killing it
        // while these fields are moved out.
        let child = unsafe { ptr::read(&mut process.child) };
        let stdin = unsafe { ptr::read(&mut process.stdin) };
        let stdout = unsafe { ptr::read(&mut process.stdout) };
        let stderr = unsafe { ptr::read(&mut process.stderr) };
        (
            SidecarReadHalf {
                stdout,
                stderr: stderr.clone(),
            },
            SidecarWriteHalf { stdin },
            SidecarProcessHandle { child, stderr },
        )
    }

    pub fn stderr_snapshot(&self) -> Vec<String> {
        self.stderr.snapshot()
    }

    pub async fn stop(&mut self) {
        stop_process_group(&mut self.child);
        let _ = self.child.wait().await;
    }

    pub fn process_id(&self) -> Option<u32> {
        self.child.id()
    }
}

impl SidecarWriteHalf {
    pub async fn write_json(&mut self, value: &serde_json::Value) -> Result<(), String> {
        let mut encoded = serde_json::to_vec(value).map_err(|error| error.to_string())?;
        encoded.push(b'\n');
        self.stdin
            .write_all(&encoded)
            .await
            .map_err(|error| format!("Could not write ACP request: {error}"))?;
        self.stdin
            .flush()
            .await
            .map_err(|error| format!("Could not flush ACP request: {error}"))
    }
}

impl SidecarReadHalf {
    pub async fn next_json(&mut self) -> Result<serde_json::Value, String> {
        loop {
            let line = self
                .stdout
                .next_line()
                .await
                .map_err(|error| format!("Could not read ACP response: {error}"))?
                .ok_or_else(|| {
                    format!("ACP sidecar exited: {}", self.stderr.snapshot().join(" | "))
                })?;
            if let Ok(value) = serde_json::from_str(&line) {
                return Ok(value);
            }
        }
    }
}

impl SidecarProcessHandle {
    pub async fn stop(&mut self) {
        stop_process_group(&mut self.child);
        let _ = self.child.wait().await;
    }

    pub fn stderr_snapshot(&self) -> Vec<String> {
        self.stderr.snapshot()
    }

    pub fn process_id(&self) -> Option<u32> {
        self.child.id()
    }
}

impl Drop for SidecarProcessHandle {
    fn drop(&mut self) {
        stop_process_group(&mut self.child);
    }
}

impl Drop for SidecarProcess {
    fn drop(&mut self) {
        stop_process_group(&mut self.child);
    }
}

fn stop_process_group(child: &mut Child) {
    let Some(pid) = child.id() else {
        return;
    };
    let target = -(pid as i32);
    // The child is placed in a fresh group during spawn, so this target cannot
    // include the app or another managed sidecar.
    if unsafe { libc::kill(target, libc::SIGKILL) } != 0 {
        let _ = child.start_kill();
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    use super::*;
    use crate::agent_conversation::protocol::{
        AgentProviderManifest, ProviderSource, ProviderTransport,
    };

    #[test]
    fn stderr_diagnostics_are_bounded() {
        let stderr = BoundedStderr::new(2);
        stderr.push("one".into());
        stderr.push("two".into());
        stderr.push("three".into());
        assert_eq!(stderr.snapshot(), vec!["two", "three"]);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn sidecar_close_kills_the_entire_process_group() {
        let manifest = AgentProviderManifest {
            id: "process-tree-fixture".into(),
            display_name: "Process tree fixture".into(),
            transport: ProviderTransport::AcpStdio,
            executable: PathBuf::from("/bin/sh"),
            args: vec![
                "-c".into(),
                "sleep 30 & child=$!; echo $child >&2; wait".into(),
            ],
            version: "1".into(),
            content_hash: "0000000000000000000000000000000000000000000000000000000000000000".into(),
            trusted_source: ProviderSource::Bundled,
        };
        let mut process =
            SidecarProcess::spawn(&manifest, &std::env::temp_dir(), "owned-process-tree").unwrap();
        let parent = process.process_id().unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        let child = loop {
            if let Some(child) = process
                .stderr_snapshot()
                .first()
                .and_then(|line| line.parse::<i32>().ok())
            {
                break child;
            }
            assert!(
                Instant::now() < deadline,
                "fixture child pid was not captured"
            );
            tokio::time::sleep(Duration::from_millis(10)).await;
        };
        process.stop().await;
        assert_ne!(
            unsafe { libc::kill(parent as i32, 0) },
            0,
            "sidecar parent survived close"
        );
        assert_ne!(
            unsafe { libc::kill(child, 0) },
            0,
            "sidecar descendant survived close"
        );
    }
}
