<script lang="ts">
  import type * as Monaco from 'monaco-editor/esm/vs/editor/editor.api';
  import { onDestroy, onMount } from 'svelte';

  let {
    root,
    relativePath,
    originalContent,
    modifiedContent,
    onOpenLine
  }: {
    root: string;
    relativePath: string;
    originalContent: string;
    modifiedContent: string;
    /**
     * Double-clicking a line asks for the real file at that line. Left out,
     * the diff behaves exactly as it did: a double click selects a word.
     */
    onOpenLine?: (line: number) => void;
  } = $props();

  let host: HTMLDivElement;
  let monacoApi: typeof Monaco | null = null;
  let editor: Monaco.editor.IStandaloneDiffEditor | null = null;
  let originalModel: Monaco.editor.ITextModel | null = null;
  let modifiedModel: Monaco.editor.ITextModel | null = null;
  let renderedKey = '';
  let disposed = false;
  let openLineListener: Monaco.IDisposable | null = null;

  function absolutePath(): string {
    return `${root.replace(/\/+$/, '')}/${relativePath.replace(/^\/+/, '')}`;
  }

  function languageForPath(path: string): string {
    const extension = path.split('.').pop()?.toLowerCase() ?? '';
    const languages: Record<string, string> = {
      cs: 'csharp',
      csx: 'csharp',
      css: 'css',
      html: 'html',
      htm: 'html',
      js: 'javascript',
      jsx: 'javascript',
      json: 'json',
      md: 'markdown',
      rs: 'rust',
      svelte: 'svelte',
      ts: 'typescript',
      tsx: 'typescript',
      xml: 'xml',
      yaml: 'yaml',
      yml: 'yaml'
    };
    return languages[extension] ?? 'plaintext';
  }

  function clearModels(): void {
    editor?.setModel(null);
    originalModel?.dispose();
    modifiedModel?.dispose();
    originalModel = null;
    modifiedModel = null;
  }

  function renderModels(): void {
    if (!monacoApi || !editor || disposed) return;
    const key = JSON.stringify([root, relativePath, originalContent, modifiedContent]);
    if (key === renderedKey) return;
    renderedKey = key;

    clearModels();
    const fileUri = monacoApi.Uri.file(absolutePath());
    const language = languageForPath(relativePath);
    originalModel = monacoApi.editor.createModel(
      originalContent,
      language,
      fileUri.with({ query: 'mcb-git=original' })
    );
    modifiedModel = monacoApi.editor.createModel(
      modifiedContent,
      language,
      fileUri.with({ query: 'mcb-git=modified' })
    );
    editor.setModel({ original: originalModel, modified: modifiedModel });
  }

  $effect(() => {
    root;
    relativePath;
    originalContent;
    modifiedContent;
    renderModels();
  });

  onMount(() => {
    void (async () => {
      const { prepareNativeCsharpEditorServices } = await import(
        '$lib/shell/editor/csharpLanguageClient'
      );
      await prepareNativeCsharpEditorServices(root);
      // The standalone Monarch languages exist only for the degraded run with
      // no real services: registered beside them, Monarch tokenizers race the
      // TextMate grammars and crash asking for the standalone theme service.
      const { monacoVscodeApiIsInitialized } = await import('$lib/shell/editor/monacoWorkers');
      if (!monacoVscodeApiIsInitialized()) {
        await import('@codingame/monaco-vscode-standalone-languages');
      }
      const loaded = await import('monaco-editor/esm/vs/editor/editor.api');
      if (disposed) return;
      monacoApi = loaded;
      editor = loaded.editor.createDiffEditor(host, {
        automaticLayout: true,
        readOnly: true,
        originalEditable: false,
        renderSideBySide: true,
        renderOverviewRuler: true,
        ignoreTrimWhitespace: false,
        minimap: { enabled: false },
        scrollBeyondLastLine: false,
        fontFamily: 'var(--font-mono)',
        fontSize: 12,
        lineHeight: 19
      });
      // Double click, not single: a single click in a diff is how a person
      // selects and reads, and jumping away on every click would make the diff
      // unusable. The line taken is the one in the file as it is now.
      openLineListener = editor.getModifiedEditor().onMouseUp((event) => {
        if (!onOpenLine || event.event.detail < 2) return;
        const line = event.target.position?.lineNumber;
        if (typeof line === 'number' && line > 0) onOpenLine(line);
      });
      renderModels();
    })().catch((error) => {
      console.error('Could not open the native Git diff editor.', error);
    });
  });

  onDestroy(() => {
    disposed = true;
    openLineListener?.dispose();
    openLineListener = null;
    clearModels();
    editor?.dispose();
    editor = null;
  });
</script>

<div class="native-diff" bind:this={host} aria-label="File changes"></div>

<style>
  .native-diff {
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    background: var(--color-surface, #17191e);
  }
</style>
