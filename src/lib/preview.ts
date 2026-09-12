// Explicitly opt-in development fixtures, never used by the desktop app.
import { emptyTokens, totalTokens } from "./format";
import type {
  AppSettings,
  UsageMetrics,
  MetricsRange,
  ModelStat,
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
  stripLocked: false,
  fontScale: 120,
  dockScale: 100,
  dockMinutes: 60,
  dockPreviews: true,
  dockHidden: false,
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
          id: "claude:300",
          label: "Five hour",
          durationMinutes: 300,
          usedPercent: 42,
          resetsAt: now + 8460,
        },
        {
          id: "claude:10080",
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
          id: "codex:10080",
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
    case "set_activity_duration": {
      const minutes=Number(args.minutes);
      if (!Number.isInteger(minutes) || minutes<1 || minutes>43200) throw new Error("Choose 1 minute through 30 days");
      settings.dockMinutes=minutes;
      return undefined as T;
    }
    case "get_recent_allowance": {
      const minutes=Number(args.minutes);
      if (!Number.isInteger(minutes) || minutes<1 || minutes>43200) throw new Error("Choose 1 minute through 30 days");
      return {minutes,from:now-minutes*60,to:now,windows:overview.snapshots.flatMap(snapshot=>snapshot.windows.map((window,index)=>({
        provider:snapshot.provider,windowId:window.id,label:window.label,
        consumedPercent:minutes<2?null:Number(((snapshot.provider==="codex"?0.8:index===0?4.2:1.3)*Math.min(minutes,360)/60).toFixed(2)),
        state:minutes<2?"learning":minutes>360?"partial":"observed",
        observedSeconds:minutes<2?0:Math.max(0,(minutes>360?360:minutes)*60-120),
        firstReadingAt:now-(minutes>360?360:minutes)*60,lastReadingAt:now-120,
        sampleCount:Math.max(1,Math.floor(Math.min(minutes,360)/2)),gapCount:minutes>360?1:0,resetCount:0
      })))} as T;
    }
    case "get_usage_insights": {
      const days=Number(args.days);
      const current=await previewInvoke<UsageMetrics>("get_usage_metrics",{range:{from:now-days*86400,to:now}});
      current.bucketSeconds=86400;
      current.activity=Array.from({length:days},(_,i)=>({
        timestamp:now-(days-i)*86400,prompts:12+(i*17)%35,requests:120+(i*53)%190,
        tokens:Math.round((0.65+Math.sin(i*.7)*.25+(i%5)*.1)*1300000)
      }));
      current.stats.promptCount=current.activity.reduce((n,p)=>n+p.prompts,0);
      current.stats.requestCount=current.activity.reduce((n,p)=>n+p.requests,0);
      current.stats.totalTokens=current.activity.reduce((n,p)=>n+p.tokens,0);
      current.stats.tokenTotals={input:current.stats.totalTokens*.25,cacheRead:current.stats.totalTokens*.61,
        cacheWrite:0,output:current.stats.totalTokens*.14,reasoning:null};
      current.stats.backgroundRequests=Math.round(current.stats.requestCount*.12);
      current.activePromptCount=current.stats.promptCount;
      current.stats.p75Tokens=61000;
      const weights=[.46,.28,.16,.1],labels=["gpt-6-astra","claude-fable","gpt-5.6-sol","claude-sonnet"];
      current.stats.modelStats=labels.map((model,i)=>({provider:i%2?"claude":"codex",model,effort:"high",
        promptCount:Math.round(current.stats.promptCount*weights[i]),completedPrompts:100,
        requestCount:Math.round(current.stats.requestCount*weights[i]),totalTokens:Math.round(current.stats.totalTokens*weights[i]),
        medianTokens:18000,p75Tokens:61000,estimatedQuotaPerPrompt:null,quotaSampleCount:0}));
      const previous=structuredClone(current);previous.stats.totalTokens=Math.round(current.stats.totalTokens*.79);
      previous.range={from:now-days*2*86400,to:now-days*86400};
      return {days,current,previous,activeDays:Math.max(1,days-2),longestStreak:Math.min(days,12),
        hourlyPrompts:Array.from({length:168},(_,i)=>i%24>=8 && i%24<22 ? (i*7+3)%24 : 0),
        unknownRequests:0,forecasts:[
          {provider:"claude",label:"Claude · 5h",state:"before_reset",remaining:24,resetsAt:now+10800,
            ratePerHour:18,exhaustsAt:now+4800,remainingAtReset:0,observedSeconds:3600,sampleCount:31,
            points:Array.from({length:7},(_,i)=>({timestamp:now-3600+i*600,remaining:42-i*3}))},
          {provider:"codex",label:"Codex · weekly",state:"after_reset",remaining:79,resetsAt:now+259200,
            ratePerHour:.18,exhaustsAt:null,remainingAtReset:66,observedSeconds:3600,sampleCount:31,
            points:Array.from({length:7},(_,i)=>({timestamp:now-3600+i*600,remaining:79.18-i*.03}))}
        ]} as T;
    }
    case "get_usage_metrics": {
      const range = args.range as MetricsRange;
      const selected = items.filter(
        (item) =>
          item.prompt.timestamp >= (range.from ?? 0) &&
          item.prompt.timestamp < range.to,
      );
      const totals = emptyTokens();
      const models = new Map<string, ModelStat>();
      const modelSamples = new Map<string, number[]>();
      for (const item of selected)
        for (const request of item.requests) {
          for (const key of [
            "input",
            "cacheRead",
            "cacheWrite",
            "output",
            "reasoning",
          ] as const) {
            if (request.tokens[key] != null)
              totals[key] = (totals[key] ?? 0) + request.tokens[key];
          }
          const key = request.model + ":" + request.effort;
          const row = models.get(key) ?? {
            provider: request.provider,
            model: request.model,
            effort: request.effort,
            promptCount: 0,
            completedPrompts: 0,
            requestCount: 0,
            totalTokens: 0,
            medianTokens: 0,
            p75Tokens: 0,
            estimatedQuotaPerPrompt: null,
            quotaSampleCount: 0,
          };
          row.promptCount++;
          row.completedPrompts++;
          row.requestCount++;
          row.totalTokens += totalTokens(request.tokens) ?? 0;
          const values = modelSamples.get(key) ?? [];
          values.push(totalTokens(request.tokens) ?? 0);
          values.sort((a, b) => a - b);
          modelSamples.set(key, values);
          row.medianTokens = values[Math.ceil(values.length * 0.5) - 1];
          row.p75Tokens = values[Math.ceil(values.length * 0.75) - 1];
          models.set(key, row);
        }
      const start =
        range.from ??
        Math.min(range.to - 1, ...items.map((item) => item.prompt.timestamp));
      const bucketSeconds = range.to - start <= 3600 ? 60 : 3600;
      const activity = Array.from(
        { length: Math.ceil((range.to - start) / bucketSeconds) },
        (_, i) => ({
          timestamp: start + i * bucketSeconds,
          tokens: 0,
          prompts: 0,
          requests: 0,
        }),
      );
      for (const item of selected) {
        const bucket =
          activity[Math.floor((item.prompt.timestamp - start) / bucketSeconds)];
        bucket.prompts++;
        bucket.requests += item.requests.length;
        bucket.tokens += totalTokens(item.tokens) ?? 0;
      }
      const samples = selected
        .map((item) => totalTokens(item.tokens) ?? 0)
        .sort((a, b) => a - b);
      const result: UsageMetrics = {
        range,
        bucketSeconds,
        activity,
        activePromptCount: selected.length,
        stats: {
          promptCount: selected.length,
          conversationCount: new Set(
            selected.map((item) => item.prompt.sessionId),
          ).size,
          requestCount: selected.reduce(
            (sum, item) => sum + item.requests.length,
            0,
          ),
          totalTokens: totalTokens(totals) ?? 0,
          tokenTotals: totals,
          medianTokens:
            samples[Math.max(0, Math.ceil(samples.length * 0.5) - 1)] ?? 0,
          p75Tokens:
            samples[Math.max(0, Math.ceil(samples.length * 0.75) - 1)] ?? 0,
          daily: [],
          modelStats: [...models.values()],
          computers: [...new Set(selected.map((item) => item.prompt.deviceId))],
          backgroundRequests: 0,
        },
      };
      return result as T;
    }
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
    case "get_signin_status":
      return [] as T;
    case "signin_input":
      return undefined as T;
    default:
      throw new Error(`Preview command unavailable: ${command}`);
  }
}
