//! Small cached records for native dock painting. No I/O on the window thread.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockModel {
    pub provider: String,
    pub model: String,
    pub effort: Option<String>,
    pub tokens: u64,
    pub requests: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockPrompt {
    pub context: String,
    pub id: String,
    pub provider: String,
    pub preview: String,
    pub timestamp: Option<i64>,
    pub tokens: Option<u64>,
    pub models: String,
    pub status: String,
    pub browser: bool,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockSummary {
    pub updated_at: Option<i64>,
    pub minutes: u32,
    pub tokens: u64,
    pub unknown_requests: u64,
    pub provider_tokens: std::collections::BTreeMap<String, u64>,
    pub models: Vec<DockModel>,
    pub prompts: Vec<DockPrompt>,
}
pub fn compact(value: u64) -> String {
    match value {
        1_000_000_000.. => format!("{:.1}B", value as f64 / 1e9),
        1_000_000.. => format!("{:.1}M", value as f64 / 1e6),
        1_000.. => format!("{:.1}K", value as f64 / 1e3),
        _ => value.to_string(),
    }
}
pub fn interval(minutes: u32) -> String {
    if minutes % 1440 == 0 {
        format!("{}d", minutes / 1440)
    } else if minutes % 60 == 0 {
        format!("{}h", minutes / 60)
    } else {
        format!("{minutes}m")
    }
}
impl DockSummary {
    pub fn recent(&self, provider: Option<&str>) -> Vec<&DockPrompt> {
        self.prompts
            .iter()
            .filter(|p| provider.is_none_or(|v| p.provider == v))
            .take(10)
            .collect()
    }
    pub fn model_summary(&self, provider: Option<&str>, limit: usize) -> String {
        let models: Vec<_> = self
            .models
            .iter()
            .filter(|m| provider.is_none_or(|p| m.provider == p))
            .take(limit)
            .map(|m| format!("{} · {}", m.model, m.effort.as_deref().unwrap_or("unknown")))
            .collect();
        if models.is_empty() {
            "No measured requests".into()
        } else {
            models.join("  /  ")
        }
    }
}
