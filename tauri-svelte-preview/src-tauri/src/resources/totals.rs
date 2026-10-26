//! The resource totals in the bottom strip. One sampler runs while at least one
//! window is watching, and pushes `resource-totals` only when a figure the strip
//! shows would change; with nobody watching nothing samples.

use std::collections::HashSet;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;
use sysinfo::{Pid, ProcessesToUpdate};
use tauri::{AppHandle, Emitter, Manager, Window, WindowEvent};

use super::{
    build_resource_sample, observe_processes, resource_sample_owners,
    resource_sample_timestamp_millis, ResourceRegistry, ResourceSampleTotals,
    ACTIVE_SOURCE_DIRECTORY_READS,
};

const RESOURCE_TOTALS_EVENT: &str = "resource-totals";
const SAMPLE_INTERVAL: Duration = Duration::from_secs(5);

/// The windows watching the totals, by label, and whether the sampler runs.
/// Labels make watch and unwatch idempotent, and a reloaded page re-adding its
/// label does not count twice.
#[derive(Default)]
pub struct WatchState {
    windows: HashSet<String>,
    destroy_hooked: HashSet<String>,
    running: bool,
}

impl WatchState {
    /// Adds a watcher. Returns whether the window still needs a destroy hook and
    /// whether a sampler must be started.
    fn add(&mut self, label: &str) -> (bool, bool) {
        self.windows.insert(label.to_owned());
        let hook = self.destroy_hooked.insert(label.to_owned());
        let start = !self.running;
        self.running = true;
        (hook, start)
    }

    fn remove(&mut self, label: &str) {
        self.windows.remove(label);
    }

    /// Called by the sampler under the same lock as `add`, so a watch landing
    /// while it decides to stop either keeps it running or starts a new one.
    fn keep_sampling(&mut self) -> bool {
        self.running = !self.windows.is_empty();
        self.running
    }
}

/// What the strip shows, so an unchanged sample is not pushed.
#[derive(PartialEq, Debug)]
struct DisplayKey(String, String, String, usize, usize);

impl DisplayKey {
    fn of(totals: &ResourceSampleTotals) -> Self {
        // The frontend receives the f32's shortest decimal; round that, as it does.
        let cpu: f64 = totals.cpu_percent.to_string().parse().unwrap_or(0.0);
        Self(
            format!("{:.1}", cpu.max(0.0)),
            format_bytes(totals.physical_footprint_bytes),
            format_bytes(totals.rss_bytes),
            totals.process_count,
            totals.active_source_directory_reads,
        )
    }
}

/// `formatResourceBytes` in `resourceSampleViewModel.ts`.
fn format_bytes(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_owned();
    }
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let bytes = bytes as f64;
    let unit = ((bytes.ln() / 1024f64.ln()).floor() as usize).min(UNITS.len() - 1);
    let value = bytes / 1024f64.powi(unit as i32);
    let precision = if unit >= 3 && value < 10.0 { 2 } else if unit >= 2 && value < 100.0 { 1 } else { 0 };
    format!("{value:.precision$} {}", UNITS[unit])
}

/// Refresh only the process ids already attributed to Assembly. A missing or
/// stale cache is rediscovered so the gutter is accurate before the Resource
/// Manager has opened and after an attributed helper restarts.
fn sample(app: &AppHandle) -> Result<ResourceSampleTotals, String> {
    let registry = app.state::<ResourceRegistry>();
    let system = Arc::clone(&registry.sample_system);
    let sampled_process_ids = Arc::clone(&registry.sampled_process_ids);
    let terminal_registry = app.state::<crate::terminal::TerminalRegistry>().inner();
    let agent_runtime = app.state::<crate::agent_conversation::manager::AgentRuntimeManager>().inner();
    let lsp_registry = app.state::<crate::files::lsp::SourceLspRegistry>().inner();

    let owners = resource_sample_owners(terminal_registry, agent_runtime, lsp_registry)?;
    let cached_process_ids = sampled_process_ids
        .lock()
        .map_err(|_| "Resource sampler is unavailable".to_string())?
        .clone();
    let mut process_ids = cached_process_ids.clone();
    process_ids.push(std::process::id());
    process_ids.extend(owners.iter().map(|owner| owner.root_pid));
    process_ids.sort_unstable();
    process_ids.dedup();

    let mut system = system
        .lock()
        .map_err(|_| "Resource sampler is unavailable".to_string())?;
    let sysinfo_pids = process_ids
        .iter()
        .copied()
        .map(Pid::from_u32)
        .collect::<Vec<_>>();
    system.refresh_processes(ProcessesToUpdate::Some(&sysinfo_pids), true);

    if cached_process_ids.is_empty()
        || cached_process_ids
            .iter()
            .any(|pid| system.process(Pid::from_u32(*pid)).is_none())
    {
        let processes = observe_processes(&mut system, std::process::id());
        let sample = build_resource_sample(
            resource_sample_timestamp_millis(),
            std::process::id(),
            &processes,
            &owners,
        );
        process_ids = sample
            .app
            .parts
            .iter()
            .map(|part| part.pid)
            .chain(sample.groups.iter().flat_map(|group| {
                group
                    .sessions
                    .iter()
                    .flat_map(|session| session.processes.iter().map(|process| process.pid))
            }))
            .collect();
        process_ids.sort_unstable();
        process_ids.dedup();
        *sampled_process_ids
            .lock()
            .map_err(|_| "Resource sampler is unavailable".to_string())? = process_ids;
        return Ok(sample.totals);
    }

    let mut totals = ResourceSampleTotals {
        cpu_percent: 0.0,
        physical_footprint_bytes: 0,
        rss_bytes: 0,
        process_count: 0,
        active_source_directory_reads: ACTIVE_SOURCE_DIRECTORY_READS.load(Ordering::Relaxed),
    };
    let mut alive_process_ids = Vec::with_capacity(process_ids.len());
    for pid in process_ids {
        let Some(process) = system.process(Pid::from_u32(pid)) else {
            continue;
        };
        let rss_bytes = process.memory();
        totals.cpu_percent += process.cpu_usage();
        totals.physical_footprint_bytes +=
            mcb_core::scanners::resources::process_footprint_bytes(pid, &process.name().to_string_lossy(), rss_bytes);
        totals.rss_bytes += rss_bytes;
        totals.process_count += 1;
        alive_process_ids.push(pid);
    }
    *sampled_process_ids
        .lock()
        .map_err(|_| "Resource sampler is unavailable".to_string())? = alive_process_ids;
    Ok(totals)
}

async fn sample_off_thread(app: &AppHandle) -> Result<ResourceSampleTotals, String> {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || sample(&app))
        .await
        .map_err(|error| format!("Resource totals task failed: {error}"))?
}

/// Start (`true`) or stop (`false`) this window watching the totals. Starting
/// returns a fresh sample so the strip is never blank while it waits for a push.
#[tauri::command]
pub async fn watch_resource_totals(
    watching: bool,
    window: Window,
    app: AppHandle,
) -> Result<Option<ResourceSampleTotals>, String> {
    let label = window.label().to_owned();
    let (hook, start) = {
        let registry = app.state::<ResourceRegistry>();
        let mut state = registry
            .totals_watch
            .lock()
            .map_err(|_| "Resource sampler is unavailable".to_string())?;
        if !watching {
            state.remove(&label);
            return Ok(None);
        }
        state.add(&label)
    };
    if hook {
        let handle = app.clone();
        window.on_window_event(move |event| {
            if matches!(event, WindowEvent::Destroyed) {
                if let Ok(mut state) = handle.state::<ResourceRegistry>().totals_watch.lock() {
                    state.remove(&label);
                }
            }
        });
    }
    if start {
        spawn_sampler(app.clone());
    }
    sample_off_thread(&app).await.map(Some)
}

fn spawn_sampler(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last: Option<DisplayKey> = None;
        loop {
            tokio::time::sleep(SAMPLE_INTERVAL).await;
            let keep = match app.state::<ResourceRegistry>().totals_watch.lock() {
                Ok(mut state) => state.keep_sampling(),
                Err(_) => false,
            };
            if !keep {
                return;
            }
            let Ok(totals) = sample_off_thread(&app).await else { continue };
            let key = DisplayKey::of(&totals);
            if last.as_ref() != Some(&key) {
                last = Some(key);
                let _ = app.emit(RESOURCE_TOTALS_EVENT, &totals);
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn totals(cpu_percent: f32, bytes: u64) -> ResourceSampleTotals {
        ResourceSampleTotals {
            cpu_percent,
            physical_footprint_bytes: bytes,
            rss_bytes: bytes,
            process_count: 3,
            active_source_directory_reads: 0,
        }
    }

    #[test]
    fn display_key_follows_what_the_strip_shows() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(5 * 1024 * 1024 + 300_000), "5.3 MB");
        assert_eq!(format_bytes(250 * 1024 * 1024), "250 MB");
        assert_eq!(format_bytes(3 * 1024 * 1024 * 1024), "3.00 GB");
        assert_ne!(DisplayKey::of(&totals(1.04, 1)), DisplayKey::of(&totals(1.06, 1)));
        assert_eq!(DisplayKey::of(&totals(1.01, 1)), DisplayKey::of(&totals(1.04, 1)));
        // Bytes that round to the same label are the same figure.
        assert_eq!(
            DisplayKey::of(&totals(1.0, 250 * 1024 * 1024)),
            DisplayKey::of(&totals(1.0, 250 * 1024 * 1024 + 4096))
        );
    }

    #[test]
    fn one_sampler_however_watchers_come_and_go() {
        let mut state = WatchState::default();
        assert_eq!(state.add("main"), (true, true));
        assert_eq!(state.add("main"), (false, false), "watching twice is one watcher");
        state.remove("main");
        state.remove("unknown");
        // A watch that lands before the sampler checks keeps the running sampler.
        assert_eq!(state.add("main"), (false, false));
        assert!(state.keep_sampling());
        state.remove("main");
        assert!(!state.keep_sampling(), "the sampler stops once nobody watches");
        assert_eq!(state.add("main"), (false, true), "the next watch starts a new sampler");
    }
}
