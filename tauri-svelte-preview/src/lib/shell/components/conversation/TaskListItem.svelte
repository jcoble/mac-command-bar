<script lang="ts">
  import type { ConversationDisplayItem } from '$lib/shell/conversation/conversationTimeline.ts';
  let { item }: { item: Extract<ConversationDisplayItem, { kind: 'tasks' }> } = $props();
</script>

<section class="timeline-card" data-testid="timeline-task-list-item"><div class="card-heading"><span>Tasks</span><strong>{item.title}</strong><span class="muted">{item.tasks.filter((task) => task.state === 'completed').length}/{item.tasks.length}</span></div><ul>{#each item.tasks as task (task.id)}<li class:completed={task.state === 'completed'}><span class="task-state" aria-hidden="true">{task.state === 'completed' ? '✓' : '·'}</span><span>{task.title}</span><small>{task.state}</small></li>{/each}</ul></section>

<style>.timeline-card{padding:12px 14px;border:1px solid var(--color-border);border-radius:10px;background:color-mix(in srgb,var(--color-surface) 72%,transparent)}.card-heading{display:flex;align-items:center;gap:9px}.card-heading span:first-child{color:var(--color-accent);font-size:12px;font-weight:700;text-transform:uppercase;letter-spacing:.04em}.card-heading strong{flex:1}.muted,small{color:var(--color-text-2);font-size:12px}.timeline-card ul{display:grid;gap:8px;margin:13px 0 0;padding:0;list-style:none}.timeline-card li{display:flex;align-items:center;gap:8px}.timeline-card li span:nth-child(2){flex:1}.timeline-card li.completed span:nth-child(2){text-decoration:line-through;color:var(--color-text-2)}.task-state{width:15px;color:var(--color-accent);font-weight:700}</style>
