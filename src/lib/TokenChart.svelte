<script lang="ts">
  import type { TokenUsage } from "./types";
  import { count, exactCount, totalTokens } from "./format";
  export let tokens: TokenUsage;
  $: total = totalTokens(tokens);
  $: segments = [
    { label: "Uncached input", value: tokens.input, color: "var(--accent)" },
    { label: "Cache reads", value: tokens.cacheRead, color: "var(--claude)" },
    {
      label: "Cache writes",
      value: tokens.cacheWrite,
      color: "var(--chart-muted)",
    },
    { label: "Output", value: tokens.output, color: "var(--codex)" },
  ];
</script>

<div class="token-chart">
  <div
    class="token-bar"
    aria-label={`Token breakdown: ${segments.map((segment) => `${segment.label} ${exactCount(segment.value)}`).join(", ")}`}
    role="img"
  >
    {#each segments as segment}<span
        style={`width:${total ? ((segment.value ?? 0) / total) * 100 : 0}%;background:${segment.color}`}
        title={`${segment.label}: ${exactCount(segment.value)}`}
      ></span>{/each}
  </div>
  <div class="token-legend">
    {#each segments as segment}<div>
        <span><i style={`background:${segment.color}`}></i>{segment.label}</span
        ><strong>{count(segment.value)}</strong>
      </div>{/each}
  </div>
  <p class="fineprint">
    Reasoning: {count(tokens.reasoning)} tokens, included in output.
  </p>
</div>
