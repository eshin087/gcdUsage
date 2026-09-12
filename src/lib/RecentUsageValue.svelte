<script lang="ts">
  import type { RecentAllowanceWindow } from "./types";
  import { coverageText, durationLabel, recentPercent } from "./recent-usage";
  let {
    value,
    minutes,
    loading = false,
    compact = false,
  }: {
    value?: RecentAllowanceWindow;
    minutes: number;
    loading?: boolean;
    compact?: boolean;
  } = $props();
</script>

<div
  class="recent-usage-value"
  class:partial={value?.state === "partial"}
  aria-busy={loading}
>
  {#if loading}<span class="fineprint">Updating recent usage…</span>
  {:else if value?.consumedPercent != null}
    <p
      title="Percentage points of this allowance, independent of token counts. Provider rounding can hide small changes."
    >
      <strong>{recentPercent(value.consumedPercent)}</strong>
      {value.state === "partial" ? "observed" : "used"}<span>
        · last {durationLabel(minutes)}</span
      >
    </p>
    <small title={coverageText(value, minutes)}
      >{value.state === "partial" ? "Partial coverage · " : ""}{!compact ||
      value.state === "partial"
        ? coverageText(value, minutes)
        : "of this allowance"}</small
    >
    {#if !compact && value.consumedPercent === 0}<small
        >No visible change between readings; small usage may be rounded.</small
      >{/if}
  {:else}
    <p>
      <strong>—</strong> recent usage<span>
        · last {durationLabel(minutes)}</span
      >
    </p>
    <small
      >{value?.state === "stale"
        ? "Waiting for current readings"
        : "Need comparable allowance readings"}</small
    >
  {/if}
</div>
