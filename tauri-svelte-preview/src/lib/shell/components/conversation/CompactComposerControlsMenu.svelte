<script lang="ts">
  import Check from '@lucide/svelte/icons/check';
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import Ellipsis from '@lucide/svelte/icons/ellipsis';
  import { Button } from '$lib/components/ui/button/index.js';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import { approvalLabel, effortLabel, modelLabel } from '$lib/shell/conversation/agentConfigLabels.ts';
  import type { AgentConversationConfigField, AgentConversationConfigState } from '$lib/shell/conversation/conversationConfig.ts';

  interface Props {
    provider: string;
    state: AgentConversationConfigState;
    pending: Partial<Record<AgentConversationConfigField, string>>;
    onChange?(field: AgentConversationConfigField, value: string): void;
  }

  let { provider, state, pending, onChange }: Props = $props();
  const optionsFor = (available: readonly string[], current: string | null): string[] => current && !available.includes(current) ? [current, ...available] : [...available];
</script>

<DropdownMenu.Root>
  <DropdownMenu.Trigger>
    {#snippet child({ props })}
      <Button {...props} variant="ghost" size="sm" class="more-trigger" aria-label="More composer controls">
        <Ellipsis size={16} aria-hidden="true" />
        <ChevronDown class="more-chevron" size={12} aria-hidden="true" />
      </Button>
    {/snippet}
  </DropdownMenu.Trigger>
  <DropdownMenu.Content side="top" align="start" sideOffset={8} class="compact-menu">
    <DropdownMenu.Label>Composer controls</DropdownMenu.Label>
    {#if state.availableModels.length || state.model}
      <DropdownMenu.Label>Model</DropdownMenu.Label>
      {#each optionsFor(state.availableModels, state.model) as model (model)}
        <DropdownMenu.Item disabled={'model' in pending} onSelect={() => onChange?.('model', model)}>
          <span class="check-slot">{#if model === state.model}<Check size={13} aria-hidden="true" />{/if}</span>{modelLabel(model)}
        </DropdownMenu.Item>
      {/each}
    {/if}
    {#if state.availableEfforts.length || state.reasoningEffort}
      <DropdownMenu.Label>Thinking effort</DropdownMenu.Label>
      {#each optionsFor(state.availableEfforts, state.reasoningEffort) as effort (effort)}
        <DropdownMenu.Item disabled={'reasoningEffort' in pending} onSelect={() => onChange?.('reasoningEffort', effort)}>
          <span class="check-slot">{#if effort === state.reasoningEffort}<Check size={13} aria-hidden="true" />{/if}</span>{effortLabel(effort)}
        </DropdownMenu.Item>
      {/each}
    {/if}
    {#if state.availableApprovalPolicies.length || state.approvalPolicy}
      <DropdownMenu.Label>Access for {provider}</DropdownMenu.Label>
      {#each optionsFor(state.availableApprovalPolicies, state.approvalPolicy) as policy (policy)}
        <DropdownMenu.Item disabled={'approvalPolicy' in pending} onSelect={() => onChange?.('approvalPolicy', policy)}>
          <span class="check-slot">{#if policy === state.approvalPolicy}<Check size={13} aria-hidden="true" />{/if}</span>{approvalLabel(policy)}
        </DropdownMenu.Item>
      {/each}
    {/if}
  </DropdownMenu.Content>
</DropdownMenu.Root>

<style>
  :global(.more-trigger) { min-width: 30px; gap: 1px; padding: 0 6px; color: var(--color-text-2); }
  :global(.more-trigger:hover) { color: var(--color-text); }
  :global(.more-chevron) { opacity: .6; }
  :global(.compact-menu) { min-width: 230px; }
  .check-slot { display: inline-grid; place-items: center; width: 15px; margin-right: 2px; color: var(--color-accent); }
</style>
