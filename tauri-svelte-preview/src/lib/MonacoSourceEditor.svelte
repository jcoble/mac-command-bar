<script lang="ts">
  import 'monaco-editor/min/vs/editor/editor.main.css';
  import EditorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker';
  import { onDestroy, onMount } from 'svelte';
  import type * as Monaco from 'monaco-editor/esm/vs/editor/editor.api';
  import type { SourcePreview } from './sourceData';

  type Props = {
    preview: SourcePreview;
    loading?: boolean;
  };

  let { preview, loading = false }: Props = $props();

  let host = $state<HTMLDivElement | null>(null);
  let editor = $state<Monaco.editor.IStandaloneCodeEditor | null>(null);
  let monacoApi: typeof Monaco | null = null;
  let currentPath = '';
  let isReady = $state(false);
  const ownedModels = new Set<Monaco.editor.ITextModel>();

  function languageId(language: SourcePreview['language']) {
    return language === 'csharp' ? 'csharp' : language;
  }

  function installWorker() {
    const target = self as unknown as {
      MonacoEnvironment?: { getWorker: (_moduleId: string, _label: string) => Worker };
    };

    target.MonacoEnvironment = {
      getWorker: () => new EditorWorker()
    };
  }

  function configureMonaco(monaco: typeof Monaco) {
    monaco.editor.defineTheme('mcb-source-dark', {
      base: 'vs-dark',
      inherit: true,
      rules: [
        { token: 'keyword', foreground: 'ff6d91', fontStyle: 'bold' },
        { token: 'keyword.control', foreground: 'ff6d91', fontStyle: 'bold' },
        { token: 'type', foreground: '5fd8f2' },
        { token: 'type.identifier', foreground: '5fd8f2' },
        { token: 'identifier', foreground: 'e7ecea' },
        { token: 'namespace', foreground: 'b67af0' },
        { token: 'string', foreground: 'a7d977' },
        { token: 'number', foreground: 'e5bb70' },
        { token: 'comment', foreground: '7c8582', fontStyle: 'italic' },
        { token: 'operator', foreground: 'f08d7e' },
        { token: 'delimiter', foreground: '77817e' }
      ],
      colors: {
        'editor.background': '#111313',
        'editor.foreground': '#e7ecea',
        'editorLineNumber.foreground': '#52605c',
        'editorLineNumber.activeForeground': '#9fb1ac',
        'editorCursor.foreground': '#5ce2cf',
        'editor.selectionBackground': '#2a696133',
        'editor.inactiveSelectionBackground': '#2a69611f',
        'editor.lineHighlightBackground': '#ffffff08',
        'editorGutter.background': '#111313',
        'editorWidget.background': '#1d2221',
        'editorWidget.border': '#ffffff1a',
        'editorSuggestWidget.background': '#1d2221',
        'scrollbarSlider.background': '#ffffff2b',
        'scrollbarSlider.hoverBackground': '#ffffff42',
        'scrollbarSlider.activeBackground': '#ffffff5a'
      }
    });
  }

  function applyPreview() {
    if (!monacoApi || !editor || !preview) return;

    const language = languageId(preview.language);
    const uri = monacoApi.Uri.file(preview.path);
    let model = monacoApi.editor.getModel(uri);

    if (!model) {
      model = monacoApi.editor.createModel(preview.content, language, uri);
      ownedModels.add(model);
    } else {
      if (model.getValue() !== preview.content) {
        model.setValue(preview.content);
      }
      if (model.getLanguageId() !== language) {
        monacoApi.editor.setModelLanguage(model, language);
      }
    }

    if (editor.getModel() !== model) {
      editor.setModel(model);
    }

    if (currentPath !== preview.path) {
      currentPath = preview.path;
      editor.setScrollTop(0);
      editor.setScrollLeft(0);
      editor.setPosition({ lineNumber: 1, column: 1 });
    }
  }

  onMount(async () => {
    if (!host) return;

    installWorker();

    const [monaco] = await Promise.all([
      import('monaco-editor/esm/vs/editor/editor.api'),
      import('monaco-editor/esm/vs/basic-languages/csharp/csharp.contribution'),
      import('monaco-editor/esm/vs/basic-languages/swift/swift.contribution'),
      import('monaco-editor/esm/vs/basic-languages/rust/rust.contribution')
    ]);

    monacoApi = monaco;
    configureMonaco(monaco);

    editor = monaco.editor.create(host, {
      automaticLayout: true,
      bracketPairColorization: { enabled: true },
      contextmenu: true,
      cursorBlinking: 'solid',
      cursorStyle: 'line-thin',
      cursorWidth: 1,
      domReadOnly: true,
      folding: true,
      fontFamily: '"SF Mono", ui-monospace, Menlo, Monaco, Consolas, monospace',
      fontLigatures: false,
      fontSize: 13,
      glyphMargin: false,
      hideCursorInOverviewRuler: true,
      language: languageId(preview.language),
      letterSpacing: 0,
      lineDecorationsWidth: 14,
      lineHeight: 21,
      lineNumbers: 'on',
      lineNumbersMinChars: 3,
      minimap: { enabled: false },
      model: null,
      occurrencesHighlight: 'off',
      overviewRulerBorder: false,
      overviewRulerLanes: 0,
      padding: { top: 16, bottom: 20 },
      readOnly: true,
      renderLineHighlight: 'gutter',
      renderWhitespace: 'selection',
      scrollBeyondLastLine: false,
      scrollbar: {
        alwaysConsumeMouseWheel: false,
        horizontalScrollbarSize: 12,
        verticalScrollbarSize: 12
      },
      smoothScrolling: true,
      stickyScroll: { enabled: false },
      tabSize: 4,
      theme: 'mcb-source-dark',
      wordWrap: 'off'
    });

    isReady = true;
    applyPreview();
  });

  $effect(() => {
    if (isReady) {
      applyPreview();
    }
  });

  onDestroy(() => {
    editor?.dispose();
    for (const model of ownedModels) {
      model.dispose();
    }
    ownedModels.clear();
  });
</script>

<div class="source-editor" data-testid="monaco-source-editor">
  <div bind:this={host} class="monaco-host"></div>

  {#if loading || !isReady}
    <div class="skeleton-code" aria-label="Loading source preview">
      {#each Array.from({ length: 13 }) as _, index}
        <span style={`--line-width: ${index % 4 === 0 ? 48 : index % 3 === 0 ? 66 : 86}%`}></span>
      {/each}
    </div>
  {/if}
</div>

<style>
  .source-editor {
    position: relative;
    height: calc(100% - 42px);
    min-height: 0;
    overflow: hidden;
    background: #111313;
  }

  .monaco-host {
    width: 100%;
    height: 100%;
  }

  .monaco-host :global(.monaco-editor),
  .monaco-host :global(.overflow-guard) {
    border-bottom-left-radius: 13px;
    border-bottom-right-radius: 13px;
  }

  .monaco-host :global(.monaco-editor .margin) {
    background: #111313;
  }

  .skeleton-code {
    position: absolute;
    inset: 0;
    display: grid;
    align-content: start;
    gap: 12px;
    padding: 22px;
    background: #111313;
  }

  .skeleton-code span {
    display: block;
    width: var(--line-width);
    height: 13px;
    border-radius: 999px;
    background: linear-gradient(90deg, #242929, #333a38, #242929);
    background-size: 180% 100%;
    animation: shimmer 1.2s ease-in-out infinite;
  }

  @keyframes shimmer {
    from {
      background-position: 100% 0;
    }
    to {
      background-position: -80% 0;
    }
  }
</style>
