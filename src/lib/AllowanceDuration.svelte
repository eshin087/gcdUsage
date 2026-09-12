<script lang="ts">
  import { durationLabel } from "./recent-usage";
  let {
    minutes,
    onchange,
  }: { minutes: number; onchange: (minutes: number) => Promise<void> } =
    $props();
  let custom = $state(false),
    amount = $state(60),
    unit = $state(1),
    saving = $state(false),
    error = $state("");
  async function apply(value: number) {
    if (!Number.isInteger(value) || value < 1 || value > 43200) {
      error = "Choose a whole duration from 1 minute to 30 days.";
      return;
    }
    saving = true;
    error = "";
    try {
      await onchange(value);
      custom = false;
    } catch (reason) {
      error = String(reason);
    } finally {
      saving = false;
    }
  }
</script>

<div class="allowance-duration">
  <div
    class="recent-duration-options"
    role="group"
    aria-label="Recent allowance duration"
  >
    <span>Recent usage · {durationLabel(minutes)}</span>
    {#each [30, 60, 360, 1440] as value}
      <button
        class:selected={minutes === value}
        disabled={saving}
        onclick={() => apply(value)}>{durationLabel(value)}</button
      >
    {/each}
    <button
      class:selected={custom}
      disabled={saving}
      onclick={() => {
        custom = !custom;
        amount = minutes;
        unit = 1;
        error = "";
      }}>Custom…</button
    >
  </div>
  {#if custom}
    <form
      class="recent-custom-duration"
      onsubmit={(event) => {
        event.preventDefault();
        void apply(amount * unit);
      }}
    >
      <label
        >Duration<input
          aria-label="Recent usage custom duration"
          type="number"
          min="1"
          max={Math.floor(43200 / unit)}
          step="1"
          bind:value={amount}
          required
        /></label
      >
      <label
        >Unit<select aria-label="Recent usage duration unit" bind:value={unit}
          ><option value={1}>Minutes</option><option value={60}>Hours</option
          ><option value={1440}>Days</option></select
        ></label
      >
      <button class="button primary" disabled={saving} type="submit"
        >Apply</button
      >
      <button
        class="button"
        disabled={saving}
        type="button"
        onclick={() => (custom = false)}>Cancel</button
      >
    </form>
  {/if}
  {#if error}<p class="field-error" role="alert">{error}</p>{/if}
  <p class="fineprint">
    Shared with the dock's token duration. Readings cover observed changes;
    resets and gaps can leave part of the period unknown.
  </p>
</div>
