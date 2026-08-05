<script lang="ts">
  import type { AgentConfigValue } from '$lib/shell/conversation/conversationTypes.ts';
  import type { ConversationDisplayItem } from '$lib/shell/conversation/conversationTimeline.ts';
  interface Props { item: Extract<ConversationDisplayItem, { kind: 'input' }>; onSubmit?(requestId: string, values: Record<string, AgentConfigValue>): void; }
  let { item, onSubmit }: Props = $props();
  let values = $state<Record<string, AgentConfigValue>>({});
  function submit(): void { onSubmit?.(item.requestId, values); }
  function selectValue(event: Event): void {
    const fieldId = (event.currentTarget as HTMLSelectElement).dataset.fieldId;
    if (!fieldId) return;
    try { values[fieldId] = JSON.parse((event.currentTarget as HTMLSelectElement).value) as AgentConfigValue; }
    catch { values[fieldId] = (event.currentTarget as HTMLSelectElement).value; }
  }
</script>

<section class="timeline-event input-event" data-testid="timeline-user-input-item"><strong>{item.title}</strong>{#if item.description}<p>{item.description}</p>{/if}<form onsubmit={(event) => { event.preventDefault(); submit(); }}>{#each item.fields as field}<label data-testid="structured-input-field">{String(field.label ?? field.id)}{#if field.description}<small>{String(field.description)}</small>{/if}{#if field.kind === 'select'}<select required={field.required !== false} data-field-id={field.id} value={JSON.stringify(values[field.id] ?? field.choices?.[0]?.value ?? '')} onchange={selectValue}>{#each field.choices ?? [] as choice (JSON.stringify(choice.value))}<option value={JSON.stringify(choice.value)}>{choice.label}</option>{/each}</select>{:else if field.kind === 'boolean'}<input type="checkbox" checked={values[field.id] === true} onchange={(event) => (values[field.id] = event.currentTarget.checked)} />{:else}<input required={field.required !== false} type={field.kind === 'password' ? 'password' : 'text'} value={String(values[field.id] ?? '')} oninput={(event) => (values[field.id] = event.currentTarget.value)} />{/if}</label>{/each}<button data-testid="structured-input-submit" type="submit">Submit</button></form></section>

<style>.timeline-event{padding:10px 12px;border-left:2px solid color-mix(in srgb,var(--color-accent) 50%,var(--color-border));background:color-mix(in srgb,var(--color-surface) 45%,transparent)}p{margin:5px 0;color:var(--color-text-2)}form{display:grid;gap:9px;margin-top:10px}label{display:grid;gap:4px;font-size:12px}label small{color:var(--color-text-2);font-size:12px}input{min-height:30px;padding:5px 7px;border:1px solid var(--color-border);border-radius:6px;background:var(--color-bg);color:inherit}form button{justify-self:start;border:1px solid var(--color-border);border-radius:7px;background:var(--color-surface);color:inherit;padding:6px 9px}</style>
