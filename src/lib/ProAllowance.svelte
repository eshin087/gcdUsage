<script lang="ts">
  import {invoke} from "@tauri-apps/api/core";
  import {native} from "./api";
  let error = $state('');
  async function source(event: MouseEvent) { if(native){event.preventDefault();try{await invoke("open_pro_documentation");}catch{error="Could not open the documentation. The source is also linked in the manual.";}} }
  // Documentation describes caps, not this account's live consumption.
  // Never subtract Codex activity from browser Chat's separate allowance.
  const checked = '2026-09-08';
</script>

<details class="pro-reference"><summary>Browser Pro reference <span class="subtle">/ remaining unavailable</span></summary>
<section class="pro-allowance" aria-label="Browser ChatGPT Pro allowance">
  <div class="pro-label"><span class="eyebrow">[ CHAT / PRO $200 ]</span><strong>Browser Pro</strong><span class="subtle">Separate from Work / Codex</span></div>
  <div><span>GPT-6 Pro</span><strong>200 <small>/ week</small></strong></div>
  <div><span>GPT-5.6 Sol Pro</span><strong>170 <small>/ day</small></strong></div>
  <div><span>Both models combined</span><strong>200 <small>/ day</small></strong></div>
  <div class="pro-remaining"><span>Requests remaining</span><strong aria-label="Remaining requests unavailable">—</strong><span class="subtle">Account counter unavailable</span></div>
  <details><summary>Documented caps, not a live account reading</summary><p>These are the published limits for Pro $200, checked {checked}. This app cannot observe browser ChatGPT requests on another computer or obtain a remaining Chat counter through its Codex connection. It does not guess a reset time or deduct local coding requests. Browser imports and manual counters are not required or offered. Check ChatGPT for your account's current availability and reset time. Limits can change.</p><p><a href="https://help.openai.com/en/articles/20001354-gpt-56-and-gpt-6-pro-in-chatgpt" target="_blank" rel="noreferrer" onclick={source}>OpenAI: GPT-5.6 and GPT-6 Pro in ChatGPT</a></p>{#if error}<p role="alert">{error}</p>{/if}</details>
</section>
</details>
