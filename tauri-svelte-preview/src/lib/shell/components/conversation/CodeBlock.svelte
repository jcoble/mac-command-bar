<script lang="ts">
  import Check from '@lucide/svelte/icons/check';
  import Copy from '@lucide/svelte/icons/copy';
  import WrapText from '@lucide/svelte/icons/wrap-text';
  import {
    highlightCode,
    monacoLanguageForFence,
    plainHighlightedLines,
    type HighlightedLine
  } from './codeHighlight.ts';

  interface Props {
    value: string;
    /** The fence's info string, e.g. `ts` or `bash`. */
    info?: string;
  }

  let { value, info = '' }: Props = $props();

  const language = $derived(monacoLanguageForFence(info));
  /** The fence's info string names the block; it is one token or nothing. */
  const label = $derived(info.trim() || 'code');
  /** Long lines scroll sideways until someone asks for them to wrap. */
  let wrapped = $state(false);
  /** Uncolored until the editor answers, so the code is readable immediately. */
  const plainLines = $derived(plainHighlightedLines(value));
  let coloredLines = $state<HighlightedLine[] | null>(null);
  const lines = $derived(coloredLines ?? plainLines);
  let copied = $state(false);
  let host = $state<HTMLDivElement | null>(null);
  let onScreen = $state(false);
  let copyTimer: ReturnType<typeof setTimeout> | null = null;

  // The editor loads for the first code block a person can actually see. A
  // transcript scrolled past a hundred prose messages never pays for it.
  $effect(() => {
    if (!host || onScreen) return;
    if (typeof IntersectionObserver === 'undefined') {
      onScreen = true;
      return;
    }
    const observer = new IntersectionObserver((entries) => {
      if (entries.some((entry) => entry.isIntersecting)) {
        onScreen = true;
        observer.disconnect();
      }
    }, { rootMargin: '400px' });
    observer.observe(host);
    return () => observer.disconnect();
  });

  $effect(() => {
    const source = value;
    const languageId = language;
    if (!onScreen) return;
    let cancelled = false;
    coloredLines = null;
    void highlightCode(source, languageId).then((next) => {
      if (!cancelled) coloredLines = next;
    });
    return () => {
      cancelled = true;
    };
  });

  $effect(() => () => {
    if (copyTimer) clearTimeout(copyTimer);
  });

  async function copyCode(): Promise<void> {
    if (typeof navigator === 'undefined' || !navigator.clipboard) return;
    await navigator.clipboard.writeText(value);
    copied = true;
    if (copyTimer) clearTimeout(copyTimer);
    copyTimer = setTimeout(() => (copied = false), 1400);
  }
</script>

<div class="code-wrap" data-testid="conversation-code-block" bind:this={host}>
  <div class="code-meta">
    <span class="code-language">{label}</span>
    <button
      class="meta-action"
      data-testid="wrap-conversation-code"
      type="button"
      aria-pressed={wrapped}
      onclick={() => (wrapped = !wrapped)}
    >
      <WrapText size={13} strokeWidth={1.8} />{wrapped ? 'No wrap' : 'Wrap'}
    </button>
    <button class="meta-action" data-testid="copy-conversation-code" type="button" onclick={() => void copyCode()}>
      {#if copied}<Check size={13} strokeWidth={2.2} />Copied{:else}<Copy size={13} strokeWidth={1.8} />Copy{/if}
    </button>
  </div>
  <pre class:wrapped data-testid="conversation-code-body"><code>{#each lines as line, index}{#if index > 0}{'\n'}{/if}{#each line as span}<span class={span.className}>{span.value}</span>{/each}{/each}</code></pre>
</div>

<style>
  .code-wrap{margin:12px 0}
  /* The header names the block and carries its two utilities; the name takes
     the room, the actions sit at the end. */
  .code-meta{display:flex;align-items:center;gap:4px;min-height:28px;padding:2px 6px 2px 10px;border:1px solid color-mix(in srgb,var(--color-border) 70%,transparent);border-bottom:0;border-radius:10px 10px 0 0;background:color-mix(in srgb,var(--color-surface) 62%,var(--color-bg));color:var(--color-text-3);font-size:12px}
  .code-language{flex:1;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;font-family:ui-monospace,SFMono-Regular,Menlo,monospace;letter-spacing:.03em}
  .meta-action{display:inline-flex;align-items:center;gap:4px;min-height:24px;border:0;border-radius:6px;background:transparent;color:inherit;padding:2px 8px;font:inherit;cursor:pointer}
  .meta-action:hover{background:color-mix(in srgb,var(--color-hover) 70%,transparent);color:var(--color-text)}
  .meta-action:focus-visible{outline:2px solid var(--color-focus-solid);outline-offset:1px}
  pre{overflow:auto;margin:0;padding:12px;border:1px solid color-mix(in srgb,var(--color-border) 70%,transparent);border-radius:0 0 10px 10px;background:color-mix(in srgb,var(--color-surface) 34%,var(--color-bg));font:12px/1.6 ui-monospace,SFMono-Regular,Menlo,monospace}
  pre.wrapped{white-space:pre-wrap;overflow-wrap:anywhere}
  code{user-select:text;-webkit-user-select:text}
  .keyword{color:var(--color-accent)}
  .string{color:var(--color-good)}
  .comment{color:var(--color-text-3);font-style:italic}
  .number{color:var(--color-attention)}
  .type{color:var(--color-live)}
  .plain{color:inherit}
  @media (prefers-reduced-motion:no-preference){
    .meta-action{transition:background .14s ease,color .14s ease}
  }
</style>
