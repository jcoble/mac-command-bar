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
  import { codeMirrorTheme } from '$lib/shell/editor/codeMirrorTheme';

  let {
    relativePath,
    originalContent,
    modifiedContent,
    onOpenLine
  }: {
    root: string;
    relativePath: string;
    originalContent: string;
    modifiedContent: string;
    onOpenLine?: (line: number) => void;
  } = $props();

  let host: HTMLDivElement;
  let mergeView: MergeView | null = null;
  let mounted = false;
  let renderGeneration = 0;

  function readOnlyExtensions(language: Extension, openLine = false): Extension[] {
    return [
      basicSetup,
      codeMirrorTheme,
      language,
      EditorState.readOnly.of(true),
      EditorView.editable.of(false),
      ...(openLine
        ? [EditorView.domEventHandlers({
            dblclick: (event, view) => {
              if (!onOpenLine) return false;
              const position = view.posAtCoords({ x: event.clientX, y: event.clientY });
              if (position === null) return false;
              onOpenLine(view.state.doc.lineAt(position).number);
              return true;
            }
          })]
        : [])
    ];
  }

  async function render(): Promise<void> {
    if (!mounted) return;
    const generation = ++renderGeneration;
    const language = await loadCodeMirrorLanguage(codeMirrorLanguageForPath(relativePath));
    if (!mounted || generation !== renderGeneration) return;
    if (mergeView) {
      mergeView.destroy();
      addMergeView(-1);
      setMergeDocBytes(0);
    }
    mergeView = new MergeView({
      parent: host,
      a: { doc: originalContent, extensions: readOnlyExtensions(language) },
      b: { doc: modifiedContent, extensions: readOnlyExtensions(language, true) },
      orientation: 'a-b',
      highlightChanges: true,
      gutter: true,
      diffConfig: { scanLimit: 1000, timeout: 500 }
    });
    addMergeView(1);
    setMergeDocBytes(textBytes(originalContent) + textBytes(modifiedContent));
  }

  $effect(() => {
    relativePath;
    originalContent;
    modifiedContent;
    void render();
  });

  onMount(() => {
    mounted = true;
    void render();
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

<div class="codemirror-diff" bind:this={host} aria-label="File changes"></div>

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
</style>
