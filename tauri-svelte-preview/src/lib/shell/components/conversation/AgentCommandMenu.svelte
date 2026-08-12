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
    {#each visible as command, index (command.id)}<button class:active={index === activeIndex} data-testid="conversation-command-row" role="option" aria-selected={index === activeIndex} aria-label={`${slashName(command)} ${command.description ?? command.label}`} type="button" onclick={() => onSelect?.(command)}><span class="command-main"><strong>{slashName(command)}</strong><small>{command.label}</small>{#if command.description}<span class="command-description">{command.description}</span>{/if}</span><span class:assembly={command.source === 'assembly'} class="command-source">{sourceLabel(command)}</span></button>{/each}
  {:else}
    <p class="command-empty" data-testid="conversation-command-empty">{emptyText || 'No commands'}</p>
  {/if}
</div>

<style>.command-menu{position:absolute;left:0;right:0;bottom:112px;z-index:4;display:flex;flex-direction:column;max-height:min(58vh,520px);overflow:auto;padding:6px;border:1px solid var(--color-border);border-radius:10px;background:var(--color-surface);box-shadow:var(--shadow-lg)}.command-menu button{display:flex;justify-content:space-between;align-items:flex-start;gap:12px;border:0;border-radius:6px;background:transparent;color:inherit;padding:8px 9px;text-align:left}.command-menu button:hover,.command-menu button.active{background:color-mix(in srgb,var(--color-accent) 10%,transparent)}.command-main{display:grid;gap:2px;min-width:0}.command-main strong{font:13px ui-monospace,SFMono-Regular,Menlo,monospace}.command-main small,.command-description,.command-source{overflow:hidden;color:var(--color-text-2);font-size:13px;text-overflow:ellipsis;white-space:nowrap}.command-description{font-size:12px}.command-source{flex:0 0 auto}.command-source.assembly{color:var(--color-accent)}.command-empty{margin:0;padding:9px;color:var(--color-text-2);font-size:13px}</style>
