<script lang="ts">
  import type { MeterDisplay, Provider, QuotaSnapshot, RecentAllowance } from "./types";
  import RecentUsageValue from "./RecentUsageValue.svelte";
  import {
    countdown,
    isStale,
    percent,
    providerName,
    quotaWindow,
    quotaPercent,
    relativeTime,
  } from "./format";
  export let provider: Provider;
  export let period: "five_hour" | "weekly" | "fable";
  export let snapshot: QuotaSnapshot | undefined;
  export let now: number;
  export let display: MeterDisplay = "remaining";
  export let recent: RecentAllowance | null = null;
  export let recentMinutes = 60;
  export let recentPending = false;
  $: window = quotaWindow(snapshot, period);
  $: stale = isStale(snapshot, window, now);
  $: value = quotaPercent(window?.usedPercent, display);
  $: unit = display === "remaining" ? "left" : "used";
  $: needsAuth = snapshot?.status === "needs_auth";
  $: statusText = needsAuth
    ? "Sign in needed"
    : stale
      ? "Stale"
      : window
        ? "Live"
        : "Unavailable";
  $: periodLabel =
    period === "five_hour"
      ? "5-hour"
      : period === "fable"
        ? "Fable weekly"
        : "Weekly";
  $: resetText =
    value == null
      ? "No reading yet"
      : window?.resetsAt == null
        ? "Reset unavailable"
        : window.resetsAt <= now
          ? "Reset pending"
          : countdown(window.resetsAt, now);
</script>

<article
  class="quota-card"
  class:stale
  aria-label={providerName(provider) + " " + periodLabel}
>
  <h2>{providerName(provider)} / {periodLabel}</h2>
  <span
    class="status-pill"
    class:live={!!window && !stale && !needsAuth}
    title={stale
      ? "Last reading " + relativeTime(snapshot?.fetchedAt, now).toLowerCase()
      : snapshot?.message}
  >
    <span class="status-dot"></span>{statusText}
  </span>
  <div class="quota-value" class:unavailable={value == null}>
    {percent(value)}{#if value != null}<span>{unit}</span>{/if}
  </div>
  <div
    class="quota-track"
    class:unavailable={value == null}
    role="progressbar"
    aria-label={providerName(provider) + " " + periodLabel + " " + unit}
    aria-valuemin="0"
    aria-valuemax="100"
    aria-valuenow={value ?? undefined}
    aria-valuetext={value == null
      ? "Unavailable"
      : Math.round(value) + "% " + unit + (stale ? ", stale reading" : "")}
  >
    {#if value != null}<div style:width={value + "%"}></div>{/if}
  </div>
  <div class="quota-footer" class:reset-time={value != null}>{resetText}</div>
  <RecentUsageValue value={recent?.windows.find(row=>row.provider===provider && row.windowId===window?.id)} minutes={recentMinutes} loading={recentPending} compact />
</article>
