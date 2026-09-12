<script lang="ts">
  import { onDestroy } from "svelte";
  import { api } from "./api";
  import AllowanceDuration from "./AllowanceDuration.svelte";
  import RecentUsageValue from "./RecentUsageValue.svelte";
  import { providerName } from "./format";
  import { count } from "./format";
  import type { AllowanceForecast, UsageInsights } from "./types";
  let {
    refreshKey = "",
    recent = null,
    recentMinutes = 60,
    recentPending = false,
    recentError = "",
    onDurationChange,
  }: {
    refreshKey?: string;
    recent?: import("./types").RecentAllowance | null;
    recentMinutes?: number;
    recentPending?: boolean;
    recentError?: string;
    onDurationChange: (minutes: number) => Promise<void>;
  } = $props();
  let days = $state(30);
  let data = $state<UsageInsights | null>(null);
  let busy = $state(false);
  let error = $state("");
  let chartMode = $state<"tokens" | "prompts">("tokens");
  // Keep at most one expensive database query running and one newest request.
  // Ignoring old responses alone would still queue every query in the backend.
  let requestRunning = false;
  let requestVersion = 0;
  let pendingRequest: { days: number; version: number } | null = null;
  let disposed = false;
  async function loadLatestInsights() {
    if (requestRunning || disposed) return;
    requestRunning = true;
    try {
      while (!disposed && pendingRequest) {
        const request = pendingRequest;
        pendingRequest = null;
        try {
          const result = await api.insights(request.days);
          if (!disposed && request.version === requestVersion) {
            data = result;
            error = "";
          }
        } catch (reason) {
          if (!disposed && request.version === requestVersion) {
            data = null;
            error = String(reason);
          }
        }
      }
    } finally {
      requestRunning = false;
      if (!disposed) busy = false;
    }
  }
  onDestroy(() => {
    disposed = true;
    pendingRequest = null;
    requestVersion += 1;
  });
  $effect(() => {
    const selectedDays = days;
    void refreshKey;
    pendingRequest = { days: selectedDays, version: ++requestVersion };
    busy = true;
    error = "";
    void loadLatestInsights();
    return () => {
      // Invalidate both queued work and results from the previous dependencies.
      pendingRequest = null;
      requestVersion += 1;
    };
  });
  const stats = $derived(data?.current.stats);
  const knownTotal = $derived(
    stats &&
      Object.entries(stats.tokenTotals).some(
        ([key, value]) => key !== "reasoning" && value != null,
      ),
  );
  const modelMix = $derived.by(() => {
    const groups = new Map<
      string,
      { label: string; provider: string; tokens: number; requests: number }
    >();
    for (const row of stats?.modelStats ?? []) {
      const key = row.provider + ":" + row.model;
      const value = groups.get(key) ?? {
        label: row.model,
        provider: row.provider,
        tokens: 0,
        requests: 0,
      };
      value.tokens += row.totalTokens;
      value.requests += row.requestCount;
      groups.set(key, value);
    }
    return [...groups.values()].sort(
      (a, b) => b.tokens - a.tokens || b.requests - a.requests,
    );
  });
  const inputTotal = $derived(
    stats
      ? (stats.tokenTotals.input ?? 0) +
          (stats.tokenTotals.cacheRead ?? 0) +
          (stats.tokenTotals.cacheWrite ?? 0)
      : 0,
  );
  const cacheShare = $derived(
    stats?.tokenTotals.cacheRead != null && inputTotal > 0
      ? (stats.tokenTotals.cacheRead / inputTotal) * 100
      : null,
  );
  const priorTokens = $derived(data?.previous.stats.totalTokens ?? 0);
  const change = $derived(
    knownTotal && priorTokens > 0
      ? (((stats?.totalTokens ?? 0) - priorTokens) / priorTokens) * 100
      : null,
  );
  const series = $derived(data?.current.activity ?? []);
  const maxValue = $derived(Math.max(1, ...series.map((p) => p[chartMode])));
  const trend = $derived(
    series
      .map(
        (p, i) =>
          28 +
          (i / Math.max(1, series.length - 1)) * 664 +
          "," +
          (164 - (p[chartMode] / maxValue) * 138),
      )
      .join(" "),
  );
  const heatMax = $derived(Math.max(1, ...(data?.hourlyPrompts ?? [])));
  const weekdays = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
  const date = (seconds: number) =>
    new Date(seconds * 1000).toLocaleDateString(undefined, {
      month: "short",
      day: "numeric",
    });
  const when = (seconds: number) =>
    new Date(seconds * 1000).toLocaleString(undefined, {
      month: "short",
      day: "numeric",
      hour: "numeric",
      minute: "2-digit",
    });
  function paceText(f: AllowanceForecast) {
    if (f.state === "before_reset" && f.exhaustsAt != null)
      return "May run out " + when(f.exhaustsAt);
    if (f.state === "after_reset" && f.remainingAtReset != null)
      return "~" + Math.round(f.remainingAtReset) + "% left at reset";
    if (f.state === "exhausted") return "Allowance exhausted";
    if (f.state === "steady") return "No meaningful recent consumption";
    if (f.state === "stale") return "Waiting for a current allowance reading";
    return "Learning your recent pace";
  }
  function forecastLines(f: AllowanceForecast) {
    const first = f.points[0]?.timestamp ?? 0;
    const last = f.points.at(-1);
    const end =
      f.remainingAtReset == null
        ? (last?.timestamp ?? first + 1)
        : (f.exhaustsAt ?? f.resetsAt ?? last?.timestamp ?? first + 1);
    const x = (t: number) =>
      16 + ((t - first) / Math.max(1, end - first)) * 368;
    const y = (r: number) => 100 - r * 0.78;
    return {
      observed: f.points
        .map((p) => x(p.timestamp) + "," + y(p.remaining))
        .join(" "),
      projected:
        last && f.remainingAtReset != null
          ? x(last.timestamp) +
            "," +
            y(last.remaining) +
            " " +
            x(end) +
            "," +
            y(f.exhaustsAt ? 0 : f.remainingAtReset)
          : "",
    };
  }
</script>

<section
  class="usage-insights"
  aria-label="Your AI usage insights"
  aria-busy={busy}
>
  <div class="insights-heading">
    <div>
      <p class="eyebrow">Your patterns, made visible</p>
      <h2>How you use AI</h2>
    </div>
    <div class="insight-periods" role="group" aria-label="Insights period">
      {#each [7, 30, 90] as range}<button
          class:selected={days === range}
          onclick={() => (days = range)}>{range} days</button
        >{/each}
    </div>
  </div>
  {#if error}<p class="insights-notice" role="alert">
      Could not load insights. {error}
    </p>{/if}
  {#if busy}<p class="fineprint" role="status">
      Updating your usage insights…
    </p>{/if}
  {#if data && stats}
    <div class:insights-pending={busy}>
      <div class="insight-stats">
        <article>
          <span>Measured tokens</span><strong
            >{knownTotal ? count(stats.totalTokens) : "—"}</strong
          ><small
            >{change == null
              ? "No comparable earlier token total"
              : (change >= 0 ? "+" : "") +
                change.toFixed(0) +
                "% vs previous " +
                data.days +
                " days"}</small
          >
        </article>
        <article>
          <span>Active days</span><strong>{data.activeDays}</strong><small
            >Longest streak · {data.longestStreak}
            {data.longestStreak === 1 ? "day" : "days"}</small
          >
        </article>
        <article>
          <span>Prompts per active day</span><strong
            >{data.activeDays
              ? (stats.promptCount / data.activeDays).toFixed(1)
              : "—"}</strong
          ><small>{stats.promptCount.toLocaleString()} prompts started</small>
        </article>
        <article>
          <span>Input reused from cache</span><strong
            >{cacheShare == null ? "—" : Math.round(cacheShare) + "%"}</strong
          ><small>Share of measured input tokens</small>
        </article>
      </div>
      <div class="insights-two">
        <article class="insight-panel">
          <div class="insight-panel-heading">
            <h3>Activity over time</h3>
            <div
              class="insight-periods"
              role="group"
              aria-label="Activity measure"
            >
              <button
                class:selected={chartMode === "tokens"}
                onclick={() => (chartMode = "tokens")}>Tokens</button
              >
              <button
                class:selected={chartMode === "prompts"}
                onclick={() => (chartMode = "prompts")}>Prompts</button
              >
            </div>
          </div>
          {#if chartMode === "prompts" || knownTotal}
            <svg
              class="insight-trend"
              viewBox="0 0 720 196"
              role="img"
              aria-label={chartMode + " across the last " + data.days + " days"}
            >
              <title>{chartMode} across the last {data.days} days</title>
              <path d="M28 26H692 M28 95H692 M28 164H692" class="chart-grid" />
              <polyline points={trend} class="chart-line" />
              {#each series as point, i}
                <circle
                  cx={28 + (i / Math.max(1, series.length - 1)) * 664}
                  cy={164 - (point[chartMode] / maxValue) * 138}
                  r="4"
                  class="chart-point"
                >
                  <title
                    >{when(point.timestamp)} · {point[
                      chartMode
                    ].toLocaleString()}
                    {chartMode}</title
                  >
                </circle>
              {/each}
              <text x="28" y="190"
                >{series[0] ? date(series[0].timestamp) : ""}</text
              >
              <text x="692" y="190" text-anchor="end"
                >{date(data.current.range.to)}</text
              >
              <text x="28" y="17">{count(maxValue)}</text>
            </svg>
          {:else}<p class="insights-notice">
              Token totals are unavailable for this period.
            </p>{/if}
          <p class="fineprint">
            Hover a point for its value. Each point covers a recorded time
            bucket.
          </p>
        </article>
        <article class="insight-panel">
          <div class="insight-panel-heading">
            <h3>Your model mix</h3>
            <span class="fineprint"
              >{knownTotal ? "Measured tokens" : "Model requests"}</span
            >
          </div>
          <div class="model-mix">
            {#each modelMix.slice(0, 8) as model}
              {@const total = knownTotal
                ? stats.totalTokens
                : stats.requestCount}
              {@const value = knownTotal ? model.tokens : model.requests}
              <div class="mix-row">
                <div>
                  <span title={model.provider + " / " + model.label}
                    >{model.label}</span
                  ><strong>{count(value)}</strong>
                </div>
                <div class="mix-track">
                  <span
                    style:width={(total > 0 ? (value / total) * 100 : 0) + "%"}
                  ></span>
                </div>
                <small
                  >{total > 0 ? Math.round((value / total) * 100) : 0}% of {knownTotal
                    ? "measured tokens"
                    : "requests"} · {model.requests.toLocaleString()} requests</small
                >
              </div>
            {:else}<p class="fineprint">
                Your model mix will appear after activity is recorded.
              </p>{/each}
          </div>
          {#if modelMix.length > 8}<p class="fineprint">
              Showing the eight busiest models.
            </p>{/if}
        </article>
      </div>
      <article class="insight-panel hours-panel">
        <div class="insight-panel-heading">
          <h3>When you reach for AI</h3>
          <span class="fineprint">Prompts started · local time</span>
        </div>
        <div class="hours-grid">
          <span
          ></span>{#each Array.from({ length: 24 }, (_, h) => h) as hour}<span
              class="hour-tick">{hour % 6 === 0 ? hour + ":00" : ""}</span
            >{/each}
          {#each weekdays as day, d}
            <span class="day-tick">{day}</span>
            {#each Array.from({ length: 24 }, (_, h) => h) as hour}
              {@const value = data.hourlyPrompts[d * 24 + hour] ?? 0}
              <div
                class="heat-cell"
                style:background={value > 0
                  ? "color-mix(in srgb,var(--accent) " +
                    (20 + (value / heatMax) * 80) +
                    "%,var(--surface))"
                  : "var(--surface)"}
                role="img"
                aria-label={day + " " + hour + ":00 · " + value + " prompts"}
                title={day +
                  " " +
                  hour +
                  ":00–" +
                  (hour + 1) +
                  ":00 · " +
                  value +
                  " prompts"}
              ></div>
            {/each}
          {/each}
        </div>
        <p class="fineprint">
          Brighter cells mean more prompts. Your busiest hours describe
          activity, not productivity.
        </p>
      </article>
      <div class="insight-observations">
        <p>
          <strong
            >{data.current.activePromptCount
              ? (
                  (stats.requestCount - stats.backgroundRequests) /
                  data.current.activePromptCount
                ).toFixed(1)
              : "—"}</strong
          > model requests per active user prompt
        </p>
        <p>
          <strong
            >{stats.requestCount
              ? Math.round(
                  (stats.backgroundRequests / stats.requestCount) * 100,
                ) + "%"
              : "—"}</strong
          > background or review requests
        </p>
        <p>
          <strong>{stats.conversationCount.toLocaleString()}</strong> conversations
          with recorded activity
        </p>
      </div>
      {#if data.unknownRequests > 0}<p class="fineprint">
          {data.unknownRequests.toLocaleString()} requests have no token measurement.
          Charts show measured totals only.
        </p>{/if}
      <section
        class="recent-allowance-section"
        aria-labelledby="recent-allowance-title"
      >
        <h2 id="recent-allowance-title">Allowance used recently</h2>
        <AllowanceDuration
          minutes={recentMinutes}
          onchange={onDurationChange}
        />
        {#if recentError}<p class="field-error" role="alert">
            Recent usage unavailable. {recentError}
          </p>{/if}
        <div class="recent-allowance-grid">
          {#each recent?.windows ?? [] as row}
            <article class="insight-panel">
              <h3>{providerName(row.provider)} / {row.label}</h3>
              <RecentUsageValue
                value={row}
                minutes={recentMinutes}
                loading={recentPending}
              />
            </article>
          {:else}<p class="fineprint">
              {recentPending
                ? "Updating recent usage…"
                : "Comparable provider readings will appear here as they are recorded."}
            </p>{/each}
        </div>
        <p class="fineprint">
          Each percentage belongs to its own allowance window. 91% to 87% left
          means 4 percentage points used; the windows are not added together.
        </p>
      </section>
      <div class="forecast-heading">
        <h2>Will your allowance last?</h2>
        <p>
          Projection from recent provider allowance readings. It changes as your
          pace changes.
        </p>
      </div>
      <div class="allowance-forecasts">
        {#each data.forecasts as forecast}
          {@const lines = forecastLines(forecast)}
          <article
            class="insight-panel allowance-forecast"
            class:forecast-risk={forecast.state === "before_reset" ||
              forecast.state === "exhausted"}
          >
            <div class="insight-panel-heading">
              <h3>{forecast.label}</h3>
              <span
                >{forecast.remaining == null
                  ? "—"
                  : Math.round(forecast.remaining) + "% left"}</span
              >
            </div>
            <strong class="forecast-verdict">{paceText(forecast)}</strong>
            {#if forecast.points.length > 1}
              <svg
                viewBox="0 0 400 126"
                role="img"
                aria-label={forecast.label +
                  ": observed allowance and estimated remaining allowance"}
              >
                <title
                  >Solid line: recorded allowance. Dashed line: projected
                  allowance.</title
                >
                <path d="M16 22H384 M16 100H384" class="chart-grid" />
                <text x="16" y="15">100%</text><text x="16" y="117">0%</text>
                <polyline points={lines.observed} class="chart-line" />
                {#if lines.projected}<polyline
                    points={lines.projected}
                    class="chart-line projection"
                  />{/if}
              </svg>
            {/if}
            <p class="fineprint">
              {forecast.ratePerHour == null
                ? "Needs at least four current readings spanning 30 minutes without long gaps."
                : forecast.ratePerHour.toFixed(2) +
                  " percentage points per hour · " +
                  forecast.sampleCount +
                  " readings over " +
                  (forecast.observedSeconds / 3600).toFixed(1) +
                  "h"}
            </p>
            {#if forecast.resetsAt}<p class="fineprint reset-time">
                Reset · {when(forecast.resetsAt)}
              </p>{/if}
          </article>
        {:else}<article class="insight-panel">
            <h3>Waiting for allowance readings</h3>
            <p class="fineprint">
              Connect your coding tools. The forecast learns from real allowance
              changes, independently of measured tokens.
            </p>
          </article>{/each}
      </div>
      <p class="fineprint forecast-footnote">
        Solid lines are recorded readings; dashed lines are projections until
        reset or exhaustion. Forecasts pause after stale readings, resets,
        account changes or long gaps.
      </p>
    </div>
  {/if}
</section>
