use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderUsageState {
    Available,
    Unavailable,
    Error,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsageWindow {
    pub name: String,
    pub semantics: String,
    pub percent_consumed: Option<f64>,
    pub percent_remaining: Option<f64>,
    pub reset_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsageSnapshot {
    pub provider: String,
    pub account: Option<String>,
    pub instance_id: String,
    pub state: ProviderUsageState,
    pub windows: Vec<ProviderUsageWindow>,
    pub captured_at: u128,
    pub source: Option<String>,
    pub source_version: Option<String>,
    pub unavailable_reason: Option<String>,
}

pub trait ProviderUsageReader: Send + Sync {
    fn read(&self, provider: &str, instance_id: &str) -> ProviderUsageSnapshot;
}

#[tauri::command]
pub fn read_current_provider_usage(
    provider: Option<String>,
    instance_id: Option<String>,
) -> Result<ProviderUsageSnapshot, String> {
    let provider = provider.unwrap_or_else(|| "unknown".to_string());
    let instance_id = instance_id.unwrap_or_else(|| "unknown".to_string());
    Ok(normalize_provider_usage(
        UnavailableProviderUsageReader.read(&provider, &instance_id),
    ))
}

#[derive(Debug, Default, Clone, Copy)]
pub struct UnavailableProviderUsageReader;

impl ProviderUsageReader for UnavailableProviderUsageReader {
    fn read(&self, provider: &str, instance_id: &str) -> ProviderUsageSnapshot {
        unavailable_snapshot(provider, instance_id, "The provider has not advertised quota data")
    }
}

pub fn unavailable_snapshot(provider: &str, instance_id: &str, reason: &str) -> ProviderUsageSnapshot {
    ProviderUsageSnapshot {
        provider: provider.trim().to_string(),
        account: None,
        instance_id: instance_id.trim().to_string(),
        state: ProviderUsageState::Unavailable,
        windows: Vec::new(),
        captured_at: now_ms(),
        source: None,
        source_version: None,
        unavailable_reason: Some(reason.to_string()),
    }
}

/// Accept only a provider-authored quota snapshot. Transcript token counts are
/// deliberately not an input to this function.
pub fn normalize_provider_usage(mut snapshot: ProviderUsageSnapshot) -> ProviderUsageSnapshot {
    if snapshot.provider.trim().is_empty() || snapshot.instance_id.trim().is_empty() {
        snapshot.state = ProviderUsageState::Unavailable;
        snapshot.windows.clear();
        snapshot.unavailable_reason = Some("Provider identity is incomplete".to_string());
        return snapshot;
    }
    if snapshot.state == ProviderUsageState::Available && snapshot.windows.is_empty() {
        snapshot.state = ProviderUsageState::Unavailable;
        snapshot.unavailable_reason = Some("The provider returned no quota windows".to_string());
    }
    for window in &mut snapshot.windows {
        window.percent_consumed = window.percent_consumed.map(|value| value.clamp(0.0, 100.0));
        window.percent_remaining = window.percent_remaining.map(|value| value.clamp(0.0, 100.0));
        if window.semantics.trim().is_empty() {
            window.semantics = "Provider-defined quota window".to_string();
        }
    }
    snapshot
}

fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_current_never_infers_quota_from_tokens() {
        let snapshot = normalize_provider_usage(ProviderUsageSnapshot {
            provider: "provider-a".into(),
            account: None,
            instance_id: "instance-a".into(),
            state: ProviderUsageState::Unavailable,
            windows: Vec::new(),
            captured_at: 1,
            source: None,
            source_version: None,
            unavailable_reason: Some("No quota data".into()),
        });
        assert_eq!(snapshot.state, ProviderUsageState::Unavailable);
        assert!(snapshot.windows.is_empty());
    }
}
