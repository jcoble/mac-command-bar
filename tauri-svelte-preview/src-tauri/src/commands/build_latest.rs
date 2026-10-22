//! Build latest (dev): runs `scripts/buildLatestDev.ts` from the checkout this dev app was built
//! from, sends its lines to the UI as `build_latest_dev` app events, and quits the app once the
//! script has handed off the restart. App events rather than a channel, so the page Vite reloads
//! while `git pull` brings in new frontend files still receives them.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tauri::{Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::agent_conversation::remote::{validate_ssh_target, RemoteConnectionManager};

const EVENT: &str = "build_latest_dev";
/// One run at a time. Stays set after a successful run: the app is quitting and the restart
/// is already waiting for it.
static RUNNING: AtomicBool = AtomicBool::new(false);

fn build_latest_process(preview_root: &Path, app_pid: u32, log: &Path, hosts: &[String])
    -> Result<(&'static str, Vec<String>), String> {
    for host in hosts {
        validate_ssh_target(host)?;
    }
    let quoted_root = format!("'{}'", preview_root.display().to_string().replace('\'', "'\\''"));
    let mut args = vec![
        "-lic".to_string(),
        format!("cd {quoted_root} && exec node --experimental-strip-types scripts/buildLatestDev.ts \"$@\""),
        "zsh".into(), "--pid".into(), app_pid.to_string(), "--log".into(), log.display().to_string(),
    ];
    args.extend(hosts.iter().cloned());
    Ok(("/bin/zsh", args))
}

/// Starts the run and returns the log path; progress arrives as app events.
#[tauri::command]
pub(crate) async fn build_latest_dev(
    app: tauri::AppHandle,
    remote: tauri::State<'_, RemoteConnectionManager>,
) -> Result<String, String> {
    let hosts = remote.environment().profiles.into_iter().map(|profile| profile.ssh_target).collect::<Vec<_>>();
    let log_dir = app.path().app_log_dir().map_err(|error| format!("Could not find the log folder: {error}"))?;
    std::fs::create_dir_all(&log_dir).map_err(|error| format!("Could not create the log folder: {error}"))?;
    let log = log_dir.join("build-latest.log");
    let preview_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent()
        .ok_or_else(|| "Could not resolve the development checkout".to_string())?.to_path_buf();
    let (program, args) = build_latest_process(&preview_root, std::process::id(), &log, &hosts)?;
    if RUNNING.swap(true, Ordering::AcqRel) {
        return Err("Build latest is already running.".into());
    }
    // Its own process group: quitting the app, or tauri dev stopping its children, leaves it running.
    let child = tokio::process::Command::new(program).args(args)
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped())
        .process_group(0).spawn();
    let mut child = match child {
        Ok(child) => child,
        Err(error) => {
            RUNNING.store(false, Ordering::Release);
            return Err(format!("Could not start Build latest: {error}"));
        }
    };
    tauri::async_runtime::spawn(async move {
        let error = follow(&app, &mut child).await.err();
        let _ = app.emit(EVENT, serde_json::json!({ "done": true, "error": error }));
        if error.is_none() {
            // The normal quit: the workspace is saved, and running local work is asked about first.
            app.exit(0);
        } else {
            RUNNING.store(false, Ordering::Release);
        }
    });
    Ok(log.display().to_string())
}

async fn follow(app: &tauri::AppHandle, child: &mut tokio::process::Child) -> Result<(), String> {
    let process_group = child.id().map(|pid| -(pid as i32));
    let mut stdout = BufReader::new(child.stdout.take().expect("stdout is piped")).lines();
    let mut stderr = BufReader::new(child.stderr.take().expect("stderr is piped")).lines();
    let operation = async {
        let (mut stdout_done, mut stderr_done, mut last) = (false, false, String::new());
        // A stream that ends or cannot be read is done; wait() reports how the run ended.
        while !stdout_done || !stderr_done {
            let line = tokio::select! {
                line = stdout.next_line(), if !stdout_done => {
                    let line = line.ok().flatten();
                    stdout_done = line.is_none();
                    line
                }
                line = stderr.next_line(), if !stderr_done => {
                    let line = line.ok().flatten();
                    stderr_done = line.is_none();
                    line
                }
            };
            if let Some(line) = line {
                let _ = app.emit(EVENT, serde_json::json!({ "line": line }));
                last = line;
            }
        }
        let status = child.wait().await.map_err(|error| format!("Could not wait for Build latest: {error}"))?;
        if status.success() { Ok(()) } else if last.is_empty() { Err(format!("Build latest failed with {status}")) } else { Err(last) }
    };
    match tokio::time::timeout(Duration::from_secs(60 * 60), operation).await {
        Ok(result) => result,
        Err(_) => {
            if let Some(process_group) = process_group {
                // SAFETY: `process_group(0)` made the child's PID its process-group ID.
                unsafe { libc::kill(process_group, libc::SIGKILL) };
            }
            Err("Build latest timed out after 60 minutes".into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_latest_passes_the_pid_log_and_hosts_as_positional_arguments() {
        let (program, args) = build_latest_process(Path::new("/work/it's/tauri-svelte-preview"), 4242,
            Path::new("/logs/build-latest.log"), &["box-a".into(), "person@box-b".into()]).unwrap();
        assert_eq!(program, "/bin/zsh");
        assert_eq!(args, vec![
            "-lic".to_string(),
            "cd '/work/it'\\''s/tauri-svelte-preview' && exec node --experimental-strip-types scripts/buildLatestDev.ts \"$@\"".to_string(),
            "zsh".into(), "--pid".into(), "4242".into(), "--log".into(), "/logs/build-latest.log".into(),
            "box-a".into(), "person@box-b".into(),
        ]);
    }

    #[test]
    fn build_latest_rejects_a_host_that_could_change_the_command() {
        for host in ["box; reboot", "--log", "$(id)"] {
            assert!(build_latest_process(Path::new("/work"), 1, Path::new("/log"), &[host.into()]).is_err(), "{host}");
        }
    }
}
