<script lang="ts">
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
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
    <strong>{item.completed ? 'Thinking' : 'Thinking…'}</strong>
    <span class="preview">{preview}</span>
  </summary>
  <div class="reasoning-body">
    <ConversationMessage text={item.text} role="assistant" itemId={item.itemId} completed={item.completed} {onFileLink} />
  </div>
</details>

<style>
  .reasoning{color:var(--color-text-2);border-radius:8px;padding:2px 6px;margin:0 -6px}
  .reasoning:hover{background:color-mix(in srgb,var(--color-surface) 40%,transparent)}
  summary{display:flex;align-items:center;gap:8px;min-height:28px;cursor:pointer;list-style:none;font-size:12px}
  summary::-webkit-details-marker{display:none}
  summary:focus-visible{outline:2px solid var(--color-focus-solid);outline-offset:-2px}
  .chevron{display:grid;place-items:center;flex:none;color:var(--color-text-3)}
  details[open] .chevron{transform:rotate(90deg)}
  summary strong{flex:none;color:var(--color-text);font-weight:620;white-space:nowrap}
  .preview{min-width:0;flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;color:var(--color-text-3)}
  .reasoning-body{margin:8px 0 4px 22px;padding:4px 0 4px 12px;border-left:1px solid color-mix(in srgb,var(--color-accent) 28%,var(--color-border))}
  @media (prefers-reduced-motion:no-preference){
    .chevron{transition:transform .14s ease}
    .reasoning{transition:background .14s ease}
  }
</style>
