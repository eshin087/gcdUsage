// Explicitly opt-in development fixtures, never used by the desktop app.
import type {
  AppSettings,
  HistoryFilter,
  HistoryItem,
  Overview,
  Provider,
  RecommendationSet,
  TaskClass,
} from "./types";
const now = Math.floor(Date.now() / 1000);
const settings: AppSettings = {
  deviceId: "preview-laptop",
  deviceName: "My laptop",
  launchAtLogin: true,
  syncFolder: null,
  reservePercent: 10,
  refreshSeconds: 120,
  codexPath: null,
  claudePath: null,
  codexHome: null,
  claudeHome: null,
  stripX: null,
  stripY: null,
  setupComplete: true,
  theme: "black",
  meterDisplay: "remaining",
  anchorToTaskbar: true,
};
const prompts = [
  "Add keyboard navigation to the command palette and preserve the selected item.",
  "Help me understand the tradeoffs in this caching strategy.",
  "Refactor the connection manager to handle interruptions gracefully.",
  "Draft a clear getting-started guide for the new desktop app.",
  "Investigate why imported records appear twice after a sync.",
  "Review this change for correctness and improve the error messages.",
  "Simplify the responsive layout and improve the empty states.",
  "Compare the performance of these two database queries.",
];
const items: HistoryItem[] = Array.from({ length: 76 }, (_, index) => {
  const provider: Provider = index % 3 === 0 ? "claude" : "codex";
  const id = `preview-prompt-${index}`;
  const tokens = {
    input: 1340 + index * 177,
    cacheRead: 6240 + index * 502,
    cacheWrite: provider === "claude" ? 900 + index * 20 : 0,
    output: 780 + index * 61,
    reasoning: index % 3 === 0 ? null : 480 + index * 33,
  };
  return {
    prompt: {
      id,
      provider,
      accountId: "preview-account",
      deviceId: index % 4 === 0 ? "Desktop" : settings.deviceId,
      sessionId: `preview-session-${Math.floor(index / 3)}`,
      turnId: id,
      timestamp: now - index * 2900,
      preview: prompts[index % prompts.length],
      status: "completed",
      kind: "user",
    },
    requests: [
      {
        id: `preview-request-${index}`,
        promptId: id,
        provider,
        accountId: "preview-account",
        deviceId: settings.deviceId,
        sessionId: `preview-session-${Math.floor(index / 3)}`,
        timestamp: now - index * 2900,
        model:
          provider === "claude"
            ? "claude-sonnet-4-6"
            : index % 2 === 0
              ? "gpt-5.4"
              : "gpt-5.3-codex",
        effort: index % 3 === 0 ? null : index % 2 === 0 ? "high" : "medium",
        tokens,
        kind: "user",
        source: "preview",
        sourceEventId: id,
      },
    ],
    tokens,
    quotaEstimate:
      index % 4 === 0
        ? {
            percent: 0.18 + index * 0.003,
            windowId: "weekly",
            confidence: "low",
            explanation:
              "Illustrative preview estimate based on a clean pair of readings.",
          }
        : null,
  };
});
const overview: Overview = {
  snapshots: [
    {
      id: "preview-claude",
      provider: "claude",
      accountId: "preview-account",
      deviceId: settings.deviceId,
      fetchedAt: now - 24,
      status: "connected",
      windows: [
        {
          id: "five_hour",
          label: "Five hour",
          durationMinutes: 300,
          usedPercent: 42,
          resetsAt: now + 8460,
        },
        {
          id: "weekly",
          label: "Weekly",
          durationMinutes: 10080,
          usedPercent: 31,
          resetsAt: now + 292000,
        },
      ],
      message: null,
      retryAfterSeconds: null,
    },
    {
      id: "preview-codex",
      provider: "codex",
      accountId: "preview-account",
      deviceId: settings.deviceId,
      fetchedAt: now - 24,
      status: "connected",
      windows: [
        {
          id: "weekly",
          label: "Weekly",
          durationMinutes: 10080,
          usedPercent: 57,
          resetsAt: now + 167000,
        },
      ],
      message: null,
      retryAfterSeconds: null,
    },
  ],
  stats: {
    promptCount: 348,
    conversationCount: 82,
    requestCount: 1246,
    totalTokens: 8492500,
    tokenTotals: {
      input: 1562000,
      cacheRead: 5261000,
      cacheWrite: 483000,
      output: 1186500,
      reasoning: 643000,
    },
    medianTokens: 18600,
    p75Tokens: 48200,
    daily: Array.from({ length: 14 }, (_, i) => ({
      date: new Date((now - (13 - i) * 86400) * 1000)
        .toISOString()
        .slice(0, 10),
      prompts: [18, 14, 32, 20, 26, 12, 9, 24, 38, 17, 27, 33, 21, 29][i],
      tokens: [
        281000, 208000, 524000, 356000, 467000, 189000, 152000, 418000, 680000,
        320000, 534000, 609000, 367000, 528000,
      ][i],
    })),
    modelStats: [
      {
        provider: "claude",
        model: "claude-sonnet-4-6",
        effort: null,
        promptCount: 128,
        completedPrompts: 128,
        requestCount: 423,
        totalTokens: 3290000,
        medianTokens: 15700,
        p75Tokens: 38100,
        estimatedQuotaPerPrompt: 0.18,
        quotaSampleCount: 25,
      },
      {
        provider: "codex",
        model: "gpt-5.3-codex",
        effort: "medium",
        promptCount: 164,
        completedPrompts: 164,
        requestCount: 618,
        totalTokens: 3820000,
        medianTokens: 20400,
        p75Tokens: 46200,
        estimatedQuotaPerPrompt: 0.29,
        quotaSampleCount: 28,
      },
      {
        provider: "codex",
        model: "gpt-5.4",
        effort: "high",
        promptCount: 56,
        completedPrompts: 56,
        requestCount: 205,
        totalTokens: 1382500,
        medianTokens: 38100,
        p75Tokens: 71400,
        estimatedQuotaPerPrompt: 0.47,
        quotaSampleCount: 22,
      },
    ],
    computers: [settings.deviceId, "Desktop"],
    backgroundRequests: 42,
  },
  settings,
  importReport: { files: 112, prompts: 348, requests: 1246, warnings: [] },
  importing: false,
  lastSync: null,
  syncMessage: null,
};
export async function previewInvoke<T>(
  command: string,
  args: Record<string, unknown> = {},
): Promise<T> {
  if (!import.meta.env.DEV)
    throw new Error("Preview data is unavailable in production.");
  switch (command) {
    case "get_overview":
      return structuredClone(overview) as T;
    case "get_history": {
      const filter = args.filter as HistoryFilter;
      const filtered = items.filter(
        (item) =>
          (!filter.query ||
            item.prompt.preview
              .toLowerCase()
              .includes(filter.query.toLowerCase())) &&
          (!filter.provider || item.prompt.provider === filter.provider) &&
          (!filter.deviceId || item.prompt.deviceId === filter.deviceId) &&
          (filter.from == null || item.prompt.timestamp >= filter.from) &&
          (filter.to == null || item.prompt.timestamp <= filter.to) &&
          (!filter.model ||
            item.requests.some((request) => request.model === filter.model)) &&
          (!filter.effort ||
            item.requests.some(
              (request) => (request.effort ?? "unknown") === filter.effort,
            )),
      );
      return {
        items: filtered.slice(
          filter.offset ?? 0,
          (filter.offset ?? 0) + (filter.limit ?? 50),
        ),
        total: filtered.length,
      } as T;
    }
    case "get_recommendations": {
      const task = args.task as TaskClass;
      const result: RecommendationSet = {
        summary:
          "You have room to keep working with both providers. These suggestions preserve a 10% reserve until your next reset.",
        recommendations: [
          {
            provider: "claude",
            model: "Claude Sonnet",
            effort: task === "complex" ? "high" : "medium",
            task,
            confidence: "medium",
            sampleCount: 128,
            reason:
              "A good fit for focused coding and writing, with enough weekly allowance for your recent pace.",
            estimatedTokens: 18700,
            estimatedQuotaPercent: 0.18,
            fitsBudget: true,
            warnings: [],
          },
          {
            provider: "codex",
            model: "GPT-5.3-Codex",
            effort:
              task === "quick" ? "low" : task === "complex" ? "high" : "medium",
            task,
            confidence: "medium",
            sampleCount: 164,
            reason:
              "Fits your projected weekly budget at your current pace. Save higher reasoning levels for work that benefits from them.",
            estimatedTokens: 24600,
            estimatedQuotaPercent: 0.29,
            fitsBudget: true,
            warnings: [],
          },
        ],
      };
      return result as T;
    }
    case "save_settings":
      Object.assign(settings, args.settings);
      return structuredClone(settings) as T;
    case "select_sync_folder":
      return "Preview / Shared Usage" as T;
    case "export_history":
      return "Preview only: no file was written" as T;
    case "refresh_usage":
      overview.snapshots.forEach(
        (snapshot) => (snapshot.fetchedAt = Math.floor(Date.now() / 1000)),
      );
      return undefined as T;
    case "import_history":
      return undefined as T;
    case "reconnect_provider":
      return undefined as T;
    default:
      throw new Error(`Preview command unavailable: ${command}`);
  }
}
