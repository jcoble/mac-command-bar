<script lang="ts">
  import type { AgentCapabilities, AgentConfigOption, AgentConfigValue } from '$lib/shell/conversation/conversationTypes.ts';
  import { configOptionPlacement } from '$lib/shell/conversation/conversationTypes.ts';

  interface Props {
    capabilities: AgentCapabilities | null;
    config: Record<string, AgentConfigValue>;
    pending: Record<string, AgentConfigValue>;
    errors: Record<string, string>;
    onChange?(optionId: string, value: AgentConfigValue): void;
  }
  let { capabilities, config, pending, errors, onChange }: Props = $props();

  const options = $derived(capabilities?.configOptions ?? []);
  const primary = $derived(options.filter((option) => ['model-picker', 'reasoning-picker', 'mode-picker'].includes(configOptionPlacement(option.category))));
  const secondary = $derived(options.filter((option) => !primary.some((candidate) => candidate.id === option.id)));

  function currentValue(option: AgentConfigOption): AgentConfigValue {
    return pending[option.id] ?? config[option.id] ?? option.value;
  }

  function optionValue(event: Event): AgentConfigValue | null {
    const value = (event.currentTarget as HTMLSelectElement).value;
    try { return JSON.parse(value) as AgentConfigValue; } catch { return null; }
  }

  function unsupportedReason(option: AgentConfigOption): string | null {
    if (!capabilities) return 'The provider capability snapshot is unavailable';
    const category = `${option.category} ${option.id}`.toLowerCase();
    if ((category.includes('permission') || category.includes('approval')) && !capabilities.interaction.permissions) {
      return 'This provider does not support permission controls for the session';
    }
    return null;
  }

  function change(option: AgentConfigOption, event: Event): void {
    const value = optionValue(event);
    if (value !== null) onChange?.(option.id, value);
  }
</script>

<div class="config-bar" data-testid="conversation-config-bar" aria-label="Conversation settings">
  <span class="provider" data-testid="conversation-provider-label">{capabilities?.provider ?? 'Provider'}{#if capabilities}<small>{capabilities.implementation.name}</small>{/if}</span>
  {#if capabilities}
    {#each primary as option (option.id)}
      <label class:pending={option.id in pending} data-testid="conversation-config-control" title={unsupportedReason(option) ?? option.description ?? ''}><span>{option.label}</span>
        {#if option.choices?.length}
          <select data-testid={`conversation-config-${option.id}`} aria-label={option.label} disabled={!!unsupportedReason(option)} value={JSON.stringify(currentValue(option))} onchange={(event) => change(option, event)}>{#each option.choices as choice (JSON.stringify(choice.value))}<option value={JSON.stringify(choice.value)}>{choice.label}</option>{/each}</select>
        {:else}<span class="read-only" title={unsupportedReason(option) ?? option.description ?? 'The provider did not advertise selectable values'}>{String(currentValue(option))}</span>{/if}
        {#if option.id in pending}<em>pending</em>{/if}{#if errors[option.id]}<small class="error">{errors[option.id]}</small>{/if}
      </label>
    {/each}
    {#if secondary.length}<details class="more-options" data-testid="conversation-more-options"><summary>Options</summary><div class="secondary-options">{#each secondary as option (option.id)}<label class:pending={option.id in pending} data-testid="conversation-config-control" title={unsupportedReason(option) ?? option.description ?? ''}><span>{option.label}</span>{#if option.choices?.length}<select data-testid={`conversation-config-${option.id}`} aria-label={option.label} disabled={!!unsupportedReason(option)} value={JSON.stringify(currentValue(option))} onchange={(event) => change(option, event)}>{#each option.choices as choice (JSON.stringify(choice.value))}<option value={JSON.stringify(choice.value)}>{choice.label}</option>{/each}</select>{:else}<span class="read-only" title={unsupportedReason(option) ?? option.description ?? 'The provider did not advertise selectable values'}>{String(currentValue(option))}</span>{/if}{#if option.id in pending}<em>pending</em>{/if}{#if errors[option.id]}<small class="error">{errors[option.id]}</small>{/if}</label>{/each}</div></details>{/if}
  {:else}<span class="unavailable" data-testid="conversation-config-unavailable">Configuration options are unavailable until the provider advertises them.</span>{/if}
</div>

<style>.config-bar{display:flex;align-items:center;gap:7px;min-height:34px;overflow:auto;color:var(--color-text-2);font-size:12px}.provider{display:flex;align-items:baseline;gap:6px;padding:0 5px;color:var(--color-text);font-weight:650}.provider small{color:var(--color-text-2);font-size:12px;font-weight:500}.config-bar label{display:flex;align-items:center;gap:5px;white-space:nowrap}.config-bar select,.read-only{max-width:190px;border:0;border-radius:6px;background:color-mix(in srgb,var(--color-surface) 75%,transparent);color:inherit;padding:4px 6px;font:inherit}.config-bar select:focus{outline:2px solid var(--color-focus-solid);outline-offset:1px}.pending{color:var(--color-text)}.pending em{color:var(--color-accent);font-size:12px}.error{color:var(--color-bad);font-size:12px}.more-options{margin-left:auto;white-space:nowrap}.more-options summary{cursor:pointer;padding:4px 6px}.secondary-options{position:absolute;z-index:3;display:grid;gap:8px;margin-top:4px;padding:10px;border:1px solid var(--color-border);border-radius:9px;background:var(--color-surface);box-shadow:var(--shadow-md)}.secondary-options label{display:flex;justify-content:space-between;gap:12px}.unavailable{font-size:12px}</style>
