<script lang="ts">
  import type { ConversationCommand } from '$lib/shell/conversation/conversationCommandCatalog.ts';
  import { slashName } from '$lib/shell/conversation/conversationCommandCatalog.ts';

  interface Props {
    commands: readonly ConversationCommand[];
    query: string;
    onSelect?(command: ConversationCommand): void;
  }
  let { commands, query, onSelect }: Props = $props();
  let activeIndex = $state(0);
  const visible = $derived(commands.slice(0, 12));

  $effect(() => {
    query;
    activeIndex = Math.min(activeIndex, Math.max(0, visible.length - 1));
  });

  function navigate(event: KeyboardEvent): void {
    if (!visible.length) return;
    if (event.key === 'ArrowDown') { event.preventDefault(); activeIndex = (activeIndex + 1) % visible.length; }
    else if (event.key === 'ArrowUp') { event.preventDefault(); activeIndex = (activeIndex - 1 + visible.length) % visible.length; }
    else if (event.key === 'Enter') { event.preventDefault(); onSelect?.(visible[activeIndex]); }
  }
</script>

{#if visible.length}
  <div class="command-menu" data-testid="conversation-command-menu" role="listbox" aria-label="Conversation commands" onkeydown={navigate} tabindex="-1">
    {#each visible as command, index (command.id)}<button class:active={index === activeIndex} data-testid="conversation-command-row" role="option" aria-selected={index === activeIndex} type="button" onclick={() => onSelect?.(command)}><span class="command-main"><strong>{slashName(command)}</strong><small>{command.label}</small></span><span class:assembly={command.source === 'assembly'} class="command-source">{command.source}</span></button>{/each}
  </div>
{/if}

<style>.command-menu{position:absolute;left:0;right:0;bottom:112px;z-index:4;display:flex;flex-direction:column;padding:6px;border:1px solid var(--color-border);border-radius:10px;background:var(--color-surface);box-shadow:0 12px 40px rgba(0,0,0,.28)}.command-menu button{display:flex;justify-content:space-between;align-items:center;gap:12px;border:0;border-radius:6px;background:transparent;color:inherit;padding:8px 9px;text-align:left}.command-menu button:hover,.command-menu button.active{background:color-mix(in srgb,var(--color-accent) 10%,transparent)}.command-main{display:grid;gap:2px;min-width:0}.command-main strong{font:13px ui-monospace,SFMono-Regular,Menlo,monospace}.command-main small,.command-source{overflow:hidden;color:var(--color-text-2);font-size:12px;text-overflow:ellipsis;white-space:nowrap}.command-source.assembly{color:var(--color-accent)}</style>
