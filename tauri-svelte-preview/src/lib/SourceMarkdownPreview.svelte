<script lang="ts">
  import { onMount, untrack } from 'svelte';
  import type { Editor } from '@milkdown/kit/core';
  import type { Node as ProseNode } from '@milkdown/kit/prose/model';
  import { fenceLanguage, highlightCode } from './shell/components/conversation/codeHighlight';

  /**
   * The Markdown file as a document you can edit in place (Milkdown). The
   * editor is downloaded the first time a Markdown file is shown this way and is
   * destroyed when this view goes away. Only real edits are reported: loading or
   * reloading the file never marks it modified, and undoing back to the loaded
   * document hands back the file's own text rather than a re-serialized copy.
   */
  type Props = {
    /** The file this view was opened on. Edits are always reported for it,
     *  even if another file has become active by the time they arrive. */
    path: string;
    content: string;
    fileName: string;
    readOnly?: boolean;
    scrollTop?: number;
    onScroll?: (scrollTop: number) => void;
    onChange?: (path: string, markdown: string) => void;
    onSave?: () => void;
  };

  /** The part of a parsed Markdown (mdast) node the image fix reads. */
  type MarkdownTreeNode = { type: string; title?: string | null; children?: MarkdownTreeNode[] };

  let { path, content, fileName, readOnly = false, scrollTop = 0, onScroll, onChange, onSave }: Props = $props();
  const ownedPath = untrack(() => path);
  let host: HTMLElement;
  let loadError = $state<string | null>(null);
  /** The text the editor last loaded or reported, so its own echo is not loaded back in. */
  let shown = '';
  let loadDocument: ((markdown: string) => void) | null = null;
  /** Reports an edit still waiting on the typing pause. */
  let flush = (): void => {};

  /** The panel calls this before it changes, closes, saves or rereads files,
   *  so an edit made in the last 150 ms is in the store first. */
  export function flushEdit(): void {
    flush();
  }

  $effect(() => {
    if (content !== shown) loadDocument?.(content);
  });

  onMount(() => {
    let editor: Editor | null = null;
    let destroyed = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const onKeyDown = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey) || event.key.toLowerCase() !== 's') return;
      event.preventDefault();
      flush();
      onSave?.();
    };
    host.addEventListener('keydown', onKeyDown);

    void (async () => {
      try {
        const [core, { commonmark }, { gfm, remarkGFMPlugin }, { history }, utils, { Plugin }, { Decoration, DecorationSet }] =
          await Promise.all([
            import('@milkdown/kit/core'),
            import('@milkdown/kit/preset/commonmark'),
            import('@milkdown/kit/preset/gfm'),
            import('@milkdown/kit/plugin/history'),
            import('@milkdown/kit/utils'),
            import('@milkdown/kit/prose/state'),
            import('@milkdown/kit/prose/view'),
            import('@milkdown/kit/prose/view/style/prosemirror.css')
          ]);
        if (destroyed) return;

        let applying = false;
        let baseSource = content;
        let baseDoc: ProseNode | null = null;

        const report = () => {
          timer = undefined;
          if (!editor) return;
          const doc = editor.ctx.get(core.editorViewCtx).state.doc;
          shown = baseDoc && doc.eq(baseDoc) ? baseSource : editor.ctx.get(core.serializerCtx)(doc);
          onChange?.(ownedPath, shown);
        };
        flush = () => {
          if (timer === undefined) return;
          clearTimeout(timer);
          report();
        };

        // Serializing the whole file costs about 6 ms for a 290-line file and
        // 30 ms for a 2,000-line one, too much for every key, so an edit is
        // reported once typing pauses. The panel flushes it sooner (flushEdit)
        // whenever it is about to change, close, save or reread files.
        const reportEdits = utils.$prose(() => new Plugin({
          view: () => ({
            update(view, previous) {
              if (applying || view.state.doc.eq(previous.doc)) return;
              clearTimeout(timer);
              timer = setTimeout(report, 150);
            }
          })
        }));

        // Milkdown 7.22.2 hands an image with no title to the schema as a null
        // title, which the schema rejects, so the image vanishes from the
        // document and from the next save. An absent title becomes ''.
        const imageTitles = utils.$remark('imageTitles', () => () => (tree: MarkdownTreeNode) => {
          const visit = (node: MarkdownTreeNode): void => {
            if (node.type === 'image' && node.title == null) node.title = '';
            node.children?.forEach(visit);
          };
          visit(tree);
        });

        // Code blocks are coloured by the same scanner the old viewer used.
        const decorate = (doc: ProseNode) => {
          const decorations: ReturnType<typeof Decoration.inline>[] = [];
          doc.descendants((node, position) => {
            if (node.type.name !== 'code_block') return true;
            let offset = position + 1;
            highlightCode(node.textContent, fenceLanguage(String(node.attrs.language ?? ''))).forEach((line, index) => {
              if (index > 0) offset += 1;
              for (const span of line) {
                if (span.className !== 'plain' && span.value) {
                  decorations.push(Decoration.inline(offset, offset + span.value.length, { class: span.className }));
                }
                offset += span.value.length;
              }
            });
            return false;
          });
          return DecorationSet.create(doc, decorations);
        };
        const highlight = utils.$prose(() => new Plugin({
          state: {
            init: (_config, state) => decorate(state.doc),
            apply: (transaction, previous: ReturnType<typeof decorate>) =>
              transaction.docChanged ? decorate(transaction.doc) : previous
          },
          props: {
            decorations(state) {
              return this.getState(state);
            }
          }
        }));

        const created = await core.Editor.make()
          .config((ctx) => {
            ctx.set(core.rootCtx, host);
            ctx.set(core.defaultValueCtx, baseSource);
            ctx.update(core.editorViewOptionsCtx, (options) => ({
              ...options,
              editable: () => !readOnly,
              // An image shows as its text and never fetches: a file's remote
              // image must not phone home when the file is opened.
              nodeViews: {
                image: (node) => {
                  const dom = document.createElement('span');
                  dom.className = 'markdown-image';
                  dom.textContent = String(node.attrs.alt || node.attrs.src || 'image');
                  dom.title = String(node.attrs.src ?? '');
                  return { dom };
                }
              }
            }));
            // Write lists and tables the way people usually type them, so saving
            // an edit does not restyle the rest of the file.
            ctx.update(core.remarkStringifyOptionsCtx, (options) => ({ ...options, bullet: '-' as const }));
            ctx.set(remarkGFMPlugin.options.key, { tablePipeAlign: false });
          })
          .use(imageTitles)
          .use(commonmark)
          .use(gfm)
          .use(history)
          .use(reportEdits)
          .use(highlight)
          .create();
        if (destroyed) {
          await created.destroy();
          return;
        }
        editor = created;
        shown = baseSource;
        baseDoc = created.ctx.get(core.editorViewCtx).state.doc;
        loadDocument = (markdown) => {
          clearTimeout(timer);
          timer = undefined;
          applying = true;
          created.action(utils.replaceAll(markdown, true));
          applying = false;
          baseSource = markdown;
          baseDoc = created.ctx.get(core.editorViewCtx).state.doc;
          shown = markdown;
        };
        if (content !== shown) loadDocument(content);
        host.scrollTop = scrollTop;
      } catch (error) {
        if (!destroyed) loadError = `Could not open the Markdown editor: ${error instanceof Error ? error.message : String(error)}`;
      }
    })();

    return () => {
      destroyed = true;
      flush();
      loadDocument = null;
      flush = () => {};
      host.removeEventListener('keydown', onKeyDown);
      void editor?.destroy();
    };
  });
</script>

<section class="markdown-document" aria-label={`Preview ${fileName}`}>
  {#if loadError}<p class="markdown-document-error">{loadError}</p>{/if}
  <div
    class="markdown-document-scroll selectable"
    bind:this={host}
    onscroll={() => onScroll?.(host.scrollTop)}
  ></div>
</section>

<style>
  .markdown-document {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    color: var(--color-text);
    background: var(--color-bg);
  }

  .markdown-document-error {
    margin: 0;
    padding: var(--space-2) var(--space-4);
    color: var(--color-bad);
    font-size: var(--text-quiet);
  }

  .markdown-document-scroll {
    flex: 1 1 auto;
    min-height: 0;
    overflow: auto;
    scrollbar-width: thin;
    scrollbar-color: var(--scrollbar-thumb) transparent;
  }

  .markdown-document-scroll :global(.ProseMirror) {
    max-width: 860px;
    min-height: 100%;
    margin: 0 auto;
    padding: var(--space-5) var(--space-6) var(--space-6);
    font-size: var(--text-body);
    line-height: 1.6;
    outline: none;
    caret-color: var(--color-accent);
  }

  .markdown-document-scroll :global(.ProseMirror > :first-child) { margin-top: 0; }

  .markdown-document-scroll :global(h1),
  .markdown-document-scroll :global(h2),
  .markdown-document-scroll :global(h3),
  .markdown-document-scroll :global(h4),
  .markdown-document-scroll :global(h5),
  .markdown-document-scroll :global(h6) {
    margin: var(--space-5) 0 var(--space-3);
    color: var(--color-text);
    font-weight: var(--text-heading-weight);
    line-height: 1.3;
  }

  .markdown-document-scroll :global(h1) {
    padding-bottom: var(--space-2);
    border-bottom: 1px solid var(--color-border);
    font-size: 24px;
  }

  .markdown-document-scroll :global(h2) { font-size: 19px; }
  .markdown-document-scroll :global(h3) { font-size: var(--text-heading); }
  .markdown-document-scroll :global(h4),
  .markdown-document-scroll :global(h5),
  .markdown-document-scroll :global(h6) { font-size: var(--text-body); }

  .markdown-document-scroll :global(p),
  .markdown-document-scroll :global(ul),
  .markdown-document-scroll :global(ol),
  .markdown-document-scroll :global(blockquote),
  .markdown-document-scroll :global(pre),
  .markdown-document-scroll :global(table) {
    margin: 0 0 var(--space-3);
  }

  .markdown-document-scroll :global(ul),
  .markdown-document-scroll :global(ol) { padding-left: var(--space-5); }
  .markdown-document-scroll :global(li > p) { margin: 0; }
  .markdown-document-scroll :global(li + li) { margin-top: var(--space-1); }

  .markdown-document-scroll :global(li[data-item-type='task']) {
    position: relative;
    list-style: none;
  }

  .markdown-document-scroll :global(li[data-item-type='task']::before) {
    position: absolute;
    top: 0.3em;
    left: -20px;
    width: 12px;
    height: 12px;
    border: 1px solid var(--color-field-border);
    border-radius: var(--radius-xs);
    content: '';
  }

  .markdown-document-scroll :global(li[data-item-type='task'][data-checked='true']::before) {
    border-color: var(--color-accent);
    background: var(--color-accent);
  }

  .markdown-document-scroll :global(li[data-item-type='task'][data-checked='true']) {
    color: var(--color-text-2);
  }

  .markdown-document-scroll :global(a) {
    color: var(--color-accent);
    text-decoration: underline;
    text-decoration-color: color-mix(in srgb, var(--color-accent) 40%, transparent);
  }

  .markdown-document-scroll :global(code) {
    padding: 1px 4px;
    border-radius: var(--radius-xs);
    background: var(--color-elevated);
    font-family: var(--font-mono);
    font-size: 0.9em;
  }

  .markdown-document-scroll :global(pre) {
    overflow: auto;
    padding: var(--space-3);
    border: 1px solid color-mix(in srgb, var(--color-border) 70%, transparent);
    border-radius: 10px;
    background: color-mix(in srgb, var(--color-surface) 34%, var(--color-bg));
    font: 13px/1.6 var(--font-mono);
    white-space: pre;
  }

  .markdown-document-scroll :global(pre code) {
    padding: 0;
    background: transparent;
    font: inherit;
  }

  .markdown-document-scroll :global(.keyword) { color: var(--color-accent); }
  .markdown-document-scroll :global(.string) { color: var(--color-good); }
  .markdown-document-scroll :global(.comment) { color: var(--color-text-3); font-style: italic; }
  .markdown-document-scroll :global(.number) { color: var(--color-attention); }
  .markdown-document-scroll :global(.type) { color: var(--color-live); }

  .markdown-document-scroll :global(blockquote) {
    padding: var(--space-1) var(--space-3);
    border-left: 3px solid color-mix(in srgb, var(--color-accent) 45%, transparent);
    color: var(--color-text-2);
  }

  .markdown-document-scroll :global(hr) {
    margin: var(--space-5) 0;
    border: 0;
    border-top: 1px solid var(--color-border);
  }

  .markdown-document-scroll :global(table) {
    display: block;
    max-width: 100%;
    overflow-x: auto;
    border-collapse: collapse;
  }

  .markdown-document-scroll :global(th),
  .markdown-document-scroll :global(td) {
    padding: var(--space-1) var(--space-3);
    border: 1px solid var(--color-border);
    text-align: left;
    vertical-align: top;
  }

  .markdown-document-scroll :global(th > p),
  .markdown-document-scroll :global(td > p) { margin: 0; }

  .markdown-document-scroll :global(th) {
    background: var(--color-surface);
    font-weight: 600;
  }

  .markdown-document-scroll :global(.markdown-image) {
    color: var(--color-text-2);
    text-decoration: underline dotted;
  }

  .markdown-document-scroll :global(.ProseMirror-selectednode) {
    outline: 2px solid var(--color-focus);
  }
</style>
