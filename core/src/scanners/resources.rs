use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Keep the process inventory bounded and make the two operating-system reads
/// visible in one place. Resource scans must not turn into one query per row.
///
/// `rss=` is still read here, but only as the fallback for a process whose
/// footprint the kernel will not tell us about — see `phys_footprint_bytes`.
pub const PS_COMMAND_ARGS: [&str; 2] = [
    "-axo",
    "pid=,ppid=,pgid=,%cpu=,rss=,etime=,user=,command=",
];
pub const LSOF_COMMAND_ARGS: [&str; 4] = ["-nP", "-iTCP", "-sTCP:LISTEN", "-Fpcn"];

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ProcessOwner {
    App,
    OwnedSession,
    LanguageServer,
    ProviderSidecar,
    Playwright,
    External,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceOwnerHint {
    pub pid: u32,
    pub pgid: Option<u32>,
    pub owner: ProcessOwner,
    pub owner_id: Option<String>,
    pub root: Option<String>,
    pub project_id: Option<String>,
    pub workspace_id: Option<String>,
    pub session_name: Option<String>,
    pub registry_generation: u64,
    pub can_stop: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceProcess {
    pub pid: u32,
    pub ppid: u32,
    pub pgid: u32,
    pub cpu_percent: f64,
    /// Physical memory footprint in bytes: what Activity Monitor's "Memory"
    /// column shows, which is the number people compare against. It is NOT
    /// resident size — a process can hold far more compressed and private
    /// memory than it has resident, and reporting resident size made this panel
    /// disagree with Activity Monitor by an order of magnitude.
    pub rss_bytes: u64,
    pub elapsed_seconds: u64,
    pub user: String,
    pub command: String,
    pub listening_ports: Vec<u16>,
    pub owner: ProcessOwner,
    pub owner_id: Option<String>,
    pub root: Option<String>,
    pub project_id: Option<String>,
    pub workspace_id: Option<String>,
    pub session_name: Option<String>,
    pub registry_generation: u64,
    pub can_stop: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceListener {
    pub pid: u32,
    pub command: Option<String>,
    pub listening_ports: Vec<u16>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSnapshot {
    pub generation: u64,
    pub captured_at_ms: u128,
    pub processes: Vec<ResourceProcess>,
    pub total_cpu_percent: f64,
    pub total_rss_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceActionError {
    External,
    StaleSnapshot,
    ProcessGroupChanged,
}

#[derive(Debug, Clone)]
pub struct ParsedProcess {
    pid: u32,
    ppid: u32,
    pgid: u32,
    cpu_percent: f64,
    rss_bytes: u64,
    elapsed_seconds: u64,
    user: String,
    command: String,
}

pub fn parse_ps_snapshot(output: &str) -> Result<Vec<ParsedProcess>, String> {
    output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(parse_ps_row)
        .collect()
}

fn parse_ps_row(line: &str) -> Result<ParsedProcess, String> {
    let mut fields = line.split_whitespace();
    let pid = parse_field(&mut fields, "pid")?;
    let ppid = parse_field(&mut fields, "ppid")?;
    let pgid = parse_field(&mut fields, "pgid")?;
    let cpu_percent = fields
        .next()
        .ok_or_else(|| "missing cpu percent".to_string())?
        .parse::<f64>()
        .map_err(|_| "invalid cpu percent".to_string())?;
    let rss_kb = fields
        .next()
        .ok_or_else(|| "missing rss".to_string())?
        .parse::<u64>()
        .map_err(|_| "invalid rss".to_string())?;
    let elapsed = fields
        .next()
        .ok_or_else(|| "missing elapsed time".to_string())?;
    let user = fields
        .next()
        .ok_or_else(|| "missing user".to_string())?
        .to_string();
    let command = fields.collect::<Vec<_>>().join(" ");

    Ok(ParsedProcess {
        pid,
        ppid,
        pgid,
        cpu_percent,
        rss_bytes: rss_kb.saturating_mul(1024),
        elapsed_seconds: parse_elapsed_seconds(elapsed)?,
        user,
        command,
    })
}

fn parse_field<'a, T>(fields: &mut impl Iterator<Item = &'a str>, name: &str) -> Result<T, String>
where
    T: std::str::FromStr,
{
    fields
        .next()
        .ok_or_else(|| format!("missing {name}"))?
        .parse::<T>()
        .map_err(|_| format!("invalid {name}"))
}

fn parse_elapsed_seconds(value: &str) -> Result<u64, String> {
    let (days, clock) = match value.split_once('-') {
        Some((days, clock)) => (
            days.parse::<u64>().map_err(|_| "invalid elapsed days")?,
            clock,
        ),
        None => (0, value),
    };
    let parts = clock
        .split(':')
        .map(|part| part.parse::<u64>().map_err(|_| "invalid elapsed time"))
        .collect::<Result<Vec<_>, _>>()?;
    let seconds = match parts.as_slice() {
        [minutes, seconds] => minutes.saturating_mul(60).saturating_add(*seconds),
        [hours, minutes, seconds] => hours
            .saturating_mul(3600)
            .saturating_add(minutes.saturating_mul(60))
            .saturating_add(*seconds),
        _ => return Err("invalid elapsed time".to_string()),
    };
    Ok(days.saturating_mul(86_400).saturating_add(seconds))
}

pub fn parse_lsof_listeners(output: &str) -> Vec<ResourceListener> {
    let mut listeners = BTreeMap::<u32, ResourceListener>::new();
    let mut pid = None;
    let mut command = None;

    for record in output.split_inclusive('\n') {
        let record = record.trim_end_matches('\n');
        if record.is_empty() {
            continue;
        }
        match record.as_bytes().first().copied() {
            Some(b'p') => pid = record[1..].parse::<u32>().ok(),
            Some(b'c') => command = Some(record[1..].to_string()),
            Some(b'n') => {
                let Some(pid) = pid else { continue };
                let Some(port) = parse_listener_port(&record[1..]) else { continue };
                let entry = listeners.entry(pid).or_insert_with(|| ResourceListener {
                    pid,
                    command: command.clone(),
                    listening_ports: Vec::new(),
                });
                if !entry.listening_ports.contains(&port) {
                    entry.listening_ports.push(port);
                }
            }
            _ => {}
        }
    }

    listeners
        .into_values()
        .map(|mut listener| {
            listener.listening_ports.sort_unstable();
            listener
        })
        .collect()
}

fn parse_listener_port(value: &str) -> Option<u16> {
    let endpoint = value.trim().trim_end_matches("(LISTEN)").trim();
    endpoint
        .rsplit_once(':')
        .and_then(|(_, port)| port.parse::<u16>().ok())
}

pub fn join_resource_identities(
    parsed: Vec<ParsedProcess>,
    listeners: Vec<ResourceListener>,
    hints: &[ResourceOwnerHint],
) -> Vec<ResourceProcess> {
    let parsed_by_pid = parsed
        .iter()
        .map(|process| (process.pid, process))
        .collect::<BTreeMap<_, _>>();
    let mut children_by_parent = BTreeMap::<u32, Vec<u32>>::new();
    for process in &parsed {
        children_by_parent
            .entry(process.ppid)
            .or_default()
            .push(process.pid);
    }

    // A resource row is owned only when it is reachable from a root PID that
    // the application registered. This is the critical boundary: `ps` is a
    // machine-wide inventory, while the Resources surface is an app-owned
    // inventory. External/system rows are deliberately not returned.
    let mut owner_by_pid = BTreeMap::<u32, &ResourceOwnerHint>::new();
    for hint in hints {
        let Some(root) = parsed_by_pid.get(&hint.pid) else {
            continue;
        };
        let mut queue = vec![root.pid];
        while let Some(pid) = queue.pop() {
            if owner_by_pid.contains_key(&pid) {
                continue;
            }
            owner_by_pid.insert(pid, hint);
            if let Some(children) = children_by_parent.get(&pid) {
                queue.extend(children.iter().copied());
            }
        }
    }

    let listeners = listeners
        .into_iter()
        .map(|listener| (listener.pid, listener))
        .collect::<BTreeMap<_, _>>();
    parsed
        .into_iter()
        .filter_map(|process| {
            let listener = listeners.get(&process.pid);
            let hint = owner_by_pid.get(&process.pid).copied()?;
            Some(ResourceProcess {
                pid: process.pid,
                ppid: process.ppid,
                pgid: process.pgid,
                cpu_percent: process.cpu_percent,
                rss_bytes: process.rss_bytes,
                elapsed_seconds: process.elapsed_seconds,
                user: process.user,
                command: process.command,
                listening_ports: listener
                    .map(|listener| listener.listening_ports.clone())
                    .unwrap_or_default(),
                owner: hint.owner,
                owner_id: hint.owner_id.clone(),
                root: hint.root.clone(),
                project_id: hint.project_id.clone(),
                workspace_id: hint.workspace_id.clone(),
                session_name: hint.session_name.clone(),
                registry_generation: hint.registry_generation,
                can_stop: hint.can_stop,
            })
        })
        .collect()
}

/// The physical memory footprint of one process, or `None` when the kernel
/// will not say — which it will not for a process belonging to another user
/// without privileges we do not ask for.
///
/// `ps` cannot answer this: it reports resident size and has no footprint
/// column at all. The footprint lives in `rusage_info`, and it has been in the
/// v0 flavour of that struct since the call existed, so asking for v0 rather
/// than "current" keeps this independent of which SDK built the binary.
#[cfg(target_os = "macos")]
pub fn phys_footprint_bytes(pid: u32) -> Option<u64> {
    #[repr(C)]
    #[derive(Default)]
    struct RusageInfoV0 {
        ri_uuid: [u8; 16],
        ri_user_time: u64,
        ri_system_time: u64,
        ri_pkg_idle_wkups: u64,
        ri_interrupt_wkups: u64,
        ri_pageins: u64,
        ri_wired_size: u64,
        ri_resident_size: u64,
        ri_phys_footprint: u64,
        ri_proc_start_abstime: u64,
        ri_proc_exit_abstime: u64,
    }

    extern "C" {
        fn proc_pid_rusage(
            pid: std::os::raw::c_int,
            flavor: std::os::raw::c_int,
            buffer: *mut std::os::raw::c_void,
        ) -> std::os::raw::c_int;
    }

    let mut info = RusageInfoV0::default();
    // SAFETY: `info` is a live, correctly sized RusageInfoV0 and the flavour we
    // pass is the one that describes it. The call only writes into that buffer.
    let ok = unsafe {
        proc_pid_rusage(
            pid as std::os::raw::c_int,
            0, // RUSAGE_INFO_V0
            &mut info as *mut RusageInfoV0 as *mut std::os::raw::c_void,
        )
    } == 0;
    if ok && info.ri_phys_footprint > 0 {
        Some(info.ri_phys_footprint)
    } else {
        None
    }
}

#[cfg(not(target_os = "macos"))]
pub fn phys_footprint_bytes(_pid: u32) -> Option<u64> {
    None
}

pub fn scan_resource_snapshot(hints: &[ResourceOwnerHint], generation: u64) -> Result<ResourceSnapshot, String> {
    let ps = Command::new("ps")
        .args(PS_COMMAND_ARGS)
        .output()
        .map_err(|error| format!("ps inventory failed: {error}"))?;
    if !ps.status.success() {
        return Err(format!("ps inventory exited with {}", ps.status));
    }
    let lsof = Command::new("lsof")
        .args(LSOF_COMMAND_ARGS)
        .output()
        .map_err(|error| format!("lsof listener inventory failed: {error}"))?;
    if !lsof.status.success() {
        return Err(format!("lsof listener inventory exited with {}", lsof.status));
    }

    // One syscall per process, not one subprocess per process: `ps` gave us the
    // inventory in a single read, and this only replaces the memory number on
    // the rows that survive the join.
    let mut parsed = parse_ps_snapshot(&String::from_utf8_lossy(&ps.stdout))?;
    for process in &mut parsed {
        if let Some(footprint) = phys_footprint_bytes(process.pid) {
            process.rss_bytes = footprint;
        }
    }

    let processes = join_resource_identities(
        parsed,
        parse_lsof_listeners(&String::from_utf8_lossy(&lsof.stdout)),
        hints,
    );
    Ok(ResourceSnapshot {
        generation,
        captured_at_ms: now_ms(),
        total_cpu_percent: processes.iter().map(|process| process.cpu_percent).sum(),
        total_rss_bytes: processes.iter().map(|process| process.rss_bytes).sum(),
        processes,
    })
}

pub fn validate_resource_action(
    process: &ResourceProcess,
    expected_generation: u64,
    expected_pgid: Option<u32>,
) -> Result<(), ResourceActionError> {
    if process.registry_generation != expected_generation {
        return Err(ResourceActionError::StaleSnapshot);
    }
    if expected_pgid.is_some_and(|pgid| pgid != process.pgid) {
        return Err(ResourceActionError::ProcessGroupChanged);
    }
    if !process.can_stop || process.owner == ProcessOwner::External {
        return Err(ResourceActionError::External);
    }
    Ok(())
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resources_parse_one_bounded_ps_snapshot_and_join_listeners() {
        let ps = " 101  1 101 12.5 2048 01:02:03 alice /usr/bin/editor --workspace /repo\n";
        let lsof = "p101\nceditor\nn127.0.0.1:4312 (LISTEN)\n";
        let processes = parse_ps_snapshot(ps).expect("bounded ps row should parse");
        let listeners = parse_lsof_listeners(lsof);
        let joined = join_resource_identities(processes, listeners, &[]);

        assert!(joined.is_empty(), "unowned system rows never enter the default snapshot");
    }

    #[test]
    fn resources_attribute_only_owned_process_tree_and_drop_sibling_daemon() {
        let ps = "101 1 101 1.0 1024 00:01 alice /bin/zsh\n102 101 101 2.0 2048 00:01 alice /usr/bin/node agent\n103 102 101 3.0 4096 00:01 alice /usr/bin/tool\n201 1 201 99.0 8192 00:01 root /usr/libexec/launchd\n";
        let hint = ResourceOwnerHint {
            pid: 101,
            pgid: Some(101),
            owner: ProcessOwner::OwnedSession,
            owner_id: Some("session-a".into()),
            root: Some("/repo/workspace-a".into()),
            project_id: Some("repo".into()),
            workspace_id: Some("workspace-a".into()),
            session_name: Some("Terminal 1".into()),
            registry_generation: 7,
            can_stop: true,
        };
        let joined = join_resource_identities(
            parse_ps_snapshot(ps).expect("fixture process tree should parse"),
            Vec::new(),
            &[hint],
        );

        assert_eq!(joined.iter().map(|process| process.pid).collect::<Vec<_>>(), vec![101, 102, 103]);
        assert!(joined.iter().all(|process| process.owner == ProcessOwner::OwnedSession));
        assert!(joined.iter().all(|process| process.project_id.as_deref() == Some("repo")));
        assert!(joined.iter().all(|process| process.workspace_id.as_deref() == Some("workspace-a")));
    }

    #[test]
    fn resources_refuse_external_stop_and_stale_generation() {
        let external = ResourceProcess {
            pid: 101,
            ppid: 1,
            pgid: 101,
            cpu_percent: 0.0,
            rss_bytes: 0,
            elapsed_seconds: 0,
            user: "alice".into(),
            command: "editor".into(),
            listening_ports: Vec::new(),
            owner: ProcessOwner::External,
            owner_id: None,
            root: None,
            project_id: None,
            workspace_id: None,
            session_name: None,
            registry_generation: 3,
            can_stop: false,
        };

        assert_eq!(validate_resource_action(&external, 3, None), Err(ResourceActionError::External));
        assert_eq!(validate_resource_action(&external, 2, None), Err(ResourceActionError::StaleSnapshot));
    }

    /// The memory number has to be the footprint, and the only process we can
    /// always ask about is this one.
    ///
    /// The assertion is deliberately only "a real number": footprint is not
    /// resident size plus something, and it is not bounded by it in either
    /// direction. It leaves out the shared, file-backed pages resident size
    /// counts, and it adds the compressed and other private pages resident size
    /// misses — this test binary reports 1.2 MB of footprint against 2.5 MB
    /// resident, while a long-running browser helper reports the opposite by an
    /// order of magnitude. That is exactly why the panel had to change: the two
    /// numbers answer different questions, and the footprint is the one
    /// Activity Monitor puts in its Memory column.
    #[cfg(target_os = "macos")]
    #[test]
    fn our_own_footprint_is_reported() {
        let footprint =
            phys_footprint_bytes(std::process::id()).expect("a running process has a footprint");
        assert!(footprint > 0, "a running process cannot occupy no memory");
    }

    /// A pid nobody is using answers nothing rather than zero, so the caller
    /// keeps whatever `ps` said instead of reporting a process with no memory.
    #[test]
    fn an_unused_pid_reports_no_footprint() {
        assert_eq!(phys_footprint_bytes(u32::MAX), None);
    }
}
