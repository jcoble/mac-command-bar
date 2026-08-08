use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Keep the process inventory bounded and make the two operating-system reads
/// visible in one place. Resource scans must not turn into one query per row.
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
    pub rss_bytes: u64,
    pub elapsed_seconds: u64,
    pub user: String,
    pub command: String,
    pub listening_ports: Vec<u16>,
    pub owner: ProcessOwner,
    pub owner_id: Option<String>,
    pub root: Option<String>,
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
    let listeners = listeners
        .into_iter()
        .map(|listener| (listener.pid, listener))
        .collect::<BTreeMap<_, _>>();
    let hints = hints
        .iter()
        .map(|hint| (hint.pid, hint))
        .collect::<BTreeMap<_, _>>();

    parsed
        .into_iter()
        .map(|process| {
            let listener = listeners.get(&process.pid);
            let hint = hints.get(&process.pid).copied();
            ResourceProcess {
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
                owner: hint.map(|hint| hint.owner).unwrap_or(ProcessOwner::External),
                owner_id: hint.and_then(|hint| hint.owner_id.clone()),
                root: hint.and_then(|hint| hint.root.clone()),
                registry_generation: hint.map(|hint| hint.registry_generation).unwrap_or(0),
                can_stop: hint.map(|hint| hint.can_stop).unwrap_or(false),
            }
        })
        .collect()
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

    let processes = join_resource_identities(
        parse_ps_snapshot(&String::from_utf8_lossy(&ps.stdout))?,
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

        assert_eq!(joined.len(), 1);
        assert_eq!(joined[0].pid, 101);
        assert_eq!(joined[0].listening_ports, vec![4312]);
        assert_eq!(joined[0].owner, ProcessOwner::External);
        assert!(!joined[0].can_stop);
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
            registry_generation: 3,
            can_stop: false,
        };

        assert_eq!(validate_resource_action(&external, 3, None), Err(ResourceActionError::External));
        assert_eq!(validate_resource_action(&external, 2, None), Err(ResourceActionError::StaleSnapshot));
    }
}
