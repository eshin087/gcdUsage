<script lang="ts">
  import {
    localDateTime,
    rangeDescription,
    resolveRange,
    timePresets,
    type TimePreset,
  } from "./lib/time-range";
  import { onMount } from "svelte";
  import { version } from "../package.json";
  import { listen } from "@tauri-apps/api/event";
  import { api, native, preview } from "./lib/api";
  import {
    buildHistoryFilter,
    count,
    dateTime,
    uniqueModelStats,
    emptyTokens,
    exactCount,
    providerName,
    relativeTime,
    totalTokens,
  } from "./lib/format";
  import type { FilterInputs } from "./lib/format";
  import type {
    AppSettings,
    UsageMetrics,
    MetricsRange,
    HistoryItem,
    HistoryPage,
    Overview,
    Provider,
    RecommendationSet,
    TaskClass,
  } from "./lib/types";
  import Icon from "./lib/Icon.svelte";
  import QuotaCard from "./lib/QuotaCard.svelte";
  import ProAllowance from "./lib/ProAllowance.svelte";
  import TokenChart from "./lib/TokenChart.svelte";
  import DailyChart from "./lib/DailyChart.svelte";
  import SignInPanel from "./lib/SignInPanel.svelte";
  import PromptDetails, {type PromptDetail} from './lib/PromptDetails.svelte';
  let selectedPrompt=$state<PromptDetail|null>(null);
  async function openPrompt(id:string){try{const detail=await api.promptDetail(id);if(detail.originalUrl){try{if(await api.openOriginal(id))return}catch{}}selectedPrompt=detail;}catch(e){error=String(e)}}
  async function takePrompt(){const id=await api.pendingPrompt();if(id)await openPrompt(id)}

  type Page =
    "overview" | "history" | "recommendations" | "settings";
  const pages: { id: Page; label: string }[] = [
    { id: "overview", label: "Overview" },
    { id: "history", label: "History" },
    { id: "recommendations", label: "Model advice" },
    { id: "settings", label: "Settings" },
  ];
  const isWindows = /Windows/i.test(navigator.userAgent);
  let page = $state<Page>("overview");
  let overview = $state<Overview | null>(null);
  let history = $state<HistoryPage>({ items: [], total: 0 });
  let advice = $state<RecommendationSet>({ recommendations: [], summary: "" });
  let settings = $state<AppSettings | null>(null);
  let dirty = $state(false);
  let now = $state(Date.now() / 1000);
  let ready = $state(false);
  let loading = $state(true);
  let historyLoading = $state(false);
  let adviceLoading = $state(false);
  let busy = $state("");
  let error = $state("");
  let notice = $state("");
  let useSso = $state(false);
  let expanded = $state<string | null>(null);
  let pageIndex = $state(0);
  let task = $state<TaskClass>("everyday");
  let filters = $state<FilterInputs>({
    query: "",
    provider: "",
    model: "",
    effort: "",
    deviceId: "",
    from: "",
    to: "",
  });
  let metrics = $state<UsageMetrics | null>(null);
  let metricsLoading = $state(true);
  let metricsError = $state("");
  let timePreset = $state<TimePreset>("24h");
  let customFrom = $state(localDateTime(Math.floor(Date.now() / 1000) - 86400));
  let customTo = $state(localDateTime(Math.floor(Date.now() / 1000)));
  let metricsClock = $state(Math.floor(Date.now() / 1000));
  let metricsRequest = 0;
  let historyRequest = 0;
  let adviceRequest = 0;
  let lastFilter = "";
  let disposed = false;
  const snapshots = $derived(overview?.snapshots ?? []);
  const allStats = $derived(overview?.stats);
  const stats = $derived(metrics?.stats);
  const displayModels = $derived(uniqueModelStats(stats?.modelStats ?? []));
  const modelOptions = $derived(
    [
      ...new Set(
        (allStats?.modelStats ?? [])
          .filter(
            (stat) => !filters.provider || stat.provider === filters.provider,
          )
          .map((stat) => stat.model),
      ),
    ].sort(),
  );
  const effortOptions = $derived(
    [
      ...new Set(
        (allStats?.modelStats ?? []).map((stat) => stat.effort ?? "unknown"),
      ),
    ].sort(),
  );
  const activeFilter = $derived(buildHistoryFilter(filters, pageIndex));
  const dateError = $derived(
    !!filters.from && !!filters.to && filters.from > filters.to,
  );
  const pageTokens = $derived(
    history.items.reduce(
      (sum, item) => sum + (totalTokens(item.tokens) ?? 0),
      0,
    ),
  );
  const pageTokenTotals = $derived(
    history.items.reduce(
      (sum, item) => ({
        input:
          sum.input == null && item.tokens.input == null
            ? null
            : (sum.input ?? 0) + (item.tokens.input ?? 0),
        cacheRead:
          sum.cacheRead == null && item.tokens.cacheRead == null
            ? null
            : (sum.cacheRead ?? 0) + (item.tokens.cacheRead ?? 0),
        cacheWrite:
          sum.cacheWrite == null && item.tokens.cacheWrite == null
            ? null
            : (sum.cacheWrite ?? 0) + (item.tokens.cacheWrite ?? 0),
        output:
          sum.output == null && item.tokens.output == null
            ? null
            : (sum.output ?? 0) + (item.tokens.output ?? 0),
        reasoning:
          sum.reasoning == null && item.tokens.reasoning == null
            ? null
            : (sum.reasoning ?? 0) + (item.tokens.reasoning ?? 0),
      }),
      emptyTokens(),
    ),
  );
  const lastReading = $derived(
    snapshots.length
      ? Math.max(...snapshots.map((snapshot) => snapshot.fetchedAt))
      : null,
  );
  const machineName = (id: string) =>
    id === overview?.settings.deviceId ? overview.settings.deviceName : id;
  const modelsFor = (item: HistoryItem) =>
    [...new Set(item.requests.map((request) => request.model))].join(", ") ||
    "Unknown model";
  const effortsFor = (item: HistoryItem) =>
    [
      ...new Set(item.requests.map((request) => request.effort ?? "unknown")),
    ].join(", ") || "unknown";
  const statusName = (value: string | undefined) =>
    ({
      connected: "Connected",
      needs_auth: "Sign in needed",
      unavailable: "Not connected",
      error: "Connection error",
    })[value ?? "unavailable"] ?? "Not connected";
  const failure = (reason: unknown) =>
    reason instanceof Error ? reason.message : String(reason);

  async function loadOverview() {
    try {
      const result = await api.overview();
      if (disposed) return;
      overview = result;
      if (!dirty) settings = { ...result.settings };
    } catch (reason) {
      if (!disposed) error = failure(reason);
    } finally {
      if (!disposed) loading = false;
    }
  }
  async function loadMetrics(range: MetricsRange, request: number) {
    try {
      const result = await api.metrics(range);
      if (!disposed && request === metricsRequest) metrics = result;
    } catch (reason) {
      if (!disposed && request === metricsRequest) {
        metricsError = failure(reason);
        metrics = null;
      }
    } finally {
      if (!disposed && request === metricsRequest) metricsLoading = false;
    }
  }
  function exploreInterval() {
    if (!metrics) return;
    filters.from =
      metrics.range.from == null ? "" : localDateTime(metrics.range.from);
    filters.to = localDateTime(metrics.range.to - 1);
    page = "history";
  }
  async function loadHistory() {
    if (dateError) return;
    const request = ++historyRequest;
    historyLoading = true;
    try {
      const result = await api.history(activeFilter);
      if (!disposed && request === historyRequest) {
        history = result;
        expanded = null;
      }
    } catch (reason) {
      if (!disposed && request === historyRequest) error = failure(reason);
    } finally {
      if (!disposed && request === historyRequest) historyLoading = false;
    }
  }
  async function loadAdvice() {
    const request = ++adviceRequest;
    adviceLoading = true;
    try {
      const result = await api.recommendations(task);
      if (!disposed && request === adviceRequest) advice = result;
    } catch (reason) {
      if (!disposed && request === adviceRequest) error = failure(reason);
    } finally {
      if (!disposed && request === adviceRequest) adviceLoading = false;
    }
  }
  async function perform(
    label: string,
    work: () => Promise<unknown>,
    success: string,
  ) {
    if (busy) return;
    busy = label;
    error = "";
    notice = "";
    try {
      await work();
      if (success) notice = success;
      await loadOverview();
    } catch (reason) {
      error = failure(reason);
    } finally {
      busy = "";
    }
  }
  const refresh = () => perform("refresh", api.refresh, "Refresh requested.");
  const reconnect = (provider: Provider) =>
    perform(
      `reconnect-${provider}`,
      () => api.reconnect(provider, provider === "claude" && useSso),
      "Checking the connection. Follow the sign-in status below.",
    );
  const importHistory = () =>
    perform(
      "import",
      api.importHistory,
      "History import requested. You can keep using the app.",
    );
  async function chooseFolder() {
    if (!settings) return;
    try {
      const folder = await api.selectSyncFolder();
      if (folder) {
        settings.syncFolder = folder;
        dirty = true;
      }
    } catch (reason) {
      error = failure(reason);
    }
  }
  async function saveSettings(completeSetup = false) {
    if (
      !settings ||
      !settings.deviceName.trim() ||
      !Number.isFinite(settings.reservePercent) ||
      settings.reservePercent < 0 ||
      settings.reservePercent > 50
    ) {
      error = "Enter a computer name and a reserve between 0% and 50%.";
      return;
    }
    await perform(
      "save",
      async () => {
        settings = await api.saveSettings({
          ...settings!,
          deviceName: settings!.deviceName.trim(),
          setupComplete: settings!.setupComplete || completeSetup,
        });
        dirty = false;
      },
      completeSetup
        ? "All set. Your meters keep running when you close this window."
        : "Settings saved.",
    );
    if (completeSetup && !error) page = "overview";
  }
  const exportHistory = () =>
    perform(
      "export",
      async () => {
        const path = await api.exportHistory(activeFilter);
        if (path) notice = `Exported to ${path}`;
      },
      "",
    );
  function resetFilters() {
    filters = {
      query: "",
      provider: "",
      model: "",
      effort: "",
      deviceId: "",
      from: "",
      to: "",
    };
    pageIndex = 0;
  }

  $effect(() => {
    const signature = JSON.stringify(filters);
    if (signature !== lastFilter) {
      lastFilter = signature;
      pageIndex = 0;
    }
  });
  $effect(() => {
    const filter = JSON.stringify(activeFilter);
    if (!ready || page !== "history" || dateError) return;
    void filter;
    const timer = setTimeout(() => void loadHistory(), 200);
    return () => clearTimeout(timer);
  });
  $effect(() => {
    const selection = task;
    if (ready && page === "recommendations") {
      void selection;
      void loadAdvice();
    }
  });
  $effect(() => {
    void page;
    window.scrollTo({ top: 0, left: 0 });
  });
  $effect(() => {
    document.documentElement.style.setProperty(
      "--font-scale",
      String((settings?.fontScale ?? 120) / 100),
    );
  });
  $effect(() => {
    const selection = timePreset;
    const from = customFrom;
    const to = customTo;
    const clock = metricsClock;
    if (!ready || page !== "overview") return;
    const request = ++metricsRequest;
    metricsLoading = true;
    metricsError = "";
    metrics = null;
    let timer: ReturnType<typeof setTimeout> | undefined;
    try {
      const range = resolveRange(selection, clock, from, to);
      try {
        localStorage.setItem(
          "gcd-metrics-range",
          JSON.stringify({ preset: selection, from, to }),
        );
      } catch {
        /* Storage may be unavailable. */
      }
      timer = setTimeout(() => void loadMetrics(range, request), 180);
    } catch (reason) {
      metricsError = failure(reason);
      metricsLoading = false;
    }
    return () => {
      clearTimeout(timer);
      ++metricsRequest;
    };
  });
  $effect(() => {
    const theme = settings?.theme ?? "black";
    const system = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      document.documentElement.dataset.theme =
        theme === "system" ? (system.matches ? "slate" : "light") : theme;
    };
    apply();
    if (theme === "system") {
      system.addEventListener("change", apply);
      return () => system.removeEventListener("change", apply);
    }
  });
  onMount(() => {
    disposed = false;
    try {
      const saved = JSON.parse(
        localStorage.getItem("gcd-metrics-range") ?? "null",
      );
      if (saved && timePresets.some(([key]) => key === saved.preset)) {
        timePreset = saved.preset;
        if (typeof saved.from === "string") customFrom = saved.from;
        if (typeof saved.to === "string") customTo = saved.to;
      }
    } catch {
      /* Use the default interval. */
    }
    ready = true;
    void loadOverview();
    const tick = setInterval(() => {
      now = Date.now() / 1000;
    }, 1000);
    const poll = setInterval(() => {
      if (native || preview) void loadOverview();
      metricsClock = Math.floor(Date.now() / 1000);
    }, 30000);
    const cleanups: (() => void)[] = [];
    if (native) {
      void listen('prompt-selected',()=>{void takePrompt()}).then(unlisten=>{if(disposed)unlisten();else{cleanups.push(unlisten);void takePrompt()}});
      for (const event of [
        "usage-updated",
        "history-updated",
        "settings-updated",
      ]) {
        void listen(event, () => {
          void loadOverview();
          if (event === "history-updated")
            metricsClock = Math.floor(Date.now() / 1000);
          if (page === "history" && event === "history-updated")
            void loadHistory();
          if (page === "recommendations") void loadAdvice();
        }).then((unlisten) =>
          disposed ? unlisten() : cleanups.push(unlisten),
        );
      }
    }
    return () => {
      disposed = true;
      clearInterval(tick);
      clearInterval(poll);
      cleanups.forEach((cleanup) => cleanup());
    };
  });
</script>

<div class="app-shell">
  <aside class="sidebar" aria-label="Main navigation">
    <div class="brand">
      <img class="brand-mark" src="/gcd-logo.png" alt="GCD"/><span>GCD<span class="brand-light">Usage</span></span>
    </div>
    <div class="workspace-label">[ AI USAGE / LOCAL NODE ]</div>
    <nav>
      {#each pages as item}<button
          class:active={page === item.id}
          aria-label={item.label}
          aria-current={page === item.id ? "page" : undefined}
          onclick={() => (page = item.id)}
          ><Icon name={item.id} /><span
            >{item.label}</span
          >{#if item.id === "history" && allStats?.promptCount}<span
              class="nav-count">{count(allStats.promptCount)}</span
            >{/if}</button
        >{/each}
    </nav>
    <div class="sidebar-bottom">
      <div class="local-note">
        <Icon name="shield" size={16} /><span>Local by default</span>
      </div>
      <div class="device-label">
        <Icon name="computer" size={15} /><span
          >{overview?.settings.deviceName ?? "This computer"}</span
        >
      </div>
      <span class="version">Version {version}</span>
    </div>
  </aside>
  <main>
    <header class="page-header">
      <div>
        <div class="eyebrow">
          {page === "overview"
            ? "SYS / TELEMETRY"
            : page === "history"
              ? "LOG / REQUEST STREAM"
              : page === "recommendations"
                ? "ROUTE / MODEL ADVICE"
                : "CONFIG / LOCAL NODE"}
        </div>
        <h1>
          {page === "recommendations"
            ? "Model routing"
            : page === "overview"
              ? "Usage terminal"
              : page === "history"
                ? "Prompt history"
                : "Settings"}
        </h1>
        <p>
          {page === "overview"
            ? "[ live allowance / measured activity / local intelligence ]"
            : page === "history"
              ? "Measured tokens from Claude Code and local Codex activity."
              : page === "recommendations"
                ? "Local suggestions informed by your allowance and recent work."
                : "Connections, display and shared history."}
        </p>
      </div>
      <button
        class="button refresh-button"
        onclick={refresh}
        disabled={!!busy || (!native && !preview)}
        ><span class:spinning={busy === "refresh"}
          ><Icon name="refresh" size={15} /></span
        >{busy === "refresh" ? "Refreshing…" : "Refresh"}</button
      >
    </header>
    {#if preview}<div class="banner preview-banner">
        <Icon name="info" size={17} /><span
          ><strong>Design preview</strong> · Sample data only. No accounts are connected.</span
        >
      </div>{/if}
    {#if !native && !preview}<div class="banner">
        <Icon name="info" size={17} /><span
          >This dashboard connects to your usage inside the GCD Usage desktop
          app.</span
        >
      </div>{/if}
    {#if error}<div class="banner error-banner" role="alert">
        <Icon name="info" size={17} /><span>{error}</span><button
          class="icon-button"
          aria-label="Dismiss error"
          onclick={() => (error = "")}><Icon name="close" size={16} /></button
        >
      </div>{/if}
    {#if notice}<div class="banner success-banner" role="status">
        <Icon name="check" size={17} /><span>{notice}</span><button
          class="icon-button"
          aria-label="Dismiss message"
          onclick={() => (notice = "")}><Icon name="close" size={16} /></button
        >
      </div>{/if}

    <SignInPanel />
    {#if page === "overview"}
      {#if overview && !overview.settings.setupComplete}<div
          class="setup-callout"
        >
          <div class="setup-icon">
            <Icon name="recommendations" size={24} />
          </div>
          <div>
            <strong>Make it yours in a minute.</strong>
            <p>
              Check your connections, choose a sync folder, and you’re ready.
            </p>
          </div>
          <button class="button primary" onclick={() => (page = "settings")}
            >Finish setup <Icon name="arrow" size={15} /></button
          >
        </div>{/if}
      <section class="quota-grid" aria-label="Current usage limits">
        <QuotaCard
          provider="claude"
          period="five_hour"
          snapshot={snapshots.find(
            (snapshot) => snapshot.provider === "claude",
          )}
          {now}
          display={settings?.meterDisplay ?? "remaining"}
          {reconnect}
        /><QuotaCard
          provider="claude"
          period="weekly"
          snapshot={snapshots.find(
            (snapshot) => snapshot.provider === "claude",
          )}
          {now}
          display={settings?.meterDisplay ?? "remaining"}
          {reconnect}
        /><QuotaCard
          provider="claude"
          period="fable"
          snapshot={snapshots.find((snapshot) => snapshot.provider === "claude")}
          {now}
          display={settings?.meterDisplay ?? "remaining"}
          {reconnect}
        /><QuotaCard
          provider="codex"
          period="weekly"
          snapshot={snapshots.find((snapshot) => snapshot.provider === "codex")}
          {now}
          display={settings?.meterDisplay ?? "remaining"}
          {reconnect}
        />
      </section>
      <ProAllowance />
      <div class="reading-meta">
        <span
          ><span class="status-dot"></span>Updated {relativeTime(
            lastReading,
            now,
          ).toLowerCase()} · Refreshes every 2 minutes</span
        ><span>Codex / ChatGPT Work allowance</span>
      </div>
      <section class="section-heading">
        <div>
          <h2>Activity telemetry</h2>
          <p>
            {timePresets.find(([key]) => key === timePreset)?.[1]} · {stats
              ?.computers.length ?? 0} computers
          </p>
        </div>
        <button
          class="text-button"
          disabled={!metrics || metricsLoading}
          onclick={exploreInterval}
          >Explore history <Icon name="arrow" size={15} /></button
        >
      </section>
      <section class="interval-controls" aria-label="Statistics interval">
        <label
          >Time range<select
            aria-label="Statistics time range"
            bind:value={timePreset}
          >
            {#each timePresets as [value, label]}<option {value}>{label}</option
              >{/each}
          </select></label
        >
        {#if timePreset === "custom"}
          <label
            >Start<input
              aria-label="Statistics start"
              type="datetime-local"
              step="1"
              bind:value={customFrom}
            /></label
          >
          <label
            >End<input
              aria-label="Statistics end"
              type="datetime-local"
              step="1"
              bind:value={customTo}
            /></label
          >
        {/if}
        <span class="interval-status" role="status"
          >{metricsLoading
            ? "Updating statistics…"
            : metricsError
              ? ""
              : metrics
                ? rangeDescription(metrics.range)
                : ""}</span
        >
      </section>
      <p class="fineprint interval-note">
        Local time · Weeks start Monday. Tokens and requests use their recorded
        time; the end time is exclusive. Quota meters above always show the
        latest allowance.
      </p>
      {#if metricsError}<p class="field-error" role="alert">
          {metricsError}
        </p>{/if}
      <div class="stats-grid" aria-busy={metricsLoading}>
        <div class="stat-item">
          <span>Prompts started</span><strong
            >{metricsLoading || !stats
              ? "—"
              : count(stats?.promptCount ?? 0)}</strong
          ><small>{count(stats?.conversationCount ?? 0)} conversations</small>
        </div>
        <div class="stat-item">
          <span>Measured tokens</span><strong
            >{metricsLoading || !stats
              ? "—"
              : count(stats?.totalTokens ?? 0)}</strong
          ><small>Includes input, caches, and output</small>
        </div>
        <div class="stat-item">
          <span>Tokens per active prompt</span><strong
            >{metricsLoading || !stats || !metrics?.activePromptCount
              ? "—"
              : count(stats.medianTokens)}</strong
          ><small
            >Median · p75 {count(
              metrics?.activePromptCount ? stats?.p75Tokens : null,
            )}</small
          >
        </div>
        <div class="stat-item">
          <span>Model requests</span><strong
            >{metricsLoading || !stats
              ? "—"
              : count(stats?.requestCount ?? 0)}</strong
          ><small
            >{count(stats?.backgroundRequests ?? 0)} background requests</small
          >
        </div>
      </div>
      <p class="fineprint interval-note">
        {count(metrics?.activePromptCount)} active user prompts, including work started
        earlier. Per-prompt figures count only tokens recorded inside this interval.
        Missing logs can leave incomplete totals.
      </p>
      <div class="charts-grid">
        <section class="panel">
          <div class="panel-heading">
            <h2>Activity over time</h2>
            <span class="subtle">Measured tokens</span>
          </div>
          <DailyChart
            activity={metrics?.activity ?? []}
            bucketSeconds={metrics?.bucketSeconds ?? 3600}
          />
        </section>
        <section class="panel">
          <div class="panel-heading">
            <h2>Where tokens go</h2>
            <span class="subtle">Selected interval</span>
          </div>
          <TokenChart tokens={stats?.tokenTotals ?? emptyTokens()} />
        </section>
      </div>
      {#if stats?.modelStats.length}<section class="panel model-panel">
          <div class="panel-heading">
            <h2>Models and reasoning</h2>
            <span class="subtle">Measured consumption, not model quality</span>
          </div>
          <div class="table-scroll">
            <table>
              <thead
                ><tr
                  ><th>Model</th><th>Reasoning</th><th class="numeric"
                    >Active prompts</th
                  ><th class="numeric">Requests</th><th class="numeric"
                    >Tokens</th
                  ><th class="numeric">Median</th><th class="numeric">p75</th
                  ></tr
                ></thead
              ><tbody
                >{#each displayModels as model}<tr
                    ><td
                      ><span class={`provider-dot ${model.provider}`}
                      ></span>{model.model}</td
                    ><td
                      ><span class="effort-chip"
                        >{model.effort ?? "unknown"}</span
                      ></td
                    ><td class="numeric">{count(model.promptCount)}</td><td
                      class="numeric">{count(model.requestCount)}</td
                    ><td class="numeric">{count(model.totalTokens)}</td><td
                      class="numeric"
                      >{count(
                        model.promptCount ? model.medianTokens : null,
                      )}</td
                    ><td class="numeric"
                      >{count(model.promptCount ? model.p75Tokens : null)}</td
                    ></tr
                  >{/each}</tbody
              >
            </table>
          </div>
        </section>{/if}
      {#if allStats?.quotaAllocations?.length}
        <section class="panel model-panel" aria-label="Quota accounting">
          <div class="panel-heading">
            <h2>Quota accounting</h2>
            <span class="subtle">All saved readings · all time</span>
          </div>
          <p class="fineprint">
            Remaining before − remaining after = observed consumption in percentage points. For example, 82% → 80% left means 2 pp consumed. Only isolated, completed prompts receive an estimate; overlapping or unseen work stays unallocated. Resets and missing readings are excluded.
          </p>
          <div class="table-scroll">
            <table>
              <thead
                ><tr
                  ><th>Allowance</th><th class="numeric">Observed</th><th
                    class="numeric">Attributed</th
                  ><th class="numeric">Unallocated</th><th class="numeric"
                    >Gaps</th
                  ></tr
                ></thead
              >
              <tbody
                >{#each allStats.quotaAllocations as allocation}<tr>
                    <td
                      >{providerName(allocation.provider)}
                      <span class="subtle"
                        >{allocation.windowId
                          .replace(allocation.provider + ":", "")
                          .replace("10080", "weekly")
                          .replace("300", "5-hour")}</span
                      ><small class="subtle">
                        · {allocation.accountId.slice(-6)}</small
                      ></td
                    >
                    <td class="numeric"
                      >{allocation.observedPercent == null
                        ? "—"
                        : allocation.observedPercent.toFixed(1) + " pp"}</td
                    >
                    <td class="numeric"
                      >{allocation.allocatedPercent == null
                        ? "—"
                        : allocation.allocatedPercent.toFixed(1) + " pp"}</td
                    >
                    <td class="numeric"
                      >{allocation.unallocatedPercent == null
                        ? "—"
                        : allocation.unallocatedPercent.toFixed(1) + " pp"}</td
                    >
                    <td class="numeric">{count(allocation.gapCount)}</td>
                  </tr>{/each}</tbody
              >
            </table>
          </div>
        </section>
      {/if}
      <footer class="page-footer">
        <span
          ><Icon name="shield" size={14} />Your credentials stay on this
          computer.</span
        ><span
          >{overview?.importing
            ? "Importing history…"
            : overview?.settings.syncFolder
              ? `Last synced ${relativeTime(overview.lastSync, now).toLowerCase()}`
              : "Folder sync is off"}</span
        >
      </footer>
    {:else if page === "history"}
      <div class="history-toolbar">
        <div class="search-field">
          <Icon name="search" size={17} /><input
            aria-label="Search prompt previews"
            placeholder="Search your prompts…"
            bind:value={filters.query}
          />{#if filters.query}<button
              class="icon-button"
              onclick={() => (filters.query = "")}
              aria-label="Clear search"><Icon name="close" size={14} /></button
            >{/if}
        </div>
        <button
          class="button"
          onclick={importHistory}
          disabled={!!busy || overview?.importing}
          >{overview?.importing || busy === "import"
            ? "Importing…"
            : "Import history"}</button
        ><button
          class="button"
          onclick={exportHistory}
          disabled={!!busy || dateError}
          ><Icon name="download" size={15} />Export CSV</button
        >
      </div>
      <div class="filters">
        <label
          >Provider<select
            bind:value={filters.provider}
            onchange={() => (filters.model = "")}
            ><option value="">All providers</option><option value="claude"
              >Claude</option
            ><option value="codex">Codex</option></select
          ></label
        ><label
          >Model<select bind:value={filters.model}
            ><option value="">All models</option
            >{#each modelOptions as model}<option value={model}>{model}</option
              >{/each}</select
          ></label
        ><label
          >Reasoning<select bind:value={filters.effort}
            ><option value="">All levels</option
            >{#each effortOptions as effort}<option value={effort}
                >{effort}</option
              >{/each}</select
          ></label
        ><label
          >Computer<select bind:value={filters.deviceId}
            ><option value="">All computers</option
            >{#each allStats?.computers ?? [] as device}<option value={device}
                >{machineName(device)}</option
              >{/each}</select
          ></label
        ><label
          >From<input
            type="datetime-local"
            step="1"
            bind:value={filters.from}
            max={filters.to || undefined}
          /></label
        ><label
          >To<input
            type="datetime-local"
            step="1"
            bind:value={filters.to}
            min={filters.from || undefined}
          /></label
        >
      </div>
      {#if dateError}<p class="field-error" role="alert">
          The end date must be on or after the start date.
        </p>{/if}
      <div class="history-count">
        <span
          >{exactCount(history.total)} matching prompts
          <span class="subtle"
            >· {count(pageTokens)} measured tokens on this page</span
          ></span
        ><button class="text-button" onclick={resetFilters}
          >Clear filters</button
        >
      </div>
      <section class="panel history-panel" aria-busy={historyLoading}>
        <div class="table-scroll">
          <table class="history-table">
            <thead
              ><tr
                ><th>Prompt</th><th>Model / effort</th><th class="numeric"
                  >Tokens</th
                ><th class="numeric">Quota impact</th><th
                  ><span class="sr-only">Details</span></th
                ></tr
              ></thead
            ><tbody
              >{#each history.items as item (item.prompt.id)}<tr
                  class:expanded={expanded === item.prompt.id}
                  ><td
                    ><div class="log-line"><span class={`provider-dot ${item.prompt.provider}`}></span><time title={dateTime(item.prompt.timestamp)}>{new Date(item.prompt.timestamp * 1000).toLocaleString([], {month:'short', day:'numeric', hour:'2-digit', minute:'2-digit'})}</time><span class="log-context" title={`${item.prompt.project || 'Project unknown'} / ${item.prompt.chatTitle || item.prompt.sessionId}`}>{item.prompt.project || 'Unknown project'}</span><button class="prompt-preview prompt-link" title={item.prompt.preview} onclick={()=>openPrompt(`local:${item.prompt.id}`)}>{item.prompt.preview || '(No prompt preview)'}</button></div></td
                  ><td
                    ><span class="model-name" title={`${modelsFor(item)} / ${effortsFor(item)}`}>{modelsFor(item)}</span><span
                      class="effort-chip">{effortsFor(item)}</span
                    ></td
                  ><td class="numeric"
                    ><strong>{count(totalTokens(item.tokens))}</strong><span
                      class="cell-note">{item.requests.length} requests</span
                    ></td
                  ><td class="numeric"
                    >{#if item.quotaEstimate}<span class="estimate"
                        >~{item.quotaEstimate.percent.toFixed(2)}%</span
                      ><span class="cell-note">estimated</span>{:else}<span
                        class="subtle">—</span
                      ><span class="cell-note">unknown</span>{/if}</td
                  ><td
                    ><button
                      class="icon-button expand-button"
                      class:opened={expanded === item.prompt.id}
                      aria-expanded={expanded === item.prompt.id}
                      aria-label={expanded === item.prompt.id
                        ? "Hide prompt details"
                        : "Show prompt details"}
                      onclick={() =>
                        (expanded =
                          expanded === item.prompt.id ? null : item.prompt.id)}
                      ><Icon name="chevron" size={16} /></button
                    ></td
                  ></tr
                >{#if expanded === item.prompt.id}<tr class="detail-row"
                    ><td colspan="5"
                      ><div class="prompt-details">
                        <div>
                          <span class="eyebrow">PROMPT DETAILS</span>
                          <p>
                            Status: <strong>{item.prompt.status}</strong> · {item
                              .prompt.kind}
                          </p>
                          <p class="fineprint">
                            Conversation: {item.prompt.sessionId}<br />Turn: {item
                              .prompt.turnId}<br />Account: {item.prompt
                              .accountId}
                          </p>
                          {#if item.quotaEstimate}<p
                              class="estimate-explanation"
                            >
                              Estimated quota impact: {item.quotaEstimate
                                .explanation} ({item.quotaEstimate.confidence} confidence)
                            </p>{:else}<p class="fineprint">
                              Quota impact is unknown without enough clean
                              readings for this prompt.
                            </p>{/if}
                        </div>
                        <TokenChart tokens={item.tokens} />
                      </div>
                      {#if item.requests.length}<div class="request-list">
                          {#each item.requests as request (request.id)}<div>
                              <span class="request-kind">{request.kind}</span
                              ><span
                                >{request.model}<small
                                  >{request.effort ?? "unknown"} reasoning</small
                                ></span
                              ><span
                                >{count(totalTokens(request.tokens))} tokens<small
                                  >{dateTime(request.timestamp)}</small
                                ></span
                              >
                            </div>{/each}
                        </div>{/if}</td
                    ></tr
                  >{/if}{:else}<tr
                  ><td colspan="5"
                    ><div class="empty-state">
                      <Icon name="history" size={32} />
                      <h3>
                        {historyLoading
                          ? "Reading your history…"
                          : "No prompts to show yet"}
                      </h3>
                      <p>
                        {historyLoading
                          ? "Large histories may take a moment."
                          : "Import your local coding history, or try a different filter."}
                      </p>
                    </div></td
                  ></tr
                >{/each}</tbody
            >
          </table>
        </div>
        <div class="pagination">
          <span
            >{history.total
              ? `${pageIndex * 50 + 1}–${Math.min((pageIndex + 1) * 50, history.total)} of ${exactCount(history.total)}`
              : "0 prompts"}</span
          >
          <div>
            <button
              class="button small"
              disabled={pageIndex === 0 || historyLoading}
              onclick={() => pageIndex--}>Previous</button
            ><button
              class="button small"
              disabled={(pageIndex + 1) * 50 >= history.total || historyLoading}
              onclick={() => pageIndex++}>Next</button
            >
          </div>
        </div>
      </section>
      {#if history.items.length}<section class="panel page-breakdown">
          <div class="panel-heading">
            <h2>Tokens in these {history.items.length} prompts</h2>
            <span class="subtle">Current page</span>
          </div>
          <TokenChart tokens={pageTokenTotals} />
        </section>{/if}
      <p class="fineprint history-footnote">
        Date filters select when prompts started; rows show each prompt’s
        complete recorded usage. Only 160 characters of each prompt are stored.
        Tokens are measured from local logs. Quota impact is an estimate;
        browser conversations are outside this history.
      </p>
    {:else if page === "recommendations"}
      <div class="task-selector" role="group" aria-label="Choose your task">
        <button
          class:selected={task === "quick"}
          onclick={() => (task = "quick")}
          ><span class="task-symbol">↗</span><strong>Quick work</strong><span
            >Small edits & everyday questions</span
          ></button
        ><button
          class:selected={task === "everyday"}
          onclick={() => (task = "everyday")}
          ><span class="task-symbol">⌘</span><strong>Everyday work</strong><span
            >Coding, writing & problem solving</span
          ></button
        ><button
          class:selected={task === "complex"}
          onclick={() => (task = "complex")}
          ><span class="task-symbol">✳</span><strong>Complex analysis</strong
          ><span>Deep reasoning & difficult problems</span></button
        >
      </div>
      <div class="advice-summary">
        <Icon name="info" size={18} />
        <p>
          {adviceLoading
            ? "Finding a good fit for your next task…"
            : advice.summary ||
              "Connect your coding tools to see model suggestions."}
        </p>
      </div>
      <div class="recommendations-grid" aria-busy={adviceLoading}>
        {#each advice.recommendations as recommendation}<article
            class="panel recommendation-card"
          >
            <div class="card-topline">
              <span class="provider-name"
                ><span class={`provider-dot ${recommendation.provider}`}
                ></span>{providerName(recommendation.provider)}</span
              ><span
                class="status-pill"
                class:live={recommendation.fitsBudget === true}
                >{recommendation.fitsBudget === true
                  ? "Within budget"
                  : recommendation.fitsBudget === false
                    ? "Budget constrained"
                    : "Provisional"}</span
              >
            </div>
            <h2>{recommendation.model}</h2>
            <div class="recommendation-effort">
              <Icon name="recommendations" size={15} />{recommendation.effort ??
                "Default"} reasoning
            </div>
            <p class="recommendation-reason">{recommendation.reason}</p>
            <div class="forecast">
              <div>
                <span>Estimated tokens / prompt</span><strong
                  >{count(recommendation.estimatedTokens)}</strong
                >
              </div>
              <div>
                <span>Estimated quota / prompt</span><strong
                  >{recommendation.estimatedQuotaPercent == null
                    ? "—"
                    : `~${recommendation.estimatedQuotaPercent.toFixed(2)}%`}</strong
                >
              </div>
            </div>
            <div class="confidence">
              <span>{recommendation.confidence} confidence</span><span
                >{recommendation.sampleCount} completed prompts</span
              >
            </div>
            {#if recommendation.sampleCount < 20}<p class="fineprint">
                Personalized forecasts need at least 20 completed prompts with
                this model and reasoning level.
              </p>{/if}{#each recommendation.warnings as warning}<p
                class="recommendation-warning"
              >
                <Icon name="info" size={14} />{warning}
              </p>{/each}
          </article>{:else}<div class="panel empty-state">
            <Icon name="recommendations" size={32} />
            <h3>
              {adviceLoading
                ? "Checking available models…"
                : "Your next model, thoughtfully chosen"}
            </h3>
            <p>Check your provider connections in Settings to get started.</p>
            <button class="button" onclick={() => (page = "settings")}
              >Open settings <Icon name="arrow" size={15} /></button
            >
          </div>{/each}
      </div>
      <section class="advice-method">
        <h2>How your suggestions work</h2>
        <div>
          <p>
            <strong>A little breathing room.</strong> We keep {overview
              ?.settings.reservePercent ?? 10}% of your allowance in reserve and
            consider the time until it resets.
          </p>
          <p>
            <strong>Your last 30 days.</strong> Measured usage helps estimate consumption.
            Model suitability comes from the app’s catalog.
          </p>
          <p>
            <strong>Always your call.</strong> Suggestions run on this computer and
            use no model allowance. Your selected model never changes automatically.
          </p>
        </div>
      </section>
    {:else if page === "settings"}
      {#if settings}<div class="settings-layout">
          <section class="panel settings-panel appearance-panel">
            <div class="panel-heading">
              <div>
                <h2>Appearance</h2>
                <p>Make the meters comfortable to glance at.</p>
              </div>
            </div>
            <label class="form-field"
              >Color theme<select
                bind:value={settings.theme}
                onchange={() => (dirty = true)}
              >
                <option value="black">Black</option><option value="slate"
                  >Slate</option
                ><option value="midnight">Midnight</option><option value="light"
                  >Light</option
                ><option value="system">System</option>
              </select></label
            >
            <label class="form-field"
              >Meter percentages<select
                bind:value={settings.meterDisplay}
                onchange={() => (dirty = true)}
              >
                <option value="remaining">Percentage left</option><option
                  value="used">Percentage used</option
                >
              </select></label
            >
            <label class="form-field font-control"
              >Font size · {settings.fontScale}%
              <input
                aria-label="Font size"
                type="range"
                min="90"
                max="160"
                step="10"
                bind:value={settings.fontScale}
                oninput={() => (dirty = true)}
              />
              <span class="fineprint"
                >90%–160% · Default 120%. Preview changes here, then save to
                update the strip.</span
              >
            </label>
            {#if isWindows}<label class="toggle-row"
                ><span
                  ><strong>Lock strip position</strong><small
                    >Prevent accidental movement. Off by default; drag anywhere
                    on the dock to move it. Click to open the dashboard.</small
                  ></span
                ><input
                  type="checkbox"
                  bind:checked={settings.stripLocked}
                  onchange={() => (dirty = true)}
                /><span class="switch" aria-hidden="true"></span></label
              >{/if}
            {#if isWindows}
              <label class="form-field"
                >Dock activity interval
                <select
                  aria-label="Dock activity interval"
                  value={[30, 60, 360, 1440, 10080].includes(
                    settings.dockMinutes,
                  )
                    ? settings.dockMinutes
                    : "custom"}
                  onchange={(event) => {
                    const value = event.currentTarget.value;
                    if (value !== "custom")
                      settings!.dockMinutes = Number(value);
                    else settings!.dockMinutes = 90;
                    dirty = true;
                  }}
                >
                  <option value="30">Last 30 minutes</option><option value="60"
                    >Last hour</option
                  ><option value="360">Last 6 hours</option><option value="1440"
                    >Last 24 hours</option
                  ><option value="10080">Last 7 days</option><option
                    value="custom">Custom duration</option
                  >
                </select>
              </label>
              <label class="form-field"
                >Custom dock duration (minutes)
                <input
                  aria-label="Custom dock duration (minutes)"
                  type="number"
                  min="1"
                  max="43200"
                  step="1"
                  bind:value={settings.dockMinutes}
                  oninput={() => (dirty = true)}
                />
              </label>
              <label class="toggle-row"
                ><span
                  ><strong>Show prompt previews on hover</strong><small
                    >Hover each meter for its last 10 prompts. Token and model
                    totals use the selected interval; the latest prompts are
                    shown regardless of date.</small
                  ></span
                ><input
                  type="checkbox"
                  bind:checked={settings.dockPreviews}
                  onchange={() => (dirty = true)}
                /><span class="switch" aria-hidden="true"></span></label
              >
              <p class="fineprint">
                Recorded activity includes all local and synced computers.
                Dock activity
                refreshes after imports and at least once a minute.
              </p>
            {/if}
            <p class="fineprint">
              Theme applies to the dashboard and Windows strip. macOS menu-bar
              colors and text size follow the system. The Windows strip is a
              freely movable window; it does not become part of the taskbar.
            </p>
          </section>
          <section class="panel settings-panel">
            <div class="panel-heading">
              <div>
                <h2>Your connections</h2>
                <p>Uses the coding tools already signed in on this computer.</p>
              </div>
              <button class="button small" disabled={!!busy} onclick={refresh}
                >Detect & refresh</button
              >
            </div>
            {#each ["claude", "codex"] as value}{@const provider =
                value as Provider}{@const snapshot = snapshots.find(
                (item) => item.provider === provider,
              )}
              <div class="connection-row">
                <div class={`connection-icon ${provider}`}>
                  {provider === "claude" ? "✳" : "◎"}
                </div>
                <div class="connection-copy">
                  <strong
                    >{provider === "claude"
                      ? "Claude Code"
                      : "Codex app / CLI"}</strong
                  ><span class:connected={snapshot?.status === "connected"}
                    >{statusName(snapshot?.status)}</span
                  >{#if snapshot?.message}<p>{snapshot.message}</p>{/if}
                </div>
                <button
                  class="button small"
                  disabled={!!busy}
                  onclick={() => reconnect(provider)}
                  >{busy === `reconnect-${provider}`
                    ? "Opening…"
                    : snapshot?.status === "connected"
                      ? "Check connection"
                      : "Connect"}</button
                >
              </div>{/each}
            <p class="fineprint connection-help">
              Connect checks your existing sign-in first. If needed, complete
              the provider’s sign-in in your browser; your meters refresh
              automatically.
            </p>
            <label class="toggle-row"
              ><span
                >Use organization SSO for Claude<small
                  >Enable only when your organization has configured SSO.</small
                ></span
              ><input type="checkbox" bind:checked={useSso} /></label
            >
          </section>
          <section class="panel settings-panel">
            <div class="panel-heading">
              <div>
                <h2>This computer</h2>
                <p>Keep the meters available whenever you work.</p>
              </div>
            </div>
            <label class="form-field"
              >Computer name<input
                bind:value={settings.deviceName}
                oninput={() => (dirty = true)}
                maxlength="80"
                placeholder="My computer"
              /></label
            ><label class="toggle-row"
              ><span
                ><strong>Launch at login</strong><small
                  >Show your meters when you sign in to your computer.</small
                ></span
              ><input
                type="checkbox"
                bind:checked={settings.launchAtLogin}
                onchange={() => (dirty = true)}
              /><span class="switch" aria-hidden="true"></span></label
            >
            <div class="settings-info">
              <span>Refresh interval</span><strong
                >Every 2 minutes, and on wake</strong
              >
            </div>
            <div class="settings-info">
              <span>Notifications</span><strong>Off</strong>
            </div>
            <div class="settings-info">
              <span>Updates</span><strong>Manual</strong>
            </div>
          </section>
          <section class="panel settings-panel">
            <div class="panel-heading">
              <div>
                <h2>History across computers</h2>
                <p>
                  Choose the same folder in OneDrive, iCloud Drive, Dropbox, or
                  another sync service on every computer.
                </p>
              </div>
            </div>
            <div class="folder-select">
              <Icon name="folder" size={20} /><span
                >{settings.syncFolder ?? "No shared folder selected"}</span
              ><button
                class="button small"
                onclick={chooseFolder}
                disabled={!!busy}>Choose folder</button
              >
            </div>
            {#if settings.syncFolder}<div class="sync-detail">
                <span
                  >{overview?.syncMessage ??
                    `Folder checked ${relativeTime(overview?.lastSync, now).toLowerCase()}`}</span
                ><button
                  class="text-button"
                  onclick={() => {
                    if (settings) settings.syncFolder = null;
                    dirty = true;
                  }}>Turn off sync</button
                >
              </div>{/if}
            {#if settings.syncFolder}
              <button
                class="button small"
                disabled={!!busy}
                onclick={importHistory}>Sync now</button
              >
              <p class="fineprint">
                {overview?.syncHealth?.pendingEvents ?? 0} records waiting to be published.
                Keep GCD Usage updated on each computer for compatible history sync.
              </p>
              {#each overview?.syncHealth?.devices ?? [] as device}
                <div class="connection-row">
                  <div class="connection-copy">
                    <strong
                      >{device.name}{device.deviceId === settings.deviceId
                        ? " (this computer)"
                        : ""}</strong
                    ><span
                      >Last seen {relativeTime(
                        device.lastSeen,
                        now,
                      ).toLowerCase()}</span
                    >
                    <p>
                      {device.deviceId === settings.deviceId
                        ? "Published to the selected folder"
                        : device.pendingBatches == null
                          ? "Receipt coverage incomplete"
                          : device.pendingBatches === 0
                            ? "Reports receiving all currently known batches"
                            : `${device.pendingBatches} batches awaiting receipt`}
                    </p>
                  </div>
                </div>
              {/each}
              {#if (overview?.syncHealth?.devices.length ?? 0) < 2}<p
                  class="fineprint"
                >
                  No other computer has reported yet. Select this same synced
                  folder on your other computer and click Sync now there.
                </p>{/if}
            {/if}
            <p class="fineprint">
              Sync includes prompt previews and usage records. Your local
              database, device paths, and credentials stay on this computer.
              Keep the shared folder private.
            </p>
          </section>
          <section class="panel settings-panel">
            <div class="panel-heading">
              <div>
                <h2>Model advice</h2>
                <p>A small reserve leaves room for unexpected work.</p>
              </div>
            </div>
            <label class="reserve-field"
              ><span>Allowance to keep in reserve</span>
              <div>
                <input
                  type="number"
                  min="0"
                  max="50"
                  step="1"
                  bind:value={settings.reservePercent}
                  oninput={() => (dirty = true)}
                /><span>%</span>
              </div></label
            >
            <p class="fineprint">
              Suggestions consider each provider’s limits independently. No
              model calls are made.
            </p>
          </section>
          <section class="panel settings-panel">
            <div class="panel-heading">
              <div>
                <h2>Local history</h2>
                <p>
                  {overview?.importing
                    ? "Importing available logs…"
                    : `${exactCount(overview?.importReport.files ?? 0)} files read in the latest import`}
                </p>
              </div>
              <button
                class="button small"
                onclick={importHistory}
                disabled={!!busy || overview?.importing}
                >{overview?.importing ? "Importing…" : "Import now"}</button
              >
            </div>
            <p class="fineprint">
              Available and archived Claude Code and Codex logs are imported,
              then watched for updates. Older logs may not record reasoning
              levels or quota impact.
            </p>
            {#if overview?.importReport.warnings.length}<details
                class="import-warnings"
              >
                <summary
                  >{overview.importReport.warnings.length} import notices</summary
                >
                <ul>
                  {#each overview.importReport.warnings as warning}<li>
                      {warning}
                    </li>{/each}
                </ul>
              </details>{/if}
            <details class="advanced">
              <summary>Advanced connection paths</summary>
              <p class="fineprint">
                Leave blank to discover installed tools and their default
                history folders.
              </p>
              <label class="form-field"
                >Codex executable<input
                  value={settings.codexPath ?? ""}
                  oninput={(event) => {
                    if (settings)
                      settings.codexPath = event.currentTarget.value || null;
                    dirty = true;
                  }}
                  placeholder="Automatic detection"
                /></label
              ><label class="form-field"
                >Claude executable<input
                  value={settings.claudePath ?? ""}
                  oninput={(event) => {
                    if (settings)
                      settings.claudePath = event.currentTarget.value || null;
                    dirty = true;
                  }}
                  placeholder="Automatic detection"
                /></label
              ><label class="form-field"
                >Codex home folder<input
                  value={settings.codexHome ?? ""}
                  oninput={(event) => {
                    if (settings)
                      settings.codexHome = event.currentTarget.value || null;
                    dirty = true;
                  }}
                  placeholder="Default Codex home"
                /></label
              ><label class="form-field"
                >Claude home folder<input
                  value={settings.claudeHome ?? ""}
                  oninput={(event) => {
                    if (settings)
                      settings.claudeHome = event.currentTarget.value || null;
                    dirty = true;
                  }}
                  placeholder="Default Claude home"
                /></label
              >
            </details>
          </section>
          <div class="save-bar">
            <span
              >{dirty
                ? "You have unsaved changes."
                : "Your settings are saved on this computer."}</span
            ><button
              class="button primary"
              disabled={!!busy || (!dirty && settings.setupComplete)}
              onclick={() => saveSettings(!settings?.setupComplete)}
              >{busy === "save"
                ? "Saving…"
                : settings.setupComplete
                  ? "Save changes"
                  : "Finish setup"}<Icon name="check" size={15} /></button
            >
          </div>
        </div>{:else}<div class="panel empty-state">
          <Icon name="settings" size={30} />
          <h3>
            {loading
              ? "Loading settings…"
              : "Settings are available in the desktop app"}
          </h3>
          <p>Your preferences are stored locally on each computer.</p>
        </div>{/if}
    {/if}
  </main>
</div>
{#if selectedPrompt}<PromptDetails detail={selectedPrompt} onclose={()=>selectedPrompt=null}/>{/if}
