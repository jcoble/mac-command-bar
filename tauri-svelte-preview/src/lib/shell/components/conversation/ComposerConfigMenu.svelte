<!--
  ComposerConfigMenu.svelte — the strip along the bottom of the message box
  where the agent's settings live.

  It replaces the old row of dropdowns above the composer. The settings are
  the same three the agent reports (model, reasoning effort, approval policy)
  and they are written through the same call; what changed is that they are
  now stated as sentences rather than raw ids, and take one line instead of
  four.

  Two controls, because the two questions are different:
    left  — how much the agent may do without asking. A decision about this
            conversation's safety, so it sits where the eye starts.
    right — which model, and how hard it is thinking. Read as one phrase
            ("5.6 Sol Extra High") on a quiet pill, opened for the rare change.

  Every label comes from `agentConfigLabels.ts`; nothing here invents wording.
-->
<script lang="ts">
  import Check from '@lucide/svelte/icons/check';
  import ChevronDown from '@lucide/svelte/icons/chevron-down';
  import ShieldCheck from '@lucide/svelte/icons/shield-check';

  import { Button } from '$lib/components/ui/button/index.js';
  import * as DropdownMenu from '$lib/components/ui/dropdown-menu/index.js';
  import {
    approvalDescription,
    approvalLabel,
    effortLabel,
    modelEffortLabel,
    modelLabel
  } from '$lib/shell/conversation/agentConfigLabels.ts';
  import {
    hasAgentConversationConfig,
    type AgentConversationConfigField,
    type AgentConversationConfigState
  } from '$lib/shell/conversation/conversationConfig.ts';

  interface Props {
    /** The agent this conversation is with, used to name the settings menu. */
    provider: string;
    state: AgentConversationConfigState;
    /** Fields whose new value is still being written. Those controls are held. */
    pending: Partial<Record<AgentConversationConfigField, string>>;
    error?: string | null;
    onChange?(field: AgentConversationConfigField, value: string): void;
  }

  let { provider, state, pending, error = null, onChange }: Props = $props();

  const modelBusy = $derived('model' in pending);
  const effortBusy = $derived('reasoningEffort' in pending);
  const approvalBusy = $derived('approvalPolicy' in pending);
  const saving = $derived(Object.keys(pending).length > 0);

  /** Which individual controls have choices the agent can accept. */
  const canChooseModel = $derived(state.availableModels.length > 0);
  const canChooseEffort = $derived(state.availableEfforts.length > 0);
  const canChooseApproval = $derived(state.availableApprovalPolicies.length > 0);
  const hasAgentSettings = $derived(hasAgentConversationConfig(state));

  /**
   * The options to show, with whatever the agent currently reports included
   * even when it is missing from the list — otherwise the menu would show a
   * check mark against nothing.
   */
  function optionsFor(available: readonly string[], current: string | null): string[] {
    if (current && !available.includes(current)) return [current, ...available];
    return [...available];
  }

  const modelOptions = $derived(optionsFor(state.availableModels, state.model));
  const effortOptions = $derived(optionsFor(state.availableEfforts, state.reasoningEffort));
  const approvalOptions = $derived(
    optionsFor(state.availableApprovalPolicies, state.approvalPolicy)
  );
</script>

<div class="config-row" data-testid="conversation-config-bar" aria-label="Agent settings">
  {#if !hasAgentSettings}
    <span class="unavailable" data-testid="conversation-config-unavailable">
      Agent settings unavailable
    </span>
  {:else}
    <!-- Left: what the agent may do on its own. -->
    <DropdownMenu.Root>
    <DropdownMenu.Trigger disabled={!canChooseApproval || approvalBusy}>
      {#snippet child({ props })}
        <Button
          {...props}
          data-testid="conversation-config-approval-policy"
          variant="ghost"
          size="xs"
          class="text-muted-foreground hover:text-foreground gap-1 px-1.5 text-[12px] font-normal"
        >
          <ShieldCheck aria-hidden="true" />
          {approvalLabel(state.approvalPolicy)}
          <ChevronDown aria-hidden="true" class="size-3 opacity-70" />
        </Button>
      {/snippet}
    </DropdownMenu.Trigger>
    <!-- Opens upward. This strip sits on the bottom edge of the window, so a
         menu dropped below the trigger has nowhere to go and is cut off by the
         edge; `avoidCollisions` then flips it back down only if the space
         above ever runs out too. -->
    <DropdownMenu.Content
      side="top"
      align="start"
      sideOffset={8}
      avoidCollisions
      collisionPadding={12}
      class="w-auto! max-w-[320px] min-w-[248px]"
    >
      <DropdownMenu.Label>When {provider} needs permission</DropdownMenu.Label>
      {#each approvalOptions as policy (policy)}
        <DropdownMenu.Item
          class="items-start gap-2 py-1.5"
          onSelect={() => onChange?.('approvalPolicy', policy)}
        >
          <span class="mt-0.5 flex size-3.5 shrink-0 items-center justify-center">
            {#if policy === state.approvalPolicy}<Check aria-hidden="true" class="size-3.5" />{/if}
          </span>
          <span class="flex min-w-0 flex-col gap-0.5">
            <span>{approvalLabel(policy)}</span>
            {#if approvalDescription(policy)}
              <span class="text-muted-foreground text-[12px] leading-[1.4] whitespace-normal">
                {approvalDescription(policy)}
              </span>
            {/if}
          </span>
        </DropdownMenu.Item>
      {/each}
    </DropdownMenu.Content>
    </DropdownMenu.Root>

    <!-- Right: which model, thinking how hard. -->
    <div class="config-right">
    {#if saving}
      <span class="text-primary text-[12px]" data-testid="conversation-config-saving">Saving…</span>
    {/if}
    {#if error}
      <span
        class="text-destructive text-[12px]"
        data-testid="conversation-config-error"
        title={error}
      >
        Settings unavailable
      </span>
    {/if}
    <DropdownMenu.Root>
      <DropdownMenu.Trigger disabled={!canChooseModel && !canChooseEffort}>
        {#snippet child({ props })}
          <Button
            {...props}
            data-testid="conversation-config-pill"
            variant="ghost"
            size="xs"
            class="border-border/70 text-muted-foreground hover:text-foreground gap-1 rounded-full border px-2.5 text-[12px] font-normal"
          >
            {modelEffortLabel(state.model, state.reasoningEffort)}
            <ChevronDown aria-hidden="true" class="size-3 opacity-70" />
          </Button>
        {/snippet}
      </DropdownMenu.Trigger>
      <!-- Same reason as the approval menu: upward, away from the window edge. -->
      <DropdownMenu.Content
        side="top"
        align="end"
        sideOffset={8}
        avoidCollisions
        collisionPadding={12}
        class="w-auto! min-w-[220px]"
      >
        <DropdownMenu.Sub>
          <DropdownMenu.SubTrigger
            data-testid="conversation-config-model"
            disabled={!canChooseModel || modelBusy}
          >
            Model
            <span class="text-muted-foreground ml-auto pl-4">{modelLabel(state.model)}</span>
          </DropdownMenu.SubTrigger>
          <!-- A submenu hangs off its row, near the bottom of the window: its
               bottom edge is pinned to the row's so it grows upward. -->
          <DropdownMenu.SubContent
            align="end"
            avoidCollisions
            collisionPadding={12}
            class="min-w-[180px]"
          >
            {#each modelOptions as model (model)}
              <DropdownMenu.Item onSelect={() => onChange?.('model', model)}>
                <span class="flex size-3.5 shrink-0 items-center justify-center">
                  {#if model === state.model}<Check aria-hidden="true" class="size-3.5" />{/if}
                </span>
                {modelLabel(model)}
              </DropdownMenu.Item>
            {/each}
          </DropdownMenu.SubContent>
        </DropdownMenu.Sub>

        <DropdownMenu.Sub>
          <DropdownMenu.SubTrigger
            data-testid="conversation-config-reasoning-effort"
            disabled={!canChooseEffort || effortBusy}
          >
            Effort
            <span class="text-muted-foreground ml-auto pl-4">
              {effortLabel(state.reasoningEffort)}
            </span>
          </DropdownMenu.SubTrigger>
          <!-- A submenu hangs off its row, near the bottom of the window: its
               bottom edge is pinned to the row's so it grows upward. -->
          <DropdownMenu.SubContent
            align="end"
            avoidCollisions
            collisionPadding={12}
            class="min-w-[180px]"
          >
            {#each effortOptions as effort (effort)}
              <DropdownMenu.Item onSelect={() => onChange?.('reasoningEffort', effort)}>
                <span class="flex size-3.5 shrink-0 items-center justify-center">
                  {#if effort === state.reasoningEffort}
                    <Check aria-hidden="true" class="size-3.5" />
                  {/if}
                </span>
                {effortLabel(effort)}
              </DropdownMenu.Item>
            {/each}
          </DropdownMenu.SubContent>
        </DropdownMenu.Sub>
      </DropdownMenu.Content>
    </DropdownMenu.Root>
    </div>
  {/if}
</div>

<style>
  /* One 24px line under the text area, inside the composer's own border. */
  .config-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    min-height: 24px;
  }
  .config-right {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .unavailable {
    color: var(--color-text-3);
    font-size: 12px;
  }
</style>
