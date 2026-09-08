<script lang="ts">
  import type { ActivityBucket } from "./types";
  import { count, exactCount } from "./format";
  export let activity: ActivityBucket[];
  export let bucketSeconds: number;
  $: max = Math.max(1, ...activity.map((bucket) => bucket.tokens));
  $: hasActivity = activity.some(
    (bucket) => bucket.tokens || bucket.requests || bucket.prompts,
  );
  const label = (timestamp: number, full = false) =>
    new Date(timestamp * 1000).toLocaleString(
      undefined,
      full
        ? { dateStyle: "medium", timeStyle: "short" }
        : bucketSeconds < 86400
          ? {
              month: "short",
              day: "numeric",
              hour: "numeric",
              minute: "2-digit",
            }
          : { month: "short", day: "numeric" },
    );
</script>

{#if hasActivity}
  <div class="daily-chart">
    <div class="chart-axis">
      <span>{count(max)}</span><span>{count(max / 2)}</span><span>0</span>
    </div>
    <div
      class="daily-bars"
      role="img"
      aria-label={`Activity in the selected interval. ${activity.map((bucket) => `${label(bucket.timestamp, true)}: ${exactCount(bucket.tokens)} tokens, ${bucket.requests} requests, ${bucket.prompts} new prompts`).join(". ")}`}
    >
      {#each activity as bucket}<div class="daily-column">
          <div
            class="daily-bar"
            style={`height:${Math.max(bucket.tokens ? 2 : 0, (bucket.tokens / max) * 100)}%`}
            title={`${label(bucket.timestamp, true)} · ${exactCount(bucket.tokens)} tokens · ${exactCount(bucket.requests)} requests · ${exactCount(bucket.prompts)} new prompts`}
          ></div>
        </div>{/each}
    </div>
  </div>
  <div class="chart-dates">
    <span>{label(activity[0].timestamp)}</span><span
      >{label(activity[activity.length - 1].timestamp)}</span
    >
  </div>
{:else}<div class="chart-empty">
    <p>No recorded activity in this interval.</p>
  </div>{/if}
