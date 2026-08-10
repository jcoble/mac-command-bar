<script lang="ts">
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import LoaderCircle from '@lucide/svelte/icons/loader-circle';
  import ConversationMessage from './ConversationMessage.svelte';
  import type { ConversationDisplayItem } from '$lib/shell/conversation/conversationTimeline.ts';

  let { item, onFileLink }: {
    item: Extract<ConversationDisplayItem, { kind: 'reasoning' }>;
    onFileLink?(path: string): void;
  } = $props();

  const preview = $derived(item.text.replace(/\s+/g, ' ').trim().slice(0, 140) || 'Working through the request');
</script>

<details class:streaming={!item.completed} class="reasoning" data-testid="timeline-reasoning-item">
  <summary>
    <span class="chevron" aria-hidden="true"><ChevronRight size={14} strokeWidth={1.8} /></span>
    {#if !item.completed}<span class="spinner" aria-hidden="true"><LoaderCircle size={14} strokeWidth={1.8} /></span>{/if}
    <strong>{item.completed ? 'Thinking' : 'Thinking…'}</strong>
    <span class="preview">{preview}</span>
  </summary>
  <div class="reasoning-body">
    <ConversationMessage text={item.text} role="assistant" label="Reasoning" itemId={item.itemId} completed={item.completed} {onFileLink} />
  </div>
</details>

<style>
  .reasoning{color:var(--color-text-2)}
  summary{display:flex;align-items:center;gap:7px;min-height:28px;cursor:pointer;list-style:none;font-size:12px}
  summary::-webkit-details-marker{display:none}
  .chevron{display:grid;place-items:center;transition:transform .16s ease}
  details[open] .chevron{transform:rotate(90deg)}
  .spinner{display:grid;place-items:center;color:var(--color-accent);animation:spin .9s linear infinite}
  summary strong{color:var(--color-text);font-weight:620;white-space:nowrap}
  .preview{min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;opacity:.82}
  .reasoning-body{margin:7px 0 0 21px;padding:3px 0 3px 12px;border-left:1px solid color-mix(in srgb,var(--color-text-2) 40%,transparent)}
  @keyframes spin{to{transform:rotate(360deg)}}
  @media (prefers-reduced-motion:reduce){.spinner{animation:none}.chevron{transition:none}}
</style>
