<script module lang="ts">
  export interface PromptDetail {id:string;provider:string;project:string|null;chatTitle:string|null;conversationId:string;turnId:string;preview:string;timestamp:number|null;tokens:import('./types').TokenUsage;models:string[];requestCount:number;originalUrl:string|null;limitation:string}
</script>
<script lang="ts">
 import {onMount} from 'svelte';
 import {dateTime,count,totalTokens} from './format';
 import {api} from './api';
 let {detail,onclose}:{detail:PromptDetail;onclose:()=>void}=$props();
 let dialog:HTMLDialogElement;let error=$state('');
 onMount(()=>{dialog.showModal()});
</script>
<dialog bind:this={dialog!} class="prompt-modal" onclose={onclose} aria-label="Recorded prompt details">
 <div class="modal-heading"><div><span class="eyebrow">RECORDED PROMPT / {detail.provider}</span><h2>{detail.chatTitle || 'Conversation details'}</h2></div><button class="icon-button" aria-label="Close prompt details" onclick={()=>dialog.close()}>×</button></div>
 <div class="context-ribbon"><span>{detail.project || 'Project not recorded'}</span><span>{detail.timestamp==null?'Date unknown':dateTime(detail.timestamp)}</span></div>
 <p class="detail-preview">{detail.preview || 'No text preview recorded'}</p>
 <div class="detail-metrics"><div><small>Measured tokens</small><strong>{count(totalTokens(detail.tokens))}</strong></div><div><small>Model requests</small><strong>{detail.requestCount || '—'}</strong></div></div>
 <p class="detail-models">{detail.models.join(' / ') || 'Model and reasoning not recorded'}</p>
 <details><summary>Conversation and prompt identifiers</summary><label>Conversation<input readonly value={detail.conversationId}/></label><label>Turn / message<input readonly value={detail.turnId}/></label></details>
 <p class="fineprint">{detail.limitation}</p>
 {#if error}<p role="alert">{error}</p>{/if}
 <div class="modal-actions">{#if detail.originalUrl}<button class="button primary" onclick={async()=>{try{await api.openOriginal(detail.id)}catch(e){error=String(e)}}}>Open original conversation ↗</button>{/if}<button class="button" onclick={()=>dialog.close()}>Done</button></div>
</dialog>
