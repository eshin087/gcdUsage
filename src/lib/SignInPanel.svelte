<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "./api";
  import { providerName } from "./format";
  import type { SignInProgress, Provider } from "./types";
  let sessions = $state<SignInProgress[]>([]);
  let code = $state<Record<string, string>>({});
  let error = $state("");
  onMount(() => {
    let disposed = false;
    const update = async () => {
      try {
        const result = await api.signIns();
        if (!disposed) sessions = result;
      } catch {}
    };
    void update();
    const timer = setInterval(update, 1000);
    return () => {
      disposed = true;
      clearInterval(timer);
    };
  });
  async function send(provider: Provider, cancel = false) {
    const value = code[provider] ?? "";
    code[provider] = "";
    try {
      await api.signInInput(provider, cancel ? null : value);
      error = "";
    } catch (e) {
      error = String(e);
    }
  }
</script>

{#each sessions as session}
  <section
    class="signin-box"
    aria-label={`${providerName(session.provider)} sign-in`}
  >
    <strong>{providerName(session.provider)} connection</strong>
    <p role="status">{session.message}</p>
    {#if session.phase === "waiting"}
      <div class="actions">
        <label
          >Verification code, if shown in your browser<input
            type="password"
            autocomplete="off"
            bind:value={code[session.provider]}
            maxlength="4096"
          /></label
        >
        <button
          class="button small"
          disabled={!code[session.provider]?.trim()}
          onclick={() => send(session.provider)}>Submit code</button
        >
        <button
          class="button small"
          onclick={() => send(session.provider, true)}>Cancel sign-in</button
        >
      </div>
    {:else if session.phase === "checking"}
      <button class="button small" onclick={() => send(session.provider, true)}
        >Cancel sign-in</button
      >
    {/if}
  </section>
{/each}
{#if error}<p role="alert">{error}</p>{/if}

<style>
  .signin-box {
    border: 1px solid var(--line);
    background: var(--surface);
    padding: 1rem 1.2rem;
    border-radius: 14px;
    margin-bottom: 1rem;
  }
  .signin-box p {
    margin: 0.4rem 0;
    color: var(--muted);
  }
  .actions {
    display: flex;
    gap: 0.7rem;
    align-items: end;
    flex-wrap: wrap;
  }
  label {
    display: grid;
    gap: 0.4rem;
    flex: 1;
    min-width: 12rem;
  }
  input {
    font: inherit;
    padding: 0.6rem;
    background: var(--bg);
    color: inherit;
    border: 1px solid var(--line);
    border-radius: 8px;
  }
</style>
