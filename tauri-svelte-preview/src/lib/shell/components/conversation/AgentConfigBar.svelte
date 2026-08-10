<script lang="ts">
  import type {
    AgentConversationConfigField,
    AgentConversationConfigState
  } from '$lib/shell/conversation/conversationConfig.ts';

  interface Props {
    provider: string;
    state: AgentConversationConfigState;
    pending: Partial<Record<AgentConversationConfigField, string>>;
    error?: string | null;
    onChange?(field: AgentConversationConfigField, value: string): void;
  }

  let { provider, state, pending, error = null, onChange }: Props = $props();

  function change(field: AgentConversationConfigField, event: Event): void {
    onChange?.(field, (event.currentTarget as HTMLSelectElement).value);
  }
</script>

<div class="config-bar" data-testid="conversation-config-bar" aria-label="Conversation settings">
  <span class="provider" data-testid="conversation-provider-label">{provider}</span>

  <label data-testid="conversation-config-control" title={state.availableModels.length ? 'Choose the model for this conversation' : 'Model choices are not available'}>
    <span>Model</span>
    <select
      data-testid="conversation-config-model"
      aria-label="Model"
      disabled={!state.availableModels.length || 'model' in pending}
      value={state.model ?? ''}
      onchange={(event) => change('model', event)}
    >
      {#if state.model && !state.availableModels.includes(state.model)}<option value={state.model}>{state.model}</option>{/if}
      {#if !state.availableModels.length}<option value="">Unavailable</option>{/if}
      {#each state.availableModels as model (model)}<option value={model}>{model}</option>{/each}
    </select>
  </label>

  <label data-testid="conversation-config-control" title={state.availableEfforts.length ? 'Choose how much reasoning to use' : 'Reasoning choices are not available'}>
    <span>Reasoning effort</span>
    <select
      data-testid="conversation-config-reasoning-effort"
      aria-label="Reasoning effort"
      disabled={!state.availableEfforts.length || 'reasoningEffort' in pending}
      value={state.reasoningEffort ?? ''}
      onchange={(event) => change('reasoningEffort', event)}
    >
      {#if state.reasoningEffort && !state.availableEfforts.includes(state.reasoningEffort)}<option value={state.reasoningEffort}>{state.reasoningEffort}</option>{/if}
      {#if !state.availableEfforts.length}<option value="">Unavailable</option>{/if}
      {#each state.availableEfforts as effort (effort)}<option value={effort}>{effort}</option>{/each}
    </select>
  </label>

  <label data-testid="conversation-config-control" title={state.availableApprovalPolicies.length ? 'Choose when tools require approval' : 'Approval choices are not available'}>
    <span>Approvals</span>
    <select
      data-testid="conversation-config-approval-policy"
      aria-label="Approvals"
      disabled={!state.availableApprovalPolicies.length || 'approvalPolicy' in pending}
      value={state.approvalPolicy ?? ''}
      onchange={(event) => change('approvalPolicy', event)}
    >
      {#if state.approvalPolicy && !state.availableApprovalPolicies.includes(state.approvalPolicy)}<option value={state.approvalPolicy}>{state.approvalPolicy}</option>{/if}
      {#if !state.availableApprovalPolicies.length}<option value="">Unavailable</option>{/if}
      {#each state.availableApprovalPolicies as policy (policy)}<option value={policy}>{policy}</option>{/each}
    </select>
  </label>

  {#if Object.keys(pending).length}<span class="saving" data-testid="conversation-config-saving">Saving…</span>{/if}
  {#if error}<span class="error" data-testid="conversation-config-error" title={error}>Configuration unavailable</span>{/if}
</div>

<style>
  .config-bar{display:flex;align-items:center;gap:10px;min-height:34px;overflow:auto;padding:0 3px;color:var(--color-text-2);font-size:12px;scrollbar-width:thin}
  .provider{padding-right:2px;color:var(--color-text);font-weight:650;text-transform:capitalize}
  label{display:flex;align-items:center;gap:5px;white-space:nowrap}
  select{max-width:190px;border:0;border-radius:6px;background:color-mix(in srgb,var(--color-surface) 76%,transparent);color:inherit;padding:4px 22px 4px 7px;font:inherit}
  select:disabled{cursor:not-allowed;opacity:.55}
  select:focus{outline:2px solid var(--color-focus-solid);outline-offset:1px}
  .saving{color:var(--color-accent)}
  .error{margin-left:auto;color:var(--color-bad);white-space:nowrap}
</style>
