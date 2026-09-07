<script lang="ts">
  import type { DailyStat } from "./types";
  import { count, exactCount, shortDate } from "./format";
  export let daily: DailyStat[];
  $: days = daily.slice(-14);
  $: max = Math.max(1, ...days.map((day) => day.tokens));
</script>

{#if days.length}
  <div class="daily-chart">
    <div class="chart-axis">
      <span>{count(max)}</span><span>{count(max / 2)}</span><span>0</span>
    </div>
    <div
      class="daily-bars"
      role="img"
      aria-label={`Daily tokens. ${days.map((day) => `${shortDate(day.date)}: ${exactCount(day.tokens)} tokens, ${day.prompts} prompts`).join(". ")}`}
    >
      {#each days as day}<div class="daily-column">
          <div
            class="daily-bar"
            style={`height:${Math.max(day.tokens ? 2 : 0, (day.tokens / max) * 100)}%`}
            title={`${shortDate(day.date)} · ${exactCount(day.tokens)} tokens · ${day.prompts} prompts`}
          ></div>
        </div>{/each}
    </div>
  </div>
  <div class="chart-dates">
    <span>{shortDate(days[0].date)}</span><span
      >{shortDate(days[days.length - 1].date)}</span
    >
  </div>
{:else}<div class="chart-empty">
    {@render IconPlaceholder()}
    <p>Your daily activity will appear after importing history.</p>
  </div>{/if}

{#snippet IconPlaceholder()}<svg
    aria-hidden="true"
    width="44"
    height="32"
    viewBox="0 0 44 32"
    ><path
      d="M4 28V17M16 28V6M28 28V11M40 28V2"
      stroke="currentColor"
      stroke-width="5"
      stroke-linecap="round"
    /></svg
  >{/snippet}
