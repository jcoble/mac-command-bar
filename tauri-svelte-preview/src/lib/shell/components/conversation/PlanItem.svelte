<script lang="ts">
  import type { ConversationDisplayItem } from '$lib/shell/conversation/conversationTimeline.ts';
  let { item }: { item: Extract<ConversationDisplayItem, { kind: 'plan' }> } = $props();
  let expanded = $state(true);
</script>

<section class="timeline-card plan-card" data-testid="timeline-plan-item">
  <button class="card-heading" type="button" aria-expanded={expanded} onclick={() => (expanded = !expanded)}>
    <span class="card-kind">Plan</span>
    <strong>{item.title}</strong>
    <span class="count">{expanded ? 'Collapse' : `${item.steps.length} steps`}</span>
  </button>
  {#if expanded}
    <ol>
      {#each item.steps as step (step.id)}
        <li class:completed={step.state === 'completed'} class:active={step.state === 'in-progress'}>
          <span class="step-state" aria-hidden="true"></span>
          <div><strong>{step.title}</strong>{#if step.detail}<p>{step.detail}</p>{/if}<small>{step.state}</small></div>
        </li>
      {/each}
    </ol>
  {/if}
</section>

<style>
  .timeline-card{padding:12px;border:1px solid color-mix(in srgb,var(--color-border) 70%,transparent);border-radius:10px;background:color-mix(in srgb,var(--color-surface) 45%,var(--color-bg) 55%)}
  .card-heading{display:flex;align-items:center;gap:8px;width:100%;border:0;border-radius:6px;background:transparent;color:inherit;text-align:left;cursor:pointer}
  .card-heading:focus-visible{outline:2px solid var(--color-focus-solid);outline-offset:2px}
  .card-kind{color:var(--color-text-3);font-size:12px;letter-spacing:.06em;text-transform:uppercase}
  .card-heading strong{flex:1;min-width:0;font-size:13px;font-weight:620}
  .count{color:var(--color-text-3);font-size:12px}
  ol{display:grid;gap:8px;margin:12px 0 0;padding:0;list-style:none}
  li{display:flex;gap:8px;font-size:13px}
  .step-state{flex:none;width:8px;height:8px;margin-top:6px;border:1px solid var(--color-text-3);border-radius:50%}
  li.completed .step-state{background:var(--color-accent);border-color:var(--color-accent)}
  li.active .step-state{box-shadow:0 0 0 3px color-mix(in srgb,var(--color-accent) 18%,transparent)}
  p{margin:4px 0 0;color:var(--color-text-2);font-size:12px}
  small{color:var(--color-text-3);font-size:12px}
</style>
