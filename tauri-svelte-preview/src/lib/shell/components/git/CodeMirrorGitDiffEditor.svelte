<script lang="ts">
  import { EditorState, type Extension } from '@codemirror/state';
  import { MergeView } from '@codemirror/merge';
  import { EditorView } from '@codemirror/view';
  import { basicSetup } from 'codemirror';
  import { onDestroy, onMount } from 'svelte';

  import { addMergeView, setMergeDocBytes, textBytes } from '$lib/shell/resourceDiagnostics.svelte';
  import {
    codeMirrorLanguageForPath,
    loadCodeMirrorLanguage
  } from '$lib/shell/editor/codeMirrorLanguage';
  import { settings } from '$lib/settingsStore.svelte';
  import { loadCodeMirrorTheme } from '$lib/shell/editor/codeMirrorTheme';

  let {
    relativePath,
    originalContent,
    modifiedContent,
    onOpenLine,
    onReviewLine
  }: {
    root: string;
    relativePath: string;
    originalContent: string;
    modifiedContent: string;
    onOpenLine?: (line: number) => void;
    onReviewLine?: (side: 'LEFT' | 'RIGHT', line: number) => void;
  } = $props();

  let host: HTMLDivElement;
  let mergeView: MergeView | null = null;
  let mounted = false;
  let renderGeneration = 0;

  function readOnlyExtensions(language: Extension, theme: Extension, side: 'LEFT' | 'RIGHT'): Extension[] {
    return [
      basicSetup,
      theme,
      language,
      EditorState.readOnly.of(true),
      EditorView.editable.of(false),
      ...(side === 'RIGHT' && onOpenLine
        ? [EditorView.domEventHandlers({
            dblclick: (event, view) => {
              const position = view.posAtCoords({ x: event.clientX, y: event.clientY });
              if (position === null) return false;
              const line = view.state.doc.lineAt(position).number;
              onOpenLine(line);
              return true;
            }
          })]
        : [])
    ];
  }

  function reviewLineClick(event: MouseEvent): void {
    if (!onReviewLine || !mergeView || !(event.target instanceof Element)) return;
    const side = mergeView.a.dom.contains(event.target) ? 'LEFT' : mergeView.b.dom.contains(event.target) ? 'RIGHT' : null;
    if (!side) return;
    const lineElement = event.target.closest('.cm-line');
    if (!lineElement) return;
    const view = side === 'LEFT' ? mergeView.a : mergeView.b;
    const position = view.posAtDOM(lineElement, 0);
    onReviewLine(side, view.state.doc.lineAt(position).number);
  }

  async function render(): Promise<void> {
    if (!mounted) return;
    const generation = ++renderGeneration;
    const [language, theme] = await Promise.all([
      loadCodeMirrorLanguage(codeMirrorLanguageForPath(relativePath)),
      loadCodeMirrorTheme(settings.appearance.themeId, {
        fontFamily: settings.editor.fontFamily,
        fontSize: settings.editor.fontSize,
        lineHeight: settings.editor.lineHeight,
        fontLigatures: settings.editor.fontLigatures
      })
    ]);
    if (!mounted || generation !== renderGeneration) return;
    if (mergeView) {
      mergeView.destroy();
      addMergeView(-1);
      setMergeDocBytes(0);
    }
    mergeView = new MergeView({
      parent: host,
      a: { doc: originalContent, extensions: readOnlyExtensions(language, theme, 'LEFT') },
      b: { doc: modifiedContent, extensions: readOnlyExtensions(language, theme, 'RIGHT') },
      orientation: 'a-b',
      highlightChanges: true,
      gutter: true,
      collapseUnchanged: { margin: 3, minSize: 4 },
      diffConfig: { scanLimit: 1000, timeout: 500 }
    });
    addMergeView(1);
    setMergeDocBytes(textBytes(originalContent) + textBytes(modifiedContent));
  }

  $effect(() => {
    relativePath;
    originalContent;
    modifiedContent;
    settings.appearance.themeId;
    settings.editor.fontFamily;
    settings.editor.fontSize;
    settings.editor.lineHeight;
    settings.editor.fontLigatures;
    void render();
  });

  onMount(() => {
    mounted = true;
    host.addEventListener('click', reviewLineClick, true);
    void render();
    return () => host.removeEventListener('click', reviewLineClick, true);
  });

  onDestroy(() => {
    mounted = false;
    renderGeneration += 1;
    if (mergeView) {
      mergeView.destroy();
      addMergeView(-1);
    }
    mergeView = null;
    setMergeDocBytes(0);
  });
</script>

<div class="codemirror-diff" bind:this={host} aria-label="File changes" data-selectable="true"></div>

<style>
  .codemirror-diff {
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    background: var(--color-surface, #17191e);
  }

  :global(.codemirror-diff .cm-mergeView) {
    height: 100%;
    overflow: auto;
  }

  :global(.codemirror-diff .cm-mergeViewEditors) {
    min-height: 100%;
  }

  :global(.codemirror-diff .cm-editor) {
    min-width: 0;
  }

  :global(.codemirror-diff .cm-merge-a .cm-changedLine) {
    background: rgba(255, 85, 85, 0.18) !important;
    box-shadow: inset 3px 0 rgba(255, 85, 85, 0.72);
  }

  :global(.codemirror-diff .cm-merge-b .cm-changedLine) {
    background: rgba(80, 250, 123, 0.16) !important;
    box-shadow: inset 3px 0 rgba(80, 250, 123, 0.66);
  }

  :global(.codemirror-diff .cm-merge-a .cm-changedText) {
    background: rgba(255, 85, 85, 0.28) !important;
  }

  :global(.codemirror-diff .cm-merge-b .cm-changedText) {
    background: rgba(80, 250, 123, 0.25) !important;
  }
</style>
