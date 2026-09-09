use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Claude,
    Codex,
}
impl Provider {
    pub fn key(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionStatus {
    Connected,
    NeedsAuth,
    Unavailable,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaWindow {
    pub id: String,
    pub label: String,
    pub duration_minutes: u32,
    pub used_percent: f64,
    pub resets_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaSnapshot {
    pub id: String,
    pub provider: Provider,
    pub account_id: String,
    pub device_id: String,
    pub fetched_at: i64,
    pub status: ConnectionStatus,
    pub windows: Vec<QuotaWindow>,
    pub message: Option<String>,
    pub retry_after_seconds: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TokenUsage {
    pub input: Option<u64>,
    pub cache_read: Option<u64>,
    pub cache_write: Option<u64>,
    pub output: Option<u64>,
    pub reasoning: Option<u64>,
}
impl TokenUsage {
    pub fn total(&self) -> u64 {
        self.input
            .unwrap_or(0)
            .saturating_add(self.cache_read.unwrap_or(0))
            .saturating_add(self.cache_write.unwrap_or(0))
            .saturating_add(self.output.unwrap_or(0))
    }
    pub fn add(&mut self, other: &Self) {
        fn sum(a: Option<u64>, b: Option<u64>) -> Option<u64> {
            match (a, b) {
                (None, None) => None,
                _ => Some(a.unwrap_or(0).saturating_add(b.unwrap_or(0))),
            }
        }
        self.input = sum(self.input, other.input);
        self.cache_read = sum(self.cache_read, other.cache_read);
        self.cache_write = sum(self.cache_write, other.cache_write);
        self.output = sum(self.output, other.output);
        self.reasoning = sum(self.reasoning, other.reasoning);
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActivityKind {
    User,
    Subagent,
    Review,
    Background,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptRecord {
    pub id: String,
    pub provider: Provider,
    pub account_id: String,
    pub device_id: String,
    pub session_id: String,
    pub turn_id: String,
    pub timestamp: i64,
    pub preview: String,
    pub status: String,
    #[serde(default)]
    pub completed_at: Option<i64>,
    pub kind: ActivityKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestUsage {
    pub id: String,
    pub prompt_id: Option<String>,
    pub provider: Provider,
    pub account_id: String,
    pub device_id: String,
    pub session_id: String,
    pub timestamp: i64,
    pub model: String,
    pub effort: Option<String>,
    pub tokens: TokenUsage,
    pub kind: ActivityKind,
    pub source: String,
    pub source_event_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaEstimate {
    pub percent: f64,
    pub window_id: String,
    pub confidence: String,
    pub explanation: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HistoryFilter {
    pub query: Option<String>,
    pub provider: Option<Provider>,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub device_id: Option<String>,
    pub from: Option<i64>,
    pub to: Option<i64>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub prompt: PromptRecord,
    pub requests: Vec<RequestUsage>,
    pub tokens: TokenUsage,
    pub quota_estimate: Option<QuotaEstimate>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPage {
    pub items: Vec<HistoryItem>,
    pub total: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyStat {
    pub date: String,
    pub prompts: u64,
    pub tokens: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricsRange {
    pub from: Option<i64>,
    /// Exclusive end: adjacent intervals never double count a request.
    pub to: i64,
}
impl MetricsRange {
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=253_402_300_799).contains(&self.to)
            || self.from.is_some_and(|from| from < 0 || from >= self.to)
        {
            return Err("Choose a valid start time before the end time".into());
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityBucket {
    pub timestamp: i64,
    pub prompts: u64,
    pub tokens: u64,
    pub requests: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageMetrics {
    pub range: MetricsRange,
    pub bucket_seconds: i64,
    pub activity: Vec<ActivityBucket>,
    /// User prompts with at least one request recorded during the interval,
    /// including prompts started earlier. These are the percentile population.
    pub active_prompt_count: u64,
    pub stats: DashboardStats,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelStat {
    pub provider: Provider,
    #[serde(default)]
    pub account_id: Option<String>,
    #[serde(default)]
    pub quota_window_id: Option<String>,
    pub model: String,
    pub effort: Option<String>,
    pub prompt_count: u64,
    pub completed_prompts: u64,
    pub request_count: u64,
    pub total_tokens: u64,
    pub median_tokens: u64,
    pub p75_tokens: u64,
    pub estimated_quota_per_prompt: Option<f64>,
    pub quota_sample_count: u64,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    pub prompt_count: u64,
    pub conversation_count: u64,
    pub request_count: u64,
    pub total_tokens: u64,
    pub token_totals: TokenUsage,
    pub median_tokens: u64,
    pub p75_tokens: u64,
    pub daily: Vec<DailyStat>,
    pub model_stats: Vec<ModelStat>,
    pub computers: Vec<String>,
    pub background_requests: u64,
    #[serde(default)]
    pub quota_allocations: Vec<QuotaAllocation>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub files: u64,
    pub prompts: u64,
    pub requests: u64,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ColorTheme {
    #[default]
    Black,
    Slate,
    Midnight,
    Light,
    System,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MeterDisplay {
    #[default]
    Remaining,
    Used,
}
impl MeterDisplay {
    pub fn label(self) -> &'static str {
        match self {
            Self::Remaining => "left",
            Self::Used => "used",
        }
    }
    pub fn percent(self, used: f64) -> Option<f64> {
        if !used.is_finite() {
            return None;
        }
        let used = used.clamp(0.0, 100.0);
        Some(match self {
            Self::Remaining => 100.0 - used,
            Self::Used => used,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub device_id: String,
    pub device_name: String,
    pub launch_at_login: bool,
    pub sync_folder: Option<String>,
    pub reserve_percent: f64,
    pub refresh_seconds: u64,
    pub codex_path: Option<String>,
    pub claude_path: Option<String>,
    pub codex_home: Option<String>,
    pub claude_home: Option<String>,
    pub strip_x: Option<i32>,
    pub strip_y: Option<i32>,
    pub setup_complete: bool,
    pub theme: ColorTheme,
    pub meter_display: MeterDisplay,
    pub strip_locked: bool,
    pub font_scale: u16,
    pub dock_minutes: u32,
    pub dock_previews: bool,
}
impl Default for AppSettings {
    fn default() -> Self {
        Self {
            device_id: uuid::Uuid::new_v4().to_string(),
            device_name: std::env::var("COMPUTERNAME")
                .or_else(|_| std::env::var("HOSTNAME"))
                .unwrap_or_else(|_| "This computer".into()),
            launch_at_login: true,
            sync_folder: None,
            reserve_percent: 10.0,
            refresh_seconds: 120,
            codex_path: None,
            claude_path: None,
            codex_home: None,
            claude_home: None,
            strip_x: None,
            strip_y: None,
            setup_complete: false,
            theme: ColorTheme::Black,
            meter_display: MeterDisplay::Remaining,
            strip_locked: false,
            font_scale: 120,
            dock_minutes: 60,
            dock_previews: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskClass {
    Quick,
    Everyday,
    Complex,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelCatalogEntry {
    pub id: String,
    pub provider: Provider,
    pub label: String,
    pub efforts: Vec<String>,
    pub quality_tier: u8,
    pub quota_pool: String,
    pub available: bool,
    pub source: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recommendation {
    pub provider: Provider,
    pub model: String,
    pub effort: Option<String>,
    pub task: TaskClass,
    pub confidence: String,
    pub sample_count: u64,
    pub reason: String,
    pub estimated_tokens: Option<u64>,
    pub estimated_quota_percent: Option<f64>,
    pub fits_budget: Option<bool>,
    pub warnings: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationSet {
    pub recommendations: Vec<Recommendation>,
    pub summary: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub dock: crate::dock::DockSummary,
    pub snapshots: Vec<QuotaSnapshot>,
    pub stats: DashboardStats,
    pub settings: AppSettings,
    pub import_report: ImportReport,
    pub importing: bool,
    pub last_sync: Option<i64>,
    pub sync_message: Option<String>,
    pub sync_health: crate::sync::SyncHealth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaAllocation {
    pub provider: Provider,
    pub account_id: String,
    pub window_id: String,
    pub observed_percent: Option<f64>,
    pub allocated_percent: Option<f64>,
    pub unallocated_percent: Option<f64>,
    pub interval_count: u64,
    pub gap_count: u64,
}

#[cfg(test)]
mod display_settings_tests {
    use super::*;
    #[test]
    fn existing_settings_get_display_defaults_without_losing_identity_or_positions() {
        let settings: AppSettings = serde_json::from_value(serde_json::json!({"deviceId":"existing","stripX":-1500,"stripY":200,"setupComplete":true,"launchAtLogin":false,"anchorToTaskbar":true})).unwrap();
        assert_eq!(settings.theme, ColorTheme::Black);
        assert_eq!(settings.meter_display, MeterDisplay::Remaining);
        assert!(!settings.strip_locked);
        assert_eq!(settings.font_scale, 120);
        assert_eq!(settings.device_id, "existing");
        assert_eq!(settings.strip_x, Some(-1500));
        assert_eq!(settings.strip_y, Some(200));
        assert!(settings.setup_complete);
        assert!(!settings.launch_at_login);
        let roundtrip: AppSettings =
            serde_json::from_value(serde_json::to_value(&settings).unwrap()).unwrap();
        assert_eq!(roundtrip.theme, settings.theme);
    }
    #[test]
    fn percentage_mode_keeps_invalid_measurements_unknown() {
        assert_eq!(MeterDisplay::Remaining.percent(0.0), Some(100.0));
        assert_eq!(MeterDisplay::Remaining.percent(100.0), Some(0.0));
        assert_eq!(MeterDisplay::Remaining.percent(62.0), Some(38.0));
        assert_eq!(MeterDisplay::Used.percent(62.0), Some(62.0));
        assert_eq!(MeterDisplay::Remaining.percent(f64::NAN), None);
    }
}
