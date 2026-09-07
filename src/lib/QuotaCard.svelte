<script lang="ts">
  import type { Provider, QuotaSnapshot } from "./types";
  import {
    countdown,
    isStale,
    percent,
    providerName,
    quotaWindow,
    relativeTime,
  } from "./format";
  import Icon from "./Icon.svelte";
  export let provider: Provider;
  export let period: "five_hour" | "weekly";
  export let snapshot: QuotaSnapshot | undefined;
  export let now: number;
  export let reconnect: (provider: Provider) => void;
  $: window = quotaWindow(snapshot, period);
  $: stale = isStale(snapshot, window, now);
  $: used =
    window == null ? null : Math.min(100, Math.max(0, window.usedPercent));
  $: needsAuth = snapshot?.status === "needs_auth";
  $: statusText = stale
    ? "Stale"
    : needsAuth
      ? "Sign in needed"
      : window
        ? "Live"
        : "Unavailable";
</script>

<article
  class="quota-card"
  class:claude={provider === "claude"}
  class:codex={provider === "codex"}
  class:stale
>
  <div class="card-topline">
    <span class="provider-name"
      ><span class="provider-mark">{provider === "claude" ? "✳" : "◎"}</span
      >{providerName(provider)}</span
    ><span class="status-pill" class:live={!!window && !stale && !needsAuth}
      ><span class="status-dot"></span>{statusText}</span
    >
  </div>
  <h2>{period === "five_hour" ? "Five-hour limit" : "Weekly limit"}</h2>
  <div class="quota-value">
    {percent(used)}<span>{used == null ? "No reading yet" : "used"}</span>
  </div>
  <div
    class="quota-track"
    role="progressbar"
    aria-label={`${providerName(provider)} ${period === "five_hour" ? "five-hour" : "weekly"} usage`}
    aria-valuemin="0"
    aria-valuemax="100"
    aria-valuenow={used ?? undefined}
    aria-valuetext={used == null
      ? "Unavailable"
      : `${Math.round(used)}% used${stale ? ", stale reading" : ""}`}
  >
    <div
      style={`width:${used ?? 0}%`}
      class:critical={used != null && used >= 90}
    ></div>
  </div>
  <div class="quota-footer">
    <span
      ><Icon name="clock" size={13} />{countdown(window?.resetsAt, now)}</span
    >{#if used != null}<strong>{percent(100 - used)} left</strong>{/if}
  </div>
  {#if needsAuth}<button
      class="text-button reconnect"
      on:click={() => reconnect(provider)}
      >Reconnect {providerName(provider)}
      <Icon name="arrow" size={14} /></button
    >
  {:else if snapshot?.message && !window}<p class="quota-note">
      {snapshot.message}
    </p>
  {:else if stale}<p class="quota-note">
      Last reading {relativeTime(snapshot?.fetchedAt, now).toLowerCase()}.
      Waiting for a fresh reading.
    </p>{/if}
</article>
