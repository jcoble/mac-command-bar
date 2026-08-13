<script lang="ts">
  import type { ConversationCommand } from '$lib/shell/conversation/conversationCommandCatalog.ts';
  import { slashName } from '$lib/shell/conversation/conversationCommandCatalog.ts';

  interface Props {
    commands: readonly ConversationCommand[];
    activeIndex: number;
    emptyText?: string;
    onSelect?(command: ConversationCommand): void;
  }
  let { commands, activeIndex, emptyText = '', onSelect }: Props = $props();
  const visible = $derived(commands);

  function sourceLabel(command: ConversationCommand): string {
    if (command.source === 'assembly') return 'Built-in';
    const description = command.description?.toLowerCase() ?? '';
    if (description.includes('(bundled)')) return 'Built-in';
    if (description.includes('(user)')) return 'Custom';
    return command.source === 'skill' ? 'Skill' : 'Provider';
  }
</script>

<div class="command-menu" data-testid="conversation-command-menu" role="listbox" aria-label="Conversation commands" tabindex="-1">
  {#if visible.length}
    {#each visible as command, index (command.id)}
      <button
        class:active={index === activeIndex}
        id={`conversation-command-${command.id}`}
        data-testid="conversation-command-row"
        role="option"
        aria-selected={index === activeIndex}
        aria-label={`${slashName(command)} ${command.description ?? command.label}`}
        type="button"
        onclick={() => onSelect?.(command)}
      >
        <span class="command-glyph" aria-hidden="true">/</span>
        <span class="command-main">
          <strong>{slashName(command)}</strong>
          <span class="command-label">{command.label}</span>
          {#if command.description}<span class="command-description">{command.description}</span>{/if}
        </span>
        <span class:assembly={command.source === 'assembly'} class="command-source">{sourceLabel(command)}</span>
      </button>
    {/each}
  {:else}
    <p class="command-empty" data-testid="conversation-command-empty">{emptyText || 'No commands'}</p>
  {/if}
</div>

<style>
  .command-menu { position: absolute; left: 0; right: 0; bottom: 106px; z-index: 4; display: flex; flex-direction: column; max-height: min(58vh, 520px); overflow: auto; padding: 7px; border: 1px solid color-mix(in srgb, var(--color-border) 84%, transparent); border-radius: 16px; background: color-mix(in srgb, var(--color-surface) 97%, transparent); box-shadow: var(--shadow-lg); }
  .command-menu button { display: flex; align-items: flex-start; gap: 8px; width: 100%; min-height: 42px; border: 0; border-radius: 10px; background: transparent; color: inherit; padding: 8px 9px; text-align: left; cursor: pointer; }
  .command-menu button:hover, .command-menu button.active { background: color-mix(in srgb, var(--color-accent) 11%, transparent); }
  .command-menu button:focus-visible { outline: 2px solid var(--color-focus-solid); outline-offset: -1px; }
  .command-glyph { display: grid; place-items: center; flex: none; width: 22px; height: 22px; border: 1px solid color-mix(in srgb, var(--color-border) 72%, transparent); border-radius: 7px; color: var(--color-accent); font: 600 13px ui-monospace, SFMono-Regular, Menlo, monospace; }
  .command-main { display: grid; min-width: 0; flex: 1; gap: 1px; }
  .command-main strong { color: var(--color-text); font: 600 13px ui-monospace, SFMono-Regular, Menlo, monospace; }
  .command-label, .command-description, .command-source { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .command-label { color: var(--color-text); font-size: 13px; }
  .command-description { color: var(--color-text-2); font-size: 12px; }
  .command-source { flex: 0 0 auto; max-width: 100px; padding-top: 2px; color: var(--color-text-2); font-size: 12px; }
  .command-source.assembly { color: var(--color-accent); }
  .command-empty { margin: 0; padding: 11px 10px; color: var(--color-text-2); font-size: 13px; }
</style>
