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

// Pages use a stable timestamp/id cursor so new imports cannot shift older rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockCursor {
    pub timestamp: i64,
    pub id: String,
}
#[derive(Debug, Clone, Default)]
pub struct DockPage {
    pub prompts: Vec<DockPrompt>,
    pub next: Option<DockCursor>,
}
impl DockPrompt {
    pub fn project_label(&self) -> &str {
        self.context.split_once(" / ").map(|v| v.0).unwrap_or(&self.context)
    }
    pub fn model_label(&self) -> String {
        let models: std::collections::BTreeSet<_> = self.models.split(" / ")
            .map(|m| m.split_once(" · ").map(|v| v.0).unwrap_or(m)).collect();
        if self.models.starts_with("Unknown model") { return "Unknown".into(); }
        let first = models.iter().next().copied().unwrap_or("Unknown");
        if models.len() > 1 { format!("{first} +{}", models.len() - 1) } else { first.into() }
    }
}


// The native picker accepts whole units and stores a bounded minute count.
pub fn duration_minutes(value: &str, unit: u32) -> Option<u32> {
    if ![1,60,1440].contains(&unit) || value.is_empty() ||
        !value.bytes().all(|b|b.is_ascii_digit()) {return None;}
    value.parse::<u32>().ok()?.checked_mul(unit).filter(|n|(1..=43200).contains(n))
}
#[cfg(test)]
mod duration_tests {
    use super::duration_minutes;
    #[test]
    fn whole_units_convert_without_overflow_or_rounding() {
        for (value,unit,expected) in [("1",1,Some(1)),("24",60,Some(1440)),
            ("30",1440,Some(43200)),("720",60,Some(43200)),("43200",1,Some(43200)),
            ("31",1440,None),("0",1,None),("1.5",60,None),("-1",1,None),
            ("+1",1,None),("",60,None),("4294967295",1440,None),("3",7,None)] {
            assert_eq!(duration_minutes(value,unit),expected);
        }
    }
}
