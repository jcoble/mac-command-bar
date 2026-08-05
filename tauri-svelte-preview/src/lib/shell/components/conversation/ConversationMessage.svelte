<script lang="ts">
  import { parseSafeMarkdown, type SafeInlinePart } from '$lib/shell/conversation/conversationMessageSafety.ts';

  interface Props {
    text: string;
    role: 'user' | 'assistant';
    label: string;
    itemId?: string;
    completed?: boolean;
    onFileLink?(path: string): void;
  }

  let { text, role, label, itemId = 'message', completed = true, onFileLink }: Props = $props();
  const blocks = $derived(parseSafeMarkdown(text));

  async function copyCode(value: string): Promise<void> {
    if (typeof navigator !== 'undefined' && navigator.clipboard) await navigator.clipboard.writeText(value);
  }
</script>

<article
  data-testid={`${role}-message-item`}
  data-item-id={itemId}
  class:user={role === 'user'}
  class:assistant={role === 'assistant'}
  class:streaming={!completed}
>
  <div class="turn-label" data-testid="conversation-message-label">{label}{#if !completed}<span class="streaming-mark"> · streaming</span>{/if}</div>
  <div class="turn-body" data-testid="conversation-message-body">
    {#each blocks as block, blockIndex}
      {#if block.kind === 'code'}
        <div class="code-wrap" data-testid="conversation-code-block">
          <div class="code-meta"><span>{block.language || 'code'}{#if !block.complete} · streaming{/if}</span><button data-testid="copy-conversation-code" type="button" onclick={() => void copyCode(block.value)}>Copy</button></div>
          <pre><code>{block.value}</code></pre>
        </div>
      {:else if block.kind === 'heading'}
        <h3 data-testid="conversation-markdown-heading">{#each block.parts as part}{@render inline(part)}{/each}</h3>
      {:else if block.kind === 'quote'}
        <blockquote data-testid="conversation-markdown-quote">{#each block.parts as part}{@render inline(part)}{/each}</blockquote>
      {:else if block.kind === 'list'}
        {#if block.ordered}<ol data-testid="conversation-markdown-list">{#each block.items as entry}<li>{#if entry.task}<input data-testid="conversation-task-checkbox" type="checkbox" checked={entry.checked} disabled />{/if}{#each entry.parts as part}{@render inline(part)}{/each}</li>{/each}</ol>
        {:else}<ul data-testid="conversation-markdown-list">{#each block.items as entry}<li>{#if entry.task}<input data-testid="conversation-task-checkbox" type="checkbox" checked={entry.checked} disabled />{/if}{#each entry.parts as part}{@render inline(part)}{/each}</li>{/each}</ul>{/if}
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
  article{max-width:760px;user-select:text;-webkit-user-select:text}.turn-label{margin-bottom:7px;color:var(--color-text-2);font-size:12px;font-weight:650;letter-spacing:.04em;text-transform:uppercase}.streaming-mark{font-weight:500;letter-spacing:0;text-transform:none}.turn-body{font-size:14px;line-height:1.65}.turn-body p{margin:0 0 12px;white-space:pre-wrap}.turn-body p:last-child{margin-bottom:0}.turn-body h3{margin:0 0 12px;font-size:16px;line-height:1.35}.turn-body blockquote{margin:0 0 12px;padding:5px 12px;border-left:2px solid var(--color-border);color:var(--color-text-2)}.turn-body ul,.turn-body ol{margin:0 0 12px;padding-left:24px}.turn-body li{padding-left:3px}.turn-body a{color:var(--color-accent);text-decoration:underline;text-underline-offset:2px}.file-link{border:0;background:transparent;color:var(--color-accent);padding:0;text-decoration:underline;text-underline-offset:2px;font:inherit;cursor:pointer}.inline-code{padding:1px 4px;border-radius:4px;background:color-mix(in srgb,var(--color-surface) 70%,var(--color-bg));font:12px/1.35 ui-monospace,SFMono-Regular,Menlo,monospace}pre{overflow:auto;margin:0;padding:14px 16px;border:1px solid color-mix(in srgb,var(--color-border) 78%,transparent);border-radius:0 0 10px 10px;background:color-mix(in srgb,var(--color-bg) 76%,black 24%);font:12px/1.55 ui-monospace,SFMono-Regular,Menlo,monospace}pre code{user-select:text;-webkit-user-select:text}.code-wrap{margin:12px 0}.code-meta{display:flex;justify-content:space-between;align-items:center;padding:7px 10px;border:1px solid color-mix(in srgb,var(--color-border) 78%,transparent);border-bottom:0;border-radius:10px 10px 0 0;color:var(--color-text-2);font-size:12px}.code-meta button{border:0;background:transparent;color:inherit;padding:3px 6px}.user{align-self:flex-end;max-width:min(680px,88%);padding:12px 15px;border-radius:16px;background:color-mix(in srgb,var(--color-surface) 88%,var(--color-accent) 12%)}.user .turn-label{color:color-mix(in srgb,var(--color-accent) 70%,var(--color-text) 30%)}.table-scroll{overflow:auto;margin-bottom:12px}.table-scroll table{border-collapse:collapse;min-width:100%}.table-scroll th,.table-scroll td{padding:7px 9px;border:1px solid var(--color-border);text-align:left;vertical-align:top}.table-scroll th{background:color-mix(in srgb,var(--color-surface) 80%,transparent)}.turn-body input{margin-right:7px}.code-wrap + p{margin-top:12px}
</style>
