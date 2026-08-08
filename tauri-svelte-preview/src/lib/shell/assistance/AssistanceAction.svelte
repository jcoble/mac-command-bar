<script lang="ts">
  import type { AssistanceRecipeId, AssistanceSurface } from './assistanceTypes.ts';

  interface Props {
    recipeId: AssistanceRecipeId;
    surface: AssistanceSurface;
    label: string;
    disabled?: boolean;
    onRequest?: (recipeId: AssistanceRecipeId, surface: AssistanceSurface) => void;
  }

  let {
    recipeId,
    surface,
    label,
    disabled = false,
    onRequest
  }: Props = $props();
</script>

<button
  type="button"
  class="assistance-action"
  data-testid={`assistance-action-${recipeId}`}
  data-assistance-surface={surface}
  {disabled}
  onclick={() => onRequest?.(recipeId, surface)}
>
  <span aria-hidden="true">✦</span>
  {label}
</button>

<style>
  .assistance-action {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    min-height: 32px;
    border: 0;
    border-radius: 6px;
    padding: 0.35rem 0.55rem;
    background: var(--color-elevated);
    color: var(--color-text);
    cursor: pointer;
    font: inherit;
  }

  .assistance-action:hover:not(:disabled) { background: var(--color-hover); }
  .assistance-action:focus-visible { outline: none; box-shadow: var(--focus-ring); }

  .assistance-action:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }
</style>
