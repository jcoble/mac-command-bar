<script lang="ts">
  import AssistanceAction from './AssistanceAction.svelte';
  import AssistanceProposal from './AssistanceProposal.svelte';
  import {
    clearAssistance,
    assistanceState,
    setAssistanceError
  } from './assistanceStore.svelte.ts';
  import type { AssistanceProposal as AssistanceProposalType, AssistanceRecipeId, AssistanceSurface } from './assistanceTypes.ts';

  interface Props {
    proposal?: AssistanceProposalType | null;
    onRequest?: (recipeId: AssistanceRecipeId, surface: AssistanceSurface) => void;
    onApply?: (selectedPatchIds: string[]) => void;
    onRetry?: () => void;
  }

  let { proposal = null, onRequest, onApply, onRetry }: Props = $props();
  let open = $state(false);

  const contextualActions: Array<{ recipeId: AssistanceRecipeId; surface: AssistanceSurface; label: string }> = [
    { recipeId: 'workspace-summary', surface: 'conversation', label: 'Explain this conversation' },
    { recipeId: 'commit', surface: 'git', label: 'Draft a commit message' },
    { recipeId: 'diff-explanation', surface: 'diff', label: 'Explain this diff' },
    { recipeId: 'problem-explanation', surface: 'problems', label: 'Explain this problem' },
    { recipeId: 'run-config-review', surface: 'run-configuration', label: 'Review this run configuration' },
    { recipeId: 'browser-feedback', surface: 'browser', label: 'Draft browser feedback' },
    { recipeId: 'workspace-summary', surface: 'context', label: 'Summarize context' },
    { recipeId: 'form-suggestion', surface: 'form', label: 'Suggest a form value' },
    { recipeId: 'save-spec-review', surface: 'save', label: 'Review before saving' },
    { recipeId: 'worktree-cleanup', surface: 'worktree', label: 'Plan worktree cleanup' }
  ];

  function request(recipeId: AssistanceRecipeId, surface: AssistanceSurface): void {
    onRequest?.(recipeId, surface);
  }

  function applySelected(selectedPatchIds: string[]): void {
    if (onApply) onApply(selectedPatchIds);
    else setAssistanceError('No typed surface adapter is connected yet');
  }
</script>

<aside class="assistance-host" data-testid="assistance-host" aria-label="Assistance">
  {#if proposal || assistanceState.proposal}
    <AssistanceProposal
      proposal={proposal ?? assistanceState.proposal!}
      onApply={applySelected}
      onDismiss={clearAssistance}
      onRetry={() => onRetry?.()}
      onContinue={clearAssistance}
    />
  {:else}
    <button
      type="button"
      class="assistance-trigger"
      data-testid="assistance-trigger"
      aria-expanded={open}
      onclick={() => (open = !open)}
    >
      <span aria-hidden="true">✦</span>
      Assistance
    </button>
    {#if open}
      <div class="assistance-menu" role="menu" data-testid="assistance-contextual-actions">
        {#each contextualActions as item (item.surface)}
          <AssistanceAction {...item} onRequest={request} />
        {/each}
      </div>
    {/if}
  {/if}
</aside>

<style>
  .assistance-host { position: fixed; right: 16px; bottom: 80px; z-index: 56; display: grid; justify-items: end; gap: 0.5rem; pointer-events: none; }
  .assistance-host :global(button), .assistance-host :global(section) { pointer-events: auto; }
  .assistance-trigger { display: inline-flex; align-items: center; gap: 0.4rem; min-height: 36px; border: 0; border-radius: 999px; padding: 0.4rem 0.75rem; background: var(--color-accent, #6ed8be); color: var(--color-on-accent, #10201b); cursor: pointer; font: inherit; box-shadow: var(--shadow-md, 0 4px 16px rgba(0, 0, 0, 0.55)); }
  .assistance-trigger:hover, .assistance-trigger:focus-visible { filter: brightness(1.04); outline: none; box-shadow: var(--focus-ring, 0 0 0 3px rgba(110, 216, 190, 0.42)); }
  .assistance-menu { display: grid; gap: 0.35rem; width: min(300px, calc(100vw - 32px)); padding: 0.6rem; border: 1px solid var(--color-border, #4b4f57); border-radius: 8px; background: var(--color-surface, #1d1f22); box-shadow: var(--shadow-md, 0 4px 16px rgba(0, 0, 0, 0.55)); }
  .assistance-menu :global(.assistance-action) { justify-content: flex-start; width: 100%; }
</style>
