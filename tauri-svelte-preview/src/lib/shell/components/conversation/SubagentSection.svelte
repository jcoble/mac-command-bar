<script lang="ts">
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import LoaderCircle from '@lucide/svelte/icons/loader-circle';
  import Users from '@lucide/svelte/icons/users';
  import type { ConversationDisplayItem } from '$lib/shell/conversation/conversationTimeline.ts';

  let { item }: { item: Extract<ConversationDisplayItem, { kind: 'subagent' }> } = $props();

  const running = $derived(/^(working|running|active|started)$/i.test(item.state));
  const failed = $derived(/^(failed|error|cancelled)$/i.test(item.state));
  const done = $derived(/^(completed|done|finished)$/i.test(item.state));

  /**
   * Everything the child event carries beyond the fields already in the
   * heading. Only plain values are shown — a nested payload has no row shape.
   */
  const details = $derived(
    Object.entries(item.metadata ?? {})
      .filter(([key, value]) =>
        !['childId', 'parentId', 'label', 'state'].includes(key) &&
        (typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean') &&
        String(value).length > 0)
      .map(([key, value]) => [key, String(value)] as const)
  );
</script>

<details class="subagent" class:running class:failed class:done data-testid="timeline-subagent-item" open={running}>
  <summary>
    <span class="chevron" aria-hidden="true"><ChevronRight size={14} strokeWidth={1.8} /></span>
    <span class="agent-icon" aria-hidden="true">
      {#if running}<span class="spinner"><LoaderCircle size={14} strokeWidth={2} /></span>{:else}<Users size={14} strokeWidth={1.8} />{/if}
    </span>
    <strong>{item.label}</strong>
    <span class="state-chip" data-testid="timeline-subagent-state">{item.state}</span>
  </summary>
  <div class="subagent-body">
    <p class="note">Read-only view of a child agent's work.</p>
    <dl>
      <div><dt>Child</dt><dd>{item.childId}</dd></div>
      {#if item.parentId}<div><dt>Parent</dt><dd>{item.parentId}</dd></div>{/if}
      {#each details as [key, value] (key)}<div><dt>{key}</dt><dd>{value}</dd></div>{/each}
    </dl>
  </div>
</details>

<style>
  .subagent{border:1px solid color-mix(in srgb,var(--color-border) 62%,transparent);border-left:2px solid color-mix(in srgb,var(--color-accent) 45%,var(--color-border));border-radius:10px;background:color-mix(in srgb,var(--color-surface) 50%,var(--color-bg) 50%);overflow:hidden}
  summary{display:flex;align-items:center;gap:8px;min-height:34px;padding:6px 10px;cursor:pointer;list-style:none}
  summary::-webkit-details-marker{display:none}
  summary:hover{background:color-mix(in srgb,var(--color-hover) 55%,transparent)}
  .chevron,.agent-icon{display:grid;place-items:center;flex:none;color:var(--color-text-2)}
  details[open] .chevron{transform:rotate(90deg)}
  strong{flex:1;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-size:13px;font-weight:620}
  .state-chip{flex:none;padding:2px 8px;border-radius:999px;background:color-mix(in srgb,var(--color-attention) 12%,transparent);color:var(--color-attention);font-size:12px;text-transform:lowercase}
  .running{border-left-color:var(--color-accent)}
  .running .agent-icon{color:var(--color-accent)}
  .running .state-chip{background:color-mix(in srgb,var(--color-accent) 14%,transparent);color:var(--color-accent)}
  .done .state-chip{background:color-mix(in srgb,var(--color-good) 13%,transparent);color:var(--color-good)}
  .failed{border-left-color:var(--color-bad)}
  .failed .state-chip{background:color-mix(in srgb,var(--color-bad) 14%,transparent);color:var(--color-bad)}
  .subagent-body{display:grid;gap:8px;padding:0 12px 12px 40px}
  .note{margin:0;color:var(--color-text-2);font-size:13px}
  dl{display:grid;gap:5px;margin:0}
  dl div{display:grid;grid-template-columns:88px minmax(0,1fr);gap:10px}
  dt{color:var(--color-text-3);font-size:13px}
  dd{margin:0;min-width:0;overflow-wrap:anywhere;font:12px/1.5 ui-monospace,SFMono-Regular,Menlo,monospace}

  @media (prefers-reduced-motion:no-preference){
    .chevron{transition:transform .14s ease}
    summary{transition:background .14s ease}
    .spinner{display:grid;place-items:center;animation:spin .9s linear infinite}
    .subagent-body{animation:body-in .14s ease-out both}
    @keyframes spin{to{transform:rotate(360deg)}}
    @keyframes body-in{from{opacity:0;transform:translateY(-2px)}to{opacity:1;transform:none}}
  }
</style>
