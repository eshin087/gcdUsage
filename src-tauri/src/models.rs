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
    pub snapshots: Vec<QuotaSnapshot>,
    pub stats: DashboardStats,
    pub settings: AppSettings,
    pub import_report: ImportReport,
    pub importing: bool,
    pub last_sync: Option<i64>,
    pub sync_message: Option<String>,
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
