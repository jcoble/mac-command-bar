use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::process::Command;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessRecord {
    pub name: String,
    pub pid: u32,
    pub user: Option<String>,
    pub cwd: Option<String>,
    pub listening_ports: Vec<u16>,
    pub args: Vec<String>,
}

pub fn scan_lsof_listeners() -> Vec<ProcessRecord> {
    let Ok(output) = Command::new("lsof")
        .args(["-nP", "-iTCP", "-sTCP:LISTEN"])
        .output()
    else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    parse_lsof_listeners(&String::from_utf8_lossy(&output.stdout))
}

pub fn parse_lsof_listeners(output: &str) -> Vec<ProcessRecord> {
    let mut by_pid: BTreeMap<u32, ProcessRecord> = BTreeMap::new();

    for line in output
        .lines()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
    {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 9 {
            continue;
        }

        let Ok(pid) = parts[1].parse::<u32>() else {
            continue;
        };

        let Some(port) = parts.iter().find_map(|part| parse_listener_port(part)) else {
            continue;
        };

        let record = by_pid.entry(pid).or_insert_with(|| ProcessRecord {
            name: parts[0].to_string(),
            pid,
            user: Some(parts[2].to_string()),
            cwd: process_cwd(pid),
            listening_ports: Vec::new(),
            args: process_args(pid),
        });

        if !record.listening_ports.contains(&port) {
            record.listening_ports.push(port);
        }
    }

    by_pid
        .into_values()
        .map(|mut record| {
            record.listening_ports.sort_unstable();
            record
        })
        .collect()
}

fn parse_listener_port(part: &str) -> Option<u16> {
    if !part.contains(':') {
        return None;
    }

    let cleaned = part
        .trim_matches('[')
        .trim_matches(']')
        .trim_end_matches("(LISTEN)");
    let suffix = cleaned.rsplit(':').next()?;
    suffix.parse::<u16>().ok()
}

fn process_cwd(pid: u32) -> Option<String> {
    let output = Command::new("lsof")
        .args(["-a", "-p", &pid.to_string(), "-d", "cwd", "-Fn"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find_map(|line| line.strip_prefix('n').map(ToOwned::to_owned))
}

fn process_args(pid: u32) -> Vec<String> {
    let output = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "command="])
        .output()
        .ok();

    output
        .filter(|output| output.status.success())
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .split_whitespace()
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}
