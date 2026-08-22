<script lang="ts">
  import {
    autocompletion,
    closeBrackets,
    closeBracketsKeymap,
    completionKeymap,
    type CompletionContext
  } from '@codemirror/autocomplete';
  import { defaultKeymap, history, historyKeymap, undoDepth } from '@codemirror/commands';
  import {
    bracketMatching,
    defaultHighlightStyle,
    foldGutter,
    foldKeymap,
    indentOnInput,
    syntaxHighlighting
  } from '@codemirror/language';
  import { lintGutter, lintKeymap, setDiagnostics, type Diagnostic } from '@codemirror/lint';
  import { highlightSelectionMatches, searchKeymap } from '@codemirror/search';
  import { EditorState, Compartment, Transaction, type Extension } from '@codemirror/state';
  import {
    crosshairCursor,
    drawSelection,
    dropCursor,
    EditorView,
    highlightActiveLine,
    highlightActiveLineGutter,
    highlightSpecialChars,
    hoverTooltip,
    keymap,
    lineNumbers,
    rectangularSelection
  } from '@codemirror/view';
  import { onDestroy, onMount } from 'svelte';

  import {
    addCodeMirrorEditorView,
    requestTrackedAnimationFrame,
    setCodeMirrorEditorStateCount,
    setCodeMirrorDocBytes,
    setCodeMirrorUndoDepth,
    textBytes
  } from '$lib/shell/resourceDiagnostics.svelte';
  import { loadCodeMirrorLanguage } from '$lib/shell/editor/codeMirrorLanguage';
  import { codeMirrorTheme } from '$lib/shell/editor/codeMirrorTheme';
  import type {
    SourceIntelligenceCallbacks,
    SourceLookupRequest
  } from '$lib/shell/editor/sourceIntelligence';
  import {
    extractSourceSymbols,
    type SourceDiagnostic,
    type SourcePreview,
    type SourceSymbol
  } from '$lib/sourceData';

  type StoredViewState = {
    anchor: number;
    head: number;
    scrollTop: number;
  };

  interface Props extends Partial<SourceIntelligenceCallbacks> {
    preview: SourcePreview;
    content?: string;
    editable?: boolean;
    visible?: boolean;
    externalDiagnostics?: SourceDiagnostic[];
    restoredViewStates?: Record<string, object>;
    loading?: boolean;
    targetLine?: number | null;
    targetLineRequestId?: number;
    onContentChange?: (content: string) => void;
    onRestoredViewStateConsumed?: (path: string) => void;
    onExternalNavigation?: (request: { path: string; line: number; column: number }) => void | Promise<void>;
    onSaveRequest?: () => void;
    onSymbolsChange?: (symbols: SourceSymbol[]) => void;
  }

  let {
    preview,
    content,
    editable = false,
    visible = true,
    externalDiagnostics = [],
    restoredViewStates = {},
    targetLine = null,
    targetLineRequestId = 0,
    onContentChange,
    onRestoredViewStateConsumed,
    onCompletionLookup,
    onDefinitionLookup,
    onExternalNavigation,
    onHoverLookup,
    onReferenceLookup,
    onReferenceCountsOutOfDate,
    onSaveRequest,
    onSymbolsChange
  }: Props = $props();

  let host: HTMLDivElement;
  let view: EditorView | null = null;
  let currentPath = '';
  let applyingContent = false;
  let languageGeneration = 0;
  let requestedLanguageKey = '';
  let loadedLanguageKey = '';
  let referenceCountTimer: ReturnType<typeof setTimeout> | null = null;
  const language = new Compartment();
  const editing = new Compartment();
  const sessionViewStates = new Map<string, StoredViewState>();
  const sessionEditorStates = new Map<string, EditorState>();

  function clearReferenceCountTimer(): void {
    if (referenceCountTimer !== null) clearTimeout(referenceCountTimer);
    referenceCountTimer = null;
  }

  function desiredContent(): string {
    return content ?? preview.content;
  }

  function editingExtensions(): Extension {
    return [EditorState.readOnly.of(!editable), EditorView.editable.of(editable)];
  }

  function languageKey(path: string, languageId: string): string {
    return `${path}\0${languageId}`;
  }

  function clearLanguageSupport(): void {
    languageGeneration += 1;
    requestedLanguageKey = '';
    loadedLanguageKey = '';
    if (view) view.dispatch({ effects: language.reconfigure([]) });
  }

  function loadVisibleLanguageSupport(): void {
    if (!view || !visible || !currentPath || currentPath !== preview.path) return;
    const path = currentPath;
    const languageId = preview.language;
    const key = languageKey(path, languageId);
    if (key === loadedLanguageKey || key === requestedLanguageKey) return;
    requestedLanguageKey = key;
    const generation = ++languageGeneration;
    void loadCodeMirrorLanguage(languageId)
      .then((extension) => {
        if (
          !view ||
          !visible ||
          generation !== languageGeneration ||
          currentPath !== path ||
          preview.path !== path ||
          preview.language !== languageId
        ) return;
        requestedLanguageKey = '';
        loadedLanguageKey = key;
        view.dispatch({ effects: language.reconfigure(extension) });
      })
      .catch(() => {
        if (generation === languageGeneration && requestedLanguageKey === key) requestedLanguageKey = '';
      });
  }

  function restoredState(path: string): StoredViewState | null {
    const candidate = restoredViewStates[path] ?? sessionViewStates.get(path);
    if (!candidate || typeof candidate !== 'object') return null;
    const state = candidate as Partial<StoredViewState>;
    if (
      typeof state.anchor !== 'number' ||
      typeof state.head !== 'number' ||
      typeof state.scrollTop !== 'number'
    ) return null;
    return state as StoredViewState;
  }

  function rememberCurrentView(): void {
    if (!view || !currentPath) return;
    sessionEditorStates.set(currentPath, view.state);
    setCodeMirrorEditorStateCount(sessionEditorStates.size);
    const selection = view.state.selection.main;
    sessionViewStates.set(currentPath, {
      anchor: selection.anchor,
      head: selection.head,
      scrollTop: view.scrollDOM.scrollTop
    });
  }

  function linePosition(state: EditorState, lineNumber: number, column = 1): number {
    const line = state.doc.line(Math.max(1, Math.min(lineNumber, state.doc.lines)));
    return Math.min(line.to, line.from + Math.max(0, column - 1));
  }

  function lookupRequestAt(state: EditorState, position = state.selection.main.head): SourceLookupRequest | null {
    const line = state.doc.lineAt(position);
    const offset = position - line.from;
    const text = line.text;
    let start = offset;
    let end = offset;
    while (start > 0 && /[\w$]/.test(text[start - 1])) start -= 1;
    while (end < text.length && /[\w$]/.test(text[end])) end += 1;
    const symbolName = text.slice(start, end);
    if (!symbolName) return null;
    return {
      symbolName,
      line: line.number,
      column: start + 1,
      filePath: preview.path
    };
  }

  async function navigate(kind: 'definition' | 'references', position?: number): Promise<void> {
    if (!view) return;
    const request = lookupRequestAt(view.state, position);
    if (!request) return;
    const targets = kind === 'definition'
      ? await onDefinitionLookup?.(request)
      : await onReferenceLookup?.(request);
    const target = targets?.[0];
    if (!target) return;
    if (target.path === preview.path) {
      const at = linePosition(view.state, target.line, target.column);
      view.dispatch({
        selection: { anchor: at },
        effects: EditorView.scrollIntoView(at, { y: 'center' })
      });
      view.focus();
      return;
    }
    await onExternalNavigation?.({ path: target.path, line: target.line, column: target.column });
  }

  async function completionSource(context: CompletionContext) {
    if (!onCompletionLookup) return null;
    const word = context.matchBefore(/[\w$]*/);
    if (!word || (word.from === word.to && !context.explicit)) return null;
    const request = lookupRequestAt(context.state, context.pos) ?? {
      symbolName: word.text,
      line: context.state.doc.lineAt(context.pos).number,
      column: context.pos - context.state.doc.lineAt(context.pos).from + 1,
      filePath: preview.path
    };
    const items = await onCompletionLookup(request);
    if (!items?.length) return null;
    return {
      from: word.from,
      options: items.map((item) => ({
        label: item.label,
        detail: item.detail || undefined,
        apply: item.insertText || item.label,
        type: item.kind.toLowerCase()
      })),
      validFor: /^[\w$]*$/
    };
  }

  const hover = hoverTooltip(async (editor, position) => {
    if (!onHoverLookup) return null;
    const request = lookupRequestAt(editor.state, position);
    if (!request) return null;
    const answer = await onHoverLookup(request);
    if (!answer?.contents.length) return null;
    const line = editor.state.doc.lineAt(position);
    const start = line.from + Math.max(0, request.column - 1);
    return {
      pos: start,
      end: Math.min(line.to, start + request.symbolName.length),
      above: true,
      create: () => {
        const dom = document.createElement('div');
        dom.className = 'cm-source-hover';
        dom.textContent = answer.contents.join('\n');
        return { dom };
      }
    };
  }, { hoverTime: 350, hideOnChange: true });

  function diagnosticsFor(state: EditorState): Diagnostic[] {
    return externalDiagnostics.map((diagnostic) => {
      const from = linePosition(state, diagnostic.line, diagnostic.column);
      return {
        from,
        to: Math.min(state.doc.length, from + 1),
        severity: diagnostic.severity === 'hint' ? 'info' : diagnostic.severity,
        message: diagnostic.message,
        source: diagnostic.source
      };
    });
  }

  const editorExtensions: Extension[] = [
    keymap.of([
      { key: 'Mod-s', run: () => { onSaveRequest?.(); return true; } },
      { key: 'F12', run: () => { void navigate('definition'); return true; } },
      { key: 'Shift-F12', run: () => { void navigate('references'); return true; } }
    ]),
    lineNumbers(),
    highlightActiveLineGutter(),
    highlightSpecialChars(),
    history({ minDepth: 5 }),
    foldGutter(),
    drawSelection(),
    dropCursor(),
    EditorState.allowMultipleSelections.of(true),
    indentOnInput(),
    syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
    bracketMatching(),
    closeBrackets(),
    rectangularSelection(),
    crosshairCursor(),
    highlightActiveLine(),
    highlightSelectionMatches(),
    keymap.of([
      ...closeBracketsKeymap,
      ...defaultKeymap,
      ...searchKeymap,
      ...historyKeymap,
      ...foldKeymap,
      ...completionKeymap,
      ...lintKeymap
    ]),
    codeMirrorTheme,
    lintGutter(),
    autocompletion({ override: [completionSource], activateOnTypingDelay: 180 }),
    hover,
    language.of([]),
    editing.of(editingExtensions()),
    EditorView.domEventHandlers({
      mousedown: (event, editor) => {
        if (!(event.metaKey || event.ctrlKey) || event.button !== 0) return false;
        const position = editor.posAtCoords({ x: event.clientX, y: event.clientY });
        if (position === null) return false;
        event.preventDefault();
        void navigate('definition', position);
        return true;
      }
    }),
    EditorView.updateListener.of((update) => {
      setCodeMirrorUndoDepth(undoDepth(update.state));
      if (currentPath) {
        sessionEditorStates.set(currentPath, update.state);
        setCodeMirrorEditorStateCount(sessionEditorStates.size);
      }
      if (!update.docChanged || applyingContent) return;
      const next = update.state.doc.toString();
      onContentChange?.(next);
      onSymbolsChange?.(extractSourceSymbols(preview, next));
      if (referenceCountTimer !== null) clearTimeout(referenceCountTimer);
      referenceCountTimer = setTimeout(() => {
        referenceCountTimer = null;
        onReferenceCountsOutOfDate?.(preview.path);
      }, 600);
    })
  ];

  function showFile(): void {
    if (!view) return;
    clearReferenceCountTimer();
    rememberCurrentView();
    currentPath = preview.path;
    const restored = restoredState(currentPath);
    const doc = desiredContent();
    const max = doc.length;
    const retained = sessionEditorStates.get(currentPath);
    let nextState = retained;
    if (!nextState) {
      nextState = EditorState.create({
        doc,
        selection: restored
          ? { anchor: Math.min(max, restored.anchor), head: Math.min(max, restored.head) }
          : undefined,
        extensions: editorExtensions
      });
    } else if (nextState.doc.toString() !== doc) {
      const selection = restored ?? nextState.selection.main;
      nextState = EditorState.create({
        doc,
        selection: {
          anchor: Math.min(max, selection.anchor),
          head: Math.min(max, selection.head)
        },
        extensions: editorExtensions
      });
    }
    applyingContent = true;
    view.setState(nextState);
    applyingContent = false;
    sessionEditorStates.set(currentPath, view.state);
    setCodeMirrorEditorStateCount(sessionEditorStates.size);
    setCodeMirrorDocBytes(textBytes(doc));
    setCodeMirrorUndoDepth(undoDepth(view.state));
    onSymbolsChange?.(extractSourceSymbols(preview, doc));
    view.dispatch(setDiagnostics(view.state, diagnosticsFor(view.state)));
    clearLanguageSupport();
    loadVisibleLanguageSupport();
    if (restored) {
      requestTrackedAnimationFrame(() => {
        if (!view || currentPath !== preview.path) return;
        view.scrollDOM.scrollTop = restored.scrollTop;
        if (restoredViewStates[currentPath]) onRestoredViewStateConsumed?.(currentPath);
      });
    }
  }

  export function captureViewStates(paths: readonly string[]): Record<string, object> {
    rememberCurrentView();
    const allowed = new Set(paths);
    return Object.fromEntries([...sessionViewStates].filter(([path]) => allowed.has(path)));
  }

  export function disposeTabModel(path: string): boolean {
    if (currentPath === path) currentPath = '';
    sessionEditorStates.delete(path);
    setCodeMirrorEditorStateCount(sessionEditorStates.size);
    return sessionViewStates.delete(path);
  }

  export function disposeAllTabModels(): void {
    currentPath = '';
    sessionViewStates.clear();
    sessionEditorStates.clear();
    setCodeMirrorEditorStateCount(0);
  }

  export function releaseSessionResources(): void {
    clearReferenceCountTimer();
    clearLanguageSupport();
    currentPath = '';
    sessionViewStates.clear();
    sessionEditorStates.clear();
    setCodeMirrorEditorStateCount(0);
    applyingContent = true;
    view?.setState(EditorState.create());
    applyingContent = false;
    setCodeMirrorDocBytes(0);
    setCodeMirrorUndoDepth(0);
  }

  $effect(() => {
    const path = preview.path;
    if (view && path !== currentPath) showFile();
  });

  $effect(() => {
    const next = desiredContent();
    if (!view || preview.path !== currentPath || view.state.doc.toString() === next) return;
    applyingContent = true;
    view.dispatch({
      changes: { from: 0, to: view.state.doc.length, insert: next },
      annotations: Transaction.addToHistory.of(false)
    });
    applyingContent = false;
    setCodeMirrorDocBytes(textBytes(next));
    setCodeMirrorUndoDepth(undoDepth(view.state));
  });

  $effect(() => {
    editable;
    if (view) view.dispatch({ effects: editing.reconfigure(editingExtensions()) });
  });

  $effect(() => {
    if (visible) loadVisibleLanguageSupport();
    else clearLanguageSupport();
  });

  $effect(() => {
    externalDiagnostics;
    if (view) view.dispatch(setDiagnostics(view.state, diagnosticsFor(view.state)));
  });

  $effect(() => {
    targetLineRequestId;
    if (!view || !targetLine || targetLine < 1) return;
    const position = linePosition(view.state, targetLine);
    view.dispatch({
      selection: { anchor: position },
      effects: EditorView.scrollIntoView(position, { y: 'center' })
    });
  });

  onMount(() => {
    view = new EditorView({ parent: host, state: EditorState.create({ extensions: editorExtensions }) });
    addCodeMirrorEditorView(1);
    showFile();
  });

  onDestroy(() => {
    clearReferenceCountTimer();
    languageGeneration += 1;
    view?.destroy();
    view = null;
    addCodeMirrorEditorView(-1);
    setCodeMirrorEditorStateCount(0);
    setCodeMirrorDocBytes(0);
    setCodeMirrorUndoDepth(0);
  });
</script>

<div class="codemirror-host" bind:this={host} aria-label={`Editor for ${preview.fileName}`}></div>

<style>
  .codemirror-host {
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    background: var(--color-surface, #17191e);
  }

  :global(.cm-source-hover) {
    max-width: min(680px, 70vw);
    padding: 7px 9px;
    white-space: pre-wrap;
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.45;
  }
</style>
