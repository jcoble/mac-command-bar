<script lang="ts">
  import type { ConversationChildAgent } from '$lib/shell/conversation/conversationTypes.ts';

  interface Props {
    children: readonly ConversationChildAgent[];
    selectedChildId: string | null;
    onSelect?(childId: string | null): void;
  }
  let { children, selectedChildId, onSelect }: Props = $props();
  const rootNodes = $derived(children.filter((child) => !children.some((candidate) => candidate.childId === child.parentId)));
  function descendants(parentId: string): ConversationChildAgent[] {
    return children.filter((child) => child.parentId === parentId);
  }
</script>

{#if children.length}
  <nav class="agent-tree" data-testid="conversation-agent-tree" aria-label="Session agents">
    <button class:active={!selectedChildId} data-testid="conversation-parent-agent" type="button" onclick={() => onSelect?.(null)}><span class="agent-dot parent"></span><span>Parent</span><small>editable</small></button>
    <div class="tree-branch" data-testid="conversation-agent-children">
      {#each rootNodes as child (child.childId)}
        {@render node(child, 0)}
      {/each}
    </div>
  </nav>
{/if}

{#snippet node(child: ConversationChildAgent, depth: number)}
  <div class="tree-node" style={`--depth:${depth}`}>
    <button class:active={selectedChildId === child.childId} data-testid="conversation-child-agent" type="button" onclick={() => onSelect?.(child.childId)} title={child.childId}>
      <span class="tree-line" aria-hidden="true">{depth ? '└' : '├'}</span><span class:working={child.state === 'active'} class:failed={child.state === 'failed'} class="agent-dot"></span><span class="node-label">{child.label}</span><small>{child.state} · read-only</small>
    </button>
    {#each descendants(child.childId) as nested (nested.childId)}{@render node(nested, depth + 1)}{/each}
  </div>
{/snippet}

<style>.agent-tree{display:flex;gap:8px;overflow:auto;padding:7px 18px;border-bottom:1px solid color-mix(in srgb,var(--color-border) 55%,transparent)}.agent-tree button{display:flex;align-items:center;gap:6px;min-height:28px;padding:5px 8px;border:1px solid transparent;border-radius:7px;background:transparent;color:var(--color-text-2);white-space:nowrap}.agent-tree button.active{border-color:color-mix(in srgb,var(--color-accent) 35%,transparent);background:color-mix(in srgb,var(--color-accent) 11%,transparent);color:var(--color-text)}.tree-branch{display:flex;gap:4px}.tree-node{display:grid;gap:3px;margin-left:calc(var(--depth) * 8px)}.agent-tree small{font-size:12px;opacity:.7}.tree-line{opacity:.35}.agent-dot{width:7px;height:7px;border-radius:50%;background:#7f8794}.agent-dot.parent{background:var(--color-accent)}.agent-dot.working{background:#52c7a8;box-shadow:0 0 0 3px color-mix(in srgb,#52c7a8 16%,transparent)}.agent-dot.failed{background:#e16f7a}.node-label{max-width:180px;overflow:hidden;text-overflow:ellipsis}</style>
