<script lang="ts">
  import ChevronRight from '@lucide/svelte/icons/chevron-right';
  import FileText from '@lucide/svelte/icons/file-text';
  import { Button } from '$lib/components/ui/button/index.js';
  import { userMessageOverflowsFold } from '$lib/shell/conversation/conversationTimeline.ts';
  import { parseSafeMarkdown, type SafeInlinePart, type SafeMarkdownBlock } from '$lib/shell/conversation/conversationMessageSafety.ts';
  import {
    cancelTrackedAnimationFrame,
    requestTrackedAnimationFrame
  } from '$lib/shell/resourceDiagnostics.svelte.ts';
  import CodeBlock from './CodeBlock.svelte';

  interface Props {
    text: string;
    role: 'user' | 'assistant';
    itemId?: string;
    completed?: boolean;
    onFileLink?(path: string): void;
  }

  let { text, role, itemId = 'message', completed = true, onFileLink }: Props = $props();
  const blocks = $derived(parseSafeMarkdown(text));

  /* A pasted log or a long brief is worth keeping, but not worth scrolling
     past every time the conversation is reopened. A sent message shows its
     first ten lines and offers the rest; a reply is never folded, because the
     reply is the thing being read. */
  let bodyHost = $state<HTMLElement | null>(null);
  let expanded = $state(false);
  let folded = $state(false);

  $effect(() => {
    const host = bodyHost;
    if (role !== 'user' || !host) return;
    void blocks;
    let frame: number | null = null;
    const measure = (): void => {
      const nextFolded = userMessageOverflowsFold(
        host.scrollHeight,
        Number.parseFloat(getComputedStyle(host).lineHeight)
      );
      if (folded !== nextFolded) folded = nextFolded;
    };
    frame = requestTrackedAnimationFrame(() => {
      frame = null;
      measure();
    });
    return () => {
      if (frame !== null) cancelTrackedAnimationFrame(frame);
    };
  });
</script>

<article
  data-testid={`${role}-message-item`}
  data-item-id={itemId}
  class:user={role === 'user'}
  class:assistant={role === 'assistant'}
  class:streaming={!completed}
>
  <div
    class="turn-body"
    class:folded={folded && !expanded}
    data-testid="conversation-message-body"
    bind:this={bodyHost}
  >
    {#each blocks as block, blockIndex}{@render node(block, blockIndex === 0)}{/each}
  </div>
  {#if folded}
    <div class="fold-row">
      <Button
        variant="ghost"
        size="sm"
        class="text-[13px] text-muted-foreground"
        data-testid="conversation-message-fold"
        aria-expanded={expanded}
        onclick={() => (expanded = !expanded)}
      >
        <span class="fold-chevron" class:open={expanded} aria-hidden="true"><ChevronRight size={13} strokeWidth={1.8} /></span>
        {expanded ? 'Show less' : 'Show more'}
      </Button>
    </div>
  {/if}
</article>

<!-- Blocks nest: a list item can hold a list, a quote can hold anything. This
     renders one block and calls itself for whatever is underneath it. -->
{#snippet node(block: SafeMarkdownBlock, first = false)}
  {#if block.kind === 'code'}
    <CodeBlock value={block.value} info={block.language} />
  {:else if block.kind === 'heading'}
    <h3 class={`heading level-${Math.min(block.level, 4)}`} data-testid="conversation-markdown-heading">{#each block.parts as part}{@render inline(part)}{/each}</h3>
  {:else if block.kind === 'quote'}
    <blockquote data-testid="conversation-markdown-quote">{#each block.blocks as inner}{@render node(inner)}{/each}</blockquote>
  {:else if block.kind === 'rule'}
    <hr data-testid="conversation-markdown-rule" />
  {:else if block.kind === 'list'}
    {#if block.ordered}<ol data-testid="conversation-markdown-list">{#each block.items as entry}{@render row(entry)}{/each}</ol>
    {:else}<ul data-testid="conversation-markdown-list">{#each block.items as entry}{@render row(entry)}{/each}</ul>{/if}
  {:else if block.kind === 'table'}
    <div class="table-scroll" data-testid="conversation-markdown-table"><table><thead><tr>{#each block.headers as cell}<th>{#each cell as part}{@render inline(part)}{/each}</th>{/each}</tr></thead><tbody>{#each block.rows as tableRow}<tr>{#each tableRow as cell}<td>{#each cell as part}{@render inline(part)}{/each}</td>{/each}</tr>{/each}</tbody></table></div>
  {:else}
    <p data-testid={first ? 'conversation-markdown-paragraph' : undefined}>{#each block.parts as part}{@render inline(part)}{/each}</p>
  {/if}
{/snippet}

{#snippet row(entry: { task: boolean; checked: boolean; parts: SafeInlinePart[]; blocks: SafeMarkdownBlock[] })}
  <li class:task-row={entry.task}>{#if entry.task}<input data-testid="conversation-task-checkbox" type="checkbox" checked={entry.checked} disabled />{/if}{#each entry.parts as part}{@render inline(part)}{/each}{#each entry.blocks as inner}{@render node(inner)}{/each}</li>
{/snippet}

{#snippet inline(part: SafeInlinePart)}
  {#if part.kind === 'strong'}<strong>{#each part.parts as inner}{@render inline(inner)}{/each}</strong>
  {:else if part.kind === 'emphasis'}<em>{#each part.parts as inner}{@render inline(inner)}{/each}</em>
  {:else if part.kind === 'strike'}<del>{#each part.parts as inner}{@render inline(inner)}{/each}</del>
  {:else if part.kind === 'code'}<code class="inline-code">{part.value}</code>
  {:else if part.kind === 'link'}<a href={part.href} target="_blank" rel="noreferrer">{#each part.parts as inner}{@render inline(inner)}{/each}</a>
  {:else if part.kind === 'file-link'}<button class="file-link" data-testid="conversation-file-link" type="button" title={part.path} onclick={() => onFileLink?.(part.path)}><FileText size={13} strokeWidth={1.8} aria-hidden="true" />{#each part.parts as inner}{@render inline(inner)}{/each}</button>
  {:else}{part.value}{/if}
{/snippet}

<style>
  /* Nothing announces who is speaking. The shape does: a reply runs the full
     768px measure with no box around it, a prompt is a narrow bubble on the
     right. That asymmetry is the only role marking in the transcript. */
  article{max-width:768px;user-select:text;-webkit-user-select:text}
  .turn-body{font-size:14px;line-height:1.62}

  /* The transcript's scale, inside a message: 12px between blocks, 8px between
     the rows of a list, and 16px above a heading so it reads as owning the
     section under it rather than floating between two. The last block gives up
     its trailing margin so the gap to the next turn is the one the list sets,
     not that gap plus this one. */
  .turn-body p{margin:0 0 12px;white-space:pre-wrap}
  .turn-body p:last-child,
  .turn-body ul:last-child,
  .turn-body ol:last-child,
  .turn-body blockquote:last-child,
  .turn-body .table-scroll:last-child{margin-bottom:0}
  .heading{margin:16px 0 8px;font-weight:640;line-height:1.35;letter-spacing:-.01em}
  .heading:first-child{margin-top:0}
  .heading.level-1{font-size:18px}
  .heading.level-2{font-size:16px}
  .heading.level-3{font-size:15px}
  .heading.level-4{font-size:14px;color:var(--color-text-2)}

  /* A quote is marked by the rule down its side and the quieter text, and by
     nothing else. It carried a fill and rounded corners as well, which made
     every quoted line look like a warning box. */
  .turn-body blockquote{margin:0 0 12px;padding:2px 0 2px 14px;border-left:2px solid color-mix(in srgb,var(--color-accent) 42%,var(--color-border));color:var(--color-text-2)}
  .turn-body ul,.turn-body ol{margin:0 0 12px;padding-left:22px}
  .turn-body li{margin-bottom:8px;padding-left:3px}
  .turn-body li:last-child{margin-bottom:0}
  .turn-body li::marker{color:var(--color-text-3)}
  .turn-body li.task-row{list-style:none;margin-left:-18px;padding-left:0}
  .turn-body a{color:var(--color-accent);text-decoration:underline;text-underline-offset:2px}
  /* A document link is the file's name with the page mark in front of it, so a
     file being pointed at is distinguishable at a glance from a link off to
     the web. The mark keeps out of the underline; the name carries it. */
  .file-link{display:inline-flex;align-items:baseline;gap:4px;border:0;background:transparent;color:var(--color-accent);padding:0;text-decoration:underline;text-underline-offset:2px;font:inherit;cursor:pointer}
  .file-link :global(svg){align-self:center;flex:none}
  /* Tinted, not outlined. The border made a two-word span read as a button,
     which was loudest exactly where inline code is most common: table cells. */
  .inline-code{padding:1.5px 5px;border-radius:5px;background:color-mix(in srgb,var(--color-surface) 78%,var(--color-bg));font:13px/1.35 var(--font-mono)}

  /* Ten lines at the body's own size and leading. The text under the cut is
     still there and still selectable; the fade says there is more without
     drawing a second edge inside the bubble. */
  .turn-body.folded{max-height:calc(10 * 1.62em);overflow:hidden;-webkit-mask-image:linear-gradient(to bottom,#000 calc(100% - 26px),transparent);mask-image:linear-gradient(to bottom,#000 calc(100% - 26px),transparent)}
  .fold-row{display:flex;margin-top:6px}
  .fold-chevron{display:grid;place-items:center;color:var(--color-text-3)}
  .fold-chevron.open{transform:rotate(90deg)}
  @media (prefers-reduced-motion:no-preference){.fold-chevron{transition:transform .14s ease}}

  .user{width:fit-content;max-width:80%;margin-left:auto;padding:10px 14px;border-radius:16px;background:var(--color-elevated)}

  .table-scroll{overflow:auto;margin-bottom:12px;border:1px solid color-mix(in srgb,var(--color-border) 70%,transparent);border-radius:8px}
  .table-scroll table{border-collapse:collapse;min-width:100%;font-size:13px}
  .table-scroll th,.table-scroll td{padding:7px 10px;border-bottom:1px solid color-mix(in srgb,var(--color-border) 55%,transparent);text-align:left;vertical-align:top}
  .table-scroll tr:last-child td{border-bottom:0}
  .table-scroll th{background:color-mix(in srgb,var(--color-surface) 65%,transparent);font-weight:620;white-space:nowrap}
  /* A task row draws its own box. The platform control paints a disabled
     checkbox the same grey whether or not it is ticked, so a finished task
     looked no different from an open one — which is the only thing the box is
     there to say. It stays a real checkbox for anything reading the page. */
  .turn-body input[type=checkbox]{appearance:none;-webkit-appearance:none;display:inline-grid;place-content:center;flex:none;width:13px;height:13px;margin:0 8px 0 0;border:1.5px solid color-mix(in srgb,var(--color-border) 95%,var(--color-text));border-radius:4px;background:transparent;vertical-align:-2px}
  .turn-body input[type=checkbox]:checked{border-color:var(--color-accent);background:var(--color-accent)}
  .turn-body input[type=checkbox]:checked::after{content:'';width:6px;height:3px;border:1.6px solid var(--color-bg);border-top:0;border-right:0;transform:translateY(-1px) rotate(-45deg)}
  /* A sub-list is one step in, and gives up the gap under its parent row so the
     two read as one group rather than two lists. */
  .turn-body li > :global(ul),.turn-body li > :global(ol){margin:8px 0 0}
  .turn-body hr{margin:16px 0;border:0;border-top:1px solid color-mix(in srgb,var(--color-border) 70%,transparent)}
  .turn-body del{color:var(--color-text-3);text-decoration-thickness:1px}

</style>
