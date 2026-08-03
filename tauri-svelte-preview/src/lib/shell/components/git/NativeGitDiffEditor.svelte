<script lang="ts">
  import type * as Monaco from 'monaco-editor/esm/vs/editor/editor.api';
  import { onDestroy, onMount } from 'svelte';

  let {
    root,
    relativePath,
    originalContent,
    modifiedContent
  }: {
    root: string;
    relativePath: string;
    originalContent: string;
    modifiedContent: string;
  } = $props();

  let host: HTMLDivElement;
  let monacoApi: typeof Monaco | null = null;
  let editor: Monaco.editor.IStandaloneDiffEditor | null = null;
  let originalModel: Monaco.editor.ITextModel | null = null;
  let modifiedModel: Monaco.editor.ITextModel | null = null;
  let renderedKey = '';
  let disposed = false;

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
      await import('@codingame/monaco-vscode-standalone-languages');
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
        fontFamily: 'ui-monospace, SFMono-Regular, Menlo, Monaco, monospace',
        fontSize: 12,
        lineHeight: 19
      });
      renderModels();
    })().catch((error) => {
      console.error('Could not open the native Git diff editor.', error);
    });
  });

  onDestroy(() => {
    disposed = true;
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
