<script lang="ts">
  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import type { AgentConfigValue } from '$lib/shell/conversation/conversationTypes.ts';
  import type { ConversationDisplayItem } from '$lib/shell/conversation/conversationTimeline.ts';

  interface Props {
    item: Extract<ConversationDisplayItem, { kind: 'input' }>;
    onSubmit?(requestId: string, values: Record<string, AgentConfigValue>, cancelled?: boolean): void;
  }

  let { item, onSubmit }: Props = $props();
  let values = $state<Record<string, AgentConfigValue>>({});

  function submit(cancelled = false): void {
    onSubmit?.(item.requestId, values, cancelled);
  }

  function chooseValue(fieldId: string, value: AgentConfigValue): void {
    values[fieldId] = value;
  }
</script>

<section class="input-event" data-testid="timeline-user-input-item">
  <strong class="input-title">{item.title}</strong>
  {#if item.description}<p>{item.description}</p>{/if}
  <form onsubmit={(event) => { event.preventDefault(); submit(); }}>
    {#each item.fields as field}
      <fieldset class="input-field">
        <legend>{String(field.label ?? field.id)}</legend>
        {#if field.description}<small>{String(field.description)}</small>{/if}
        {#if field.kind === 'select'}
          <div class="choice-list" role="radiogroup" aria-label={String(field.label ?? field.id)}>
            {#each field.choices ?? [] as choice (JSON.stringify(choice.value))}
              {@const selected = JSON.stringify(values[field.id] ?? field.choices?.[0]?.value ?? '') === JSON.stringify(choice.value)}
              <button class:selected type="button" role="radio" aria-checked={selected} onclick={() => chooseValue(field.id, choice.value)}>{choice.label}</button>
            {/each}
          </div>
        {:else if field.kind === 'boolean'}
          <label class="boolean-field">
            <input type="checkbox" checked={values[field.id] === true} onchange={(event) => (values[field.id] = event.currentTarget.checked)} />Yes
          </label>
        {:else}
          <Input
            required={field.required !== false}
            type={field.kind === 'password' ? 'password' : 'text'}
            value={String(values[field.id] ?? '')}
            oninput={(event) => (values[field.id] = event.currentTarget.value)}
          />
        {/if}
      </fieldset>
    {/each}
    <div class="input-actions">
      <Button size="sm" data-testid="structured-input-submit" type="submit">Submit</Button>
      <Button variant="ghost" size="sm" data-testid="structured-input-cancel" onclick={() => submit(true)}>Cancel</Button>
    </div>
  </form>
</section>

<style>
  .input-event{padding:12px;border:1px solid color-mix(in srgb,var(--color-accent) 30%,var(--color-border));border-radius:10px;background:color-mix(in srgb,var(--color-accent) 6%,transparent)}
  .input-title{font-size:13px;font-weight:620}
  p{margin:4px 0 0;color:var(--color-text-2);font-size:12px}
  form{display:grid;gap:12px;margin-top:12px}
  .input-field{display:grid;gap:4px;margin:0;padding:0;border:0}
  legend{font-size:12px}
  fieldset small{color:var(--color-text-3);font-size:12px}
  .choice-list{display:grid;gap:4px}
  .choice-list button{min-height:28px;padding:4px 8px;border:1px solid transparent;border-radius:6px;background:var(--color-bg);color:inherit;text-align:left;font:13px/1.25 inherit;cursor:pointer}
  .choice-list button:hover,.choice-list button.selected{border-color:color-mix(in srgb,var(--color-accent) 32%,var(--color-border));background:color-mix(in srgb,var(--color-accent) 9%,var(--color-surface))}
  .choice-list button:focus-visible{outline:2px solid var(--color-focus-solid);outline-offset:1px}
  input[type="checkbox"]{width:15px;accent-color:var(--color-accent)}
  .boolean-field{display:inline-flex;align-items:center;gap:8px;color:var(--color-text-2);font-size:12px}
  .input-actions{display:flex;gap:8px}
</style>
