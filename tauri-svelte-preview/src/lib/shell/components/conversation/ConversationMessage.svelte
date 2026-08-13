<script lang="ts">
  import { parseSafeMarkdown, type SafeInlinePart } from '$lib/shell/conversation/conversationMessageSafety.ts';
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
</script>

<article
  data-testid={`${role}-message-item`}
  data-item-id={itemId}
  class:user={role === 'user'}
  class:assistant={role === 'assistant'}
  class:streaming={!completed}
>
  <div class="turn-body" data-testid="conversation-message-body">
    {#each blocks as block, blockIndex}
      {#if block.kind === 'code'}
        <CodeBlock value={block.value} info={block.language} />
      {:else if block.kind === 'heading'}
        <h3 class={`heading level-${Math.min(block.level, 4)}`} data-testid="conversation-markdown-heading">{#each block.parts as part}{@render inline(part)}{/each}</h3>
      {:else if block.kind === 'quote'}
        <blockquote data-testid="conversation-markdown-quote">{#each block.parts as part}{@render inline(part)}{/each}</blockquote>
      {:else if block.kind === 'list'}
        {#if block.ordered}<ol data-testid="conversation-markdown-list">{#each block.items as entry}<li class:task-row={entry.task}>{#if entry.task}<input data-testid="conversation-task-checkbox" type="checkbox" checked={entry.checked} disabled />{/if}{#each entry.parts as part}{@render inline(part)}{/each}</li>{/each}</ol>
        {:else}<ul data-testid="conversation-markdown-list">{#each block.items as entry}<li class:task-row={entry.task}>{#if entry.task}<input data-testid="conversation-task-checkbox" type="checkbox" checked={entry.checked} disabled />{/if}{#each entry.parts as part}{@render inline(part)}{/each}</li>{/each}</ul>{/if}
      {:else if block.kind === 'table'}
        <div class="table-scroll" data-testid="conversation-markdown-table"><table><thead><tr>{#each block.headers as cell}<th>{#each cell as part}{@render inline(part)}{/each}</th>{/each}</tr></thead><tbody>{#each block.rows as row}<tr>{#each row as cell}<td>{#each cell as part}{@render inline(part)}{/each}</td>{/each}</tr>{/each}</tbody></table></div>
      {:else}
        <p data-testid={blockIndex === 0 ? 'conversation-markdown-paragraph' : undefined}>{#each block.parts as part}{@render inline(part)}{/each}</p>
      {/if}
    {/each}
  </div>
</article>

{#snippet inline(part: SafeInlinePart)}
  {#if part.kind === 'strong'}<strong>{part.value}</strong>
  {:else if part.kind === 'emphasis'}<em>{part.value}</em>
  {:else if part.kind === 'code'}<code class="inline-code">{part.value}</code>
  {:else if part.kind === 'link'}<a href={part.href} target="_blank" rel="noreferrer">{part.value}</a>
  {:else if part.kind === 'file-link'}<button class="file-link" data-testid="conversation-file-link" type="button" onclick={() => onFileLink?.(part.path)}>{part.value}</button>
  {:else}{part.value}{/if}
{/snippet}

<style>
  /* Nothing announces who is speaking. The shape does: a reply runs the full
     768px measure with no box around it, a prompt is a narrow bubble on the
     right. That asymmetry is the only role marking in the transcript. */
  article{max-width:768px;user-select:text;-webkit-user-select:text}
  .turn-body{font-size:14px;line-height:1.62}

  /* Paragraphs and headings: one rhythm, 12px between blocks, and a heading
     that has more room above it than below so it reads as owning what follows. */
  .turn-body p{margin:0 0 12px;white-space:pre-wrap}
  .turn-body p:last-child{margin-bottom:0}
  .heading{margin:20px 0 8px;font-weight:640;line-height:1.35;letter-spacing:-.01em}
  .heading:first-child{margin-top:0}
  .heading.level-1{font-size:18px}
  .heading.level-2{font-size:16px}
  .heading.level-3{font-size:15px}
  .heading.level-4{font-size:14px;color:var(--color-text-2)}

  .turn-body blockquote{margin:0 0 12px;padding:6px 14px;border-left:2px solid color-mix(in srgb,var(--color-accent) 45%,var(--color-border));border-radius:0 6px 6px 0;background:color-mix(in srgb,var(--color-surface) 40%,transparent);color:var(--color-text-2)}
  .turn-body ul,.turn-body ol{margin:0 0 12px;padding-left:22px}
  .turn-body li{margin-bottom:5px;padding-left:3px}
  .turn-body li:last-child{margin-bottom:0}
  .turn-body li::marker{color:var(--color-text-3)}
  .turn-body li.task-row{list-style:none;margin-left:-18px;padding-left:0}
  .turn-body a{color:var(--color-accent);text-decoration:underline;text-underline-offset:2px}
  .file-link{border:0;background:transparent;color:var(--color-accent);padding:0;text-decoration:underline;text-underline-offset:2px;font:inherit;cursor:pointer}
  .inline-code{padding:1.5px 5px;border-radius:6px;border:1px solid color-mix(in srgb,var(--color-border) 55%,transparent);background:color-mix(in srgb,var(--color-surface) 55%,var(--color-bg));font:13px/1.35 ui-monospace,SFMono-Regular,Menlo,monospace}

  .user{max-width:80%;margin-left:auto;padding:12px;border-radius:16px;background:color-mix(in srgb,var(--color-surface) 82%,var(--color-accent) 18%)}

  .table-scroll{overflow:auto;margin-bottom:12px;border:1px solid color-mix(in srgb,var(--color-border) 70%,transparent);border-radius:8px}
  .table-scroll table{border-collapse:collapse;min-width:100%;font-size:13px}
  .table-scroll th,.table-scroll td{padding:7px 10px;border-bottom:1px solid color-mix(in srgb,var(--color-border) 55%,transparent);text-align:left;vertical-align:top}
  .table-scroll tr:last-child td{border-bottom:0}
  .table-scroll th{background:color-mix(in srgb,var(--color-surface) 65%,transparent);font-weight:620;white-space:nowrap}
  .turn-body input{margin-right:7px;accent-color:var(--color-accent)}

</style>
