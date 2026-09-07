// Keep this wire contract aligned with src-tauri/src/models.rs (camelCase serde).
export type Provider = "claude" | "codex";
export type ConnectionStatus =
  "connected" | "needs_auth" | "unavailable" | "error";
export type ActivityKind = "user" | "subagent" | "review" | "background";
export type TaskClass = "quick" | "everyday" | "complex";
export interface QuotaWindow {
  id: string;
  label: string;
  durationMinutes: number;
  usedPercent: number;
  resetsAt: number | null;
}
export interface QuotaSnapshot {
  id: string;
  provider: Provider;
  accountId: string;
  deviceId: string;
  fetchedAt: number;
  status: ConnectionStatus;
  windows: QuotaWindow[];
  message: string | null;
  retryAfterSeconds: number | null;
}
export interface TokenUsage {
  input: number | null;
  cacheRead: number | null;
  cacheWrite: number | null;
  output: number | null;
  reasoning: number | null;
}
export interface PromptRecord {
  id: string;
  provider: Provider;
  accountId: string;
  deviceId: string;
  sessionId: string;
  turnId: string;
  timestamp: number;
  preview: string;
  status: string;
  completedAt?: number | null;
  kind: ActivityKind;
}
export interface RequestUsage {
  id: string;
  promptId: string | null;
  provider: Provider;
  accountId: string;
  deviceId: string;
  sessionId: string;
  timestamp: number;
  model: string;
  effort: string | null;
  tokens: TokenUsage;
  kind: ActivityKind;
  source: string;
  sourceEventId: string;
}
export interface QuotaEstimate {
  percent: number;
  windowId: string;
  confidence: string;
  explanation: string;
}
export interface HistoryFilter {
  query?: string | null;
  provider?: Provider | null;
  model?: string | null;
  effort?: string | null;
  deviceId?: string | null;
  from?: number | null;
  to?: number | null;
  limit?: number;
  offset?: number;
}
export interface HistoryItem {
  prompt: PromptRecord;
  requests: RequestUsage[];
  tokens: TokenUsage;
  quotaEstimate: QuotaEstimate | null;
}
export interface HistoryPage {
  items: HistoryItem[];
  total: number;
}
export interface DailyStat {
  date: string;
  prompts: number;
  tokens: number;
}
export interface ModelStat {
  provider: Provider;
  accountId?: string | null;
  quotaWindowId?: string | null;
  model: string;
  effort: string | null;
  promptCount: number;
  completedPrompts: number;
  requestCount: number;
  totalTokens: number;
  medianTokens: number;
  p75Tokens: number;
  estimatedQuotaPerPrompt: number | null;
  quotaSampleCount: number;
}
export interface DashboardStats {
  promptCount: number;
  conversationCount: number;
  requestCount: number;
  totalTokens: number;
  tokenTotals: TokenUsage;
  medianTokens: number;
  p75Tokens: number;
  daily: DailyStat[];
  modelStats: ModelStat[];
  computers: string[];
  backgroundRequests: number;
  quotaAllocations?: QuotaAllocation[];
}
export interface QuotaAllocation {
  provider: Provider;
  accountId: string;
  windowId: string;
  observedPercent: number | null;
  allocatedPercent: number | null;
  unallocatedPercent: number | null;
  intervalCount: number;
  gapCount: number;
}
export interface ImportReport {
  files: number;
  prompts: number;
  requests: number;
  warnings: string[];
}
export type ColorTheme = "black" | "slate" | "midnight" | "light" | "system";
export type MeterDisplay = "remaining" | "used";
export interface AppSettings {
  deviceId: string;
  deviceName: string;
  launchAtLogin: boolean;
  syncFolder: string | null;
  reservePercent: number;
  refreshSeconds: number;
  codexPath: string | null;
  claudePath: string | null;
  codexHome: string | null;
  claudeHome: string | null;
  stripX: number | null;
  stripY: number | null;
  setupComplete: boolean;
  theme: ColorTheme;
  meterDisplay: MeterDisplay;
  anchorToTaskbar: boolean;
}
export interface Recommendation {
  provider: Provider;
  model: string;
  effort: string | null;
  task: TaskClass;
  confidence: string;
  sampleCount: number;
  reason: string;
  estimatedTokens: number | null;
  estimatedQuotaPercent: number | null;
  fitsBudget: boolean | null;
  warnings: string[];
}
export interface RecommendationSet {
  recommendations: Recommendation[];
  summary: string;
}
export interface Overview {
  snapshots: QuotaSnapshot[];
  stats: DashboardStats;
  settings: AppSettings;
  importReport: ImportReport;
  importing: boolean;
  lastSync: number | null;
  syncMessage: string | null;
}
