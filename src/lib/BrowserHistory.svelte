<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "./api";
  import { count, dateTime } from "./format";
  import {
    localDateTime,
    resolveRange,
    timePresets,
    type TimePreset,
  } from "./time-range";
  import type { BrowserPage } from "./types";
  let rows = $state<BrowserPage>({
    items: [],
    total: 0,
    conversations: 0,
    replies: 0,
  });
  let query = $state(""),
    provider = $state(""),
    account = $state("Personal"),
    error = $state(""),
    notice = $state("");
  let loading = $state(false),
    importing = $state(false),
    offset = $state(0),
    ready = $state(false);
  let preset = $state<TimePreset>("all"),
    from = $state(localDateTime(Math.floor(Date.now() / 1000) - 86400)),
    to = $state(localDateTime(Math.floor(Date.now() / 1000)));
  let clock = $state(Math.floor(Date.now() / 1000)),
    revision = $state(0),
    request = 0;
  onMount(() => {
    ready = true;
    const timer = setInterval(
      () => (clock = Math.floor(Date.now() / 1000)),
      30000,
    );
    return () => {
      ready = false;
      request++;
      clearInterval(timer);
    };
  });
  $effect(() => {
    query;
    provider;
    preset;
    from;
    to;
    offset = 0;
  });
  $effect(() => {
    if (!ready) return;
    const current = ++request;
    let range;
    try {
      range = resolveRange(preset, clock, from, to);
    } catch (e) {
      error = String(e);
      loading = false;
      return;
    }
    revision;
    const filter = {
      query,
      provider,
      offset,
      from: preset === "all" ? null : range.from,
      to: preset === "all" ? null : range.to,
    };
    loading = true;
    error = "";
    const timer = setTimeout(() => {
      api
        .browserHistory(filter)
        .then((value) => {
          if (current === request) rows = value;
        })
        .catch((e) => {
          if (current === request) error = String(e);
        })
        .finally(() => {
          if (current === request) loading = false;
        });
    }, 180);
    return () => clearTimeout(timer);
  });
  async function importFile() {
    importing = true;
    error = "";
    notice = "";
    try {
      const report = await api.importBrowser(account);
      if (report) {
        notice = `Added ${report.added} prompts; ${report.duplicates} already present; ${report.skipped} unsupported records skipped.`;
        revision++;
      }
    } catch (e) {
      error = String(e);
    } finally {
      importing = false;
    }
  }
</script>

<section class="panel browser-panel">
  <h2>Import past browser conversations</h2>
  <p>
    Download a conversation export from Claude or ChatGPT. If it is a ZIP,
    extract it and choose its conversations.json file (up to 64 MB). No browser
    extension is needed.
  </p>
  <div class="import-row">
    <label
      >Account label<input
        bind:value={account}
        maxlength="80"
        placeholder="Personal"
      /></label
    ><button
      class="button"
      disabled={importing || !account.trim()}
      onclick={importFile}
      >{importing ? "Importing…" : "Import browser export"}</button
    >
  </div>
  <p class="fineprint">
    Use the same label for the same account on every computer. Imports keep
    160-character prompt previews and available model metadata. Exact token use,
    hidden reasoning, and the original computer are unavailable from these
    imports. Import only conversations you are permitted to store and
    synchronize.
  </p>
  {#if notice}<p role="status">{notice}</p>{/if}
  {#if error}<p role="alert">{error}</p>{/if}
</section>
<section class="panel browser-panel">
  <div class="filters">
    <label
      >Search browser prompts<input
        bind:value={query}
        maxlength="1024"
        type="search"
      /></label
    >
    <label
      >Browser provider<select bind:value={provider}
        ><option value="">All providers</option><option value="claude"
          >Claude</option
        ><option value="chatgpt">ChatGPT</option></select
      ></label
    >
    <label
      >Browser time range<select bind:value={preset}
        >{#each timePresets as [value, label]}<option {value}>{label}</option
          >{/each}</select
      ></label
    >
    {#if preset === "custom"}<label
        >Browser range start<input
          type="datetime-local"
          step="1"
          bind:value={from}
        /></label
      ><label
        >Browser range end<input
          type="datetime-local"
          step="1"
          bind:value={to}
        /></label
      >{/if}
  </div>
  <div class="summary" aria-busy={loading}>
    <span><strong>{count(rows.total)}</strong> prompts</span><span
      ><strong>{count(rows.conversations)}</strong> conversations</span
    ><span><strong>{count(rows.replies)}</strong> visible replies</span><span
      >Token usage <strong>—</strong></span
    >
  </div>
  <p class="fineprint">
    These counts describe exported messages. Visible replies are not
    model-request counts. Undated prompts appear only in All time.
  </p>
  <div class="table-wrap">
    <table>
      <thead
        ><tr
          ><th>Prompt</th><th>Provider / account</th><th>Model / reasoning</th
          ><th>Date</th></tr
        ></thead
      ><tbody>
        {#each rows.items as row (row.id)}<tr
            ><td>{row.preview || "No text preview available"}</td><td
              >{row.provider === "chatgpt" ? "ChatGPT" : "Claude"}<small
                >{row.accountLabel}</small
              ></td
            ><td
              >{row.models.join(", ") || "Unknown model"}<small
                >{row.effort ?? "Unknown reasoning"}</small
              ></td
            ><td
              >{row.timestamp == null ? "Unknown" : dateTime(row.timestamp)}</td
            ></tr
          >{/each}
      </tbody>
    </table>
  </div>
  {#if !loading && !rows.total}<p>
      No imported browser prompts match this range.
    </p>{/if}
  <div class="pagination">
    <button
      class="button small"
      disabled={offset === 0}
      onclick={() => (offset = Math.max(0, offset - 50))}
      >Previous browser page</button
    ><span
      >{rows.total
        ? `${offset + 1}–${Math.min(offset + 50, rows.total)} of ${rows.total}`
        : "0 prompts"}</span
    ><button
      class="button small"
      disabled={offset + 50 >= rows.total}
      onclick={() => (offset += 50)}>Next browser page</button
    >
  </div>
</section>

<style>
  .browser-panel {
    padding: 1.3rem;
    margin-bottom: 1.2rem;
  }
  .browser-panel h2 {
    margin-top: 0;
  }
  .browser-panel p {
    line-height: 1.55;
  }
  .import-row,
  .filters {
    display: flex;
    gap: 1rem;
    flex-wrap: wrap;
    align-items: end;
  }
  .filters label {
    flex: 1;
    min-width: 10rem;
  }
  label {
    display: grid;
    gap: 0.4rem;
  }
  input,
  select {
    font: inherit;
    padding: 0.6rem;
    color: inherit;
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: 8px;
  }
  .summary {
    display: flex;
    gap: 1.8rem;
    flex-wrap: wrap;
    margin: 1.4rem 0;
  }
  .summary strong {
    font-size: 1.3em;
  }
  .table-wrap {
    overflow: auto;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    text-align: left;
  }
  th,
  td {
    padding: 0.8rem;
    border-bottom: 1px solid var(--line);
    vertical-align: top;
  }
  td:first-child {
    min-width: 12rem;
    max-width: 28rem;
    overflow-wrap: anywhere;
  }
  small {
    display: block;
    margin-top: 0.3rem;
    opacity: 0.65;
  }
  .pagination {
    display: flex;
    gap: 1rem;
    align-items: center;
    justify-content: space-between;
    margin-top: 1rem;
  }
</style>
