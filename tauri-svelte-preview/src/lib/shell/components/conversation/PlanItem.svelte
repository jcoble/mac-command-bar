<script lang="ts">
  import type { ConversationDisplayItem } from '$lib/shell/conversation/conversationTimeline.ts';
  let { item }: { item: Extract<ConversationDisplayItem, { kind: 'plan' }> } = $props();
  let expanded = $state(true);
</script>

<section class="timeline-card plan-card" data-testid="timeline-plan-item">
  <button class="card-heading" type="button" aria-expanded={expanded} onclick={() => (expanded = !expanded)}><span>Plan</span><strong>{item.title}</strong><span class="muted">{expanded ? 'Collapse' : `${item.steps.length} steps`}</span></button>
  {#if expanded}<ol>{#each item.steps as step (step.id)}<li class:completed={step.state === 'completed'} class:active={step.state === 'in-progress'}><span class="step-state" aria-hidden="true"></span><div><strong>{step.title}</strong>{#if step.detail}<p>{step.detail}</p>{/if}<small>{step.state}</small></div></li>{/each}</ol>{/if}
</section>

<style>.timeline-card{padding:12px 14px;border:1px solid var(--color-border);border-radius:10px;background:color-mix(in srgb,var(--color-surface) 72%,transparent)}.card-heading{display:flex;align-items:center;gap:9px;width:100%;border:0;background:transparent;color:inherit;text-align:left}.card-heading span:first-child{color:var(--color-accent);font-size:12px;font-weight:700;text-transform:uppercase;letter-spacing:.04em}.card-heading strong{flex:1}.muted{color:var(--color-text-2);font-size:12px}.plan-card ol{display:grid;gap:11px;margin:14px 0 0;padding:0;list-style:none}.plan-card li{display:flex;gap:9px}.step-state{width:9px;height:9px;margin-top:6px;border:1px solid var(--color-text-2);border-radius:50%}.plan-card li.completed .step-state{background:var(--color-accent);border-color:var(--color-accent)}.plan-card li.active .step-state{box-shadow:0 0 0 3px color-mix(in srgb,var(--color-accent) 18%,transparent)}.plan-card p{margin:3px 0;color:var(--color-text-2);font-size:12px}.plan-card small{color:var(--color-text-2);font-size:12px}</style>
