<script lang="ts">
  /**
   * BrowserMiniComposer.svelte — describe the change, attach the picture.
   *
   * This is a handover, not a send. Attach burns the marks into the captured
   * page, saves that as an attachment on the session, puts a short note about
   * the page and the picked element into the draft, and moves to the session so
   * the person can finish the sentence and send it themselves. Nothing leaves
   * this panel on its own.
   */
  import Paperclip from '@lucide/svelte/icons/paperclip';

  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';

  interface Props {
    description: string;
    /** How many marks are on the page, so the button can say what it takes. */
    markCount: number;
    /** True while the picture is being captured and saved. */
    busy: boolean;
    /** Nothing to attach: no session, no page, or no marks and no picked element. */
    disabled: boolean;
    error: string;
    onDescriptionChange(value: string): void;
    onAttach(): void;
  }

  let { description, markCount, busy, disabled, error, onDescriptionChange, onAttach }: Props =
    $props();

  const summary = $derived(
    markCount === 0 ? 'Nothing marked yet' : markCount === 1 ? '1 mark' : `${markCount} marks`
  );
</script>

<div class="mini-composer" data-testid="browser-mini-composer">
  <form
    class="row"
    onsubmit={(event) => {
      event.preventDefault();
      if (!disabled && !busy) onAttach();
    }}
  >
    <Input
      aria-label="Describe the change"
      placeholder="Describe the change…"
      value={description}
      data-testid="browser-mini-composer-input"
      oninput={(event) => onDescriptionChange(event.currentTarget.value)}
    />
    <Button
      size="sm"
      disabled={disabled || busy}
      data-testid="browser-attach"
      onclick={onAttach}
    >
      <Paperclip aria-hidden="true" />
      {busy ? 'Attaching…' : 'Attach'}
    </Button>
  </form>
  <p class="summary" data-testid="browser-mark-summary">{summary}</p>
  {#if error}
    <p class="failure" role="alert">{error}</p>
  {/if}
</div>

<style>
  .mini-composer {
    display: flex;
    flex-direction: column;
    gap: 4px;
    border-top: 1px solid var(--color-border);
    padding: 8px 12px;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .summary {
    margin: 0;
    font-size: 12px;
    color: var(--color-text-2);
  }

  .failure {
    margin: 0;
    font-size: 12px;
    color: var(--color-bad);
  }
</style>
