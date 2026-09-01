import { RangeSetBuilder, type Extension } from '@codemirror/state';
import { Decoration, EditorView, ViewPlugin, WidgetType, type DecorationSet, type ViewUpdate } from '@codemirror/view';

import { formatSourceCodeLensTitle, sourceCodeLensCountKey } from '$lib/sourceCodeLensKeys';
import { dotnetWorkspaceLensTitles, type DotnetWorkspaceAction } from '$lib/workspaceCodeLens';
import type { SourceLookupRequest } from './sourceIntelligence';
import type { SourcePreview, SourceReferenceTarget, SourceSymbol } from '$lib/sourceData';

type Count = { count: number; atLeast: boolean };

export interface CodeMirrorCodeLensOptions {
  preview: SourcePreview;
  enabled: boolean;
  onAnchorLookup?: (preview: SourcePreview) => Promise<SourceSymbol[] | null>;
  onCount?: (request: SourceLookupRequest) => Count | null | Promise<Count | null>;
  onReferences?: (request: SourceLookupRequest) => Promise<SourceReferenceTarget[]>;
  onOpenReference?: (target: SourceReferenceTarget) => void | Promise<void>;
  onDotnetAction?: (action: DotnetWorkspaceAction) => void | Promise<void>;
}

type LensRow = {
  key: string;
  request?: SourceLookupRequest;
  action?: DotnetWorkspaceAction;
  line: number;
  title: string;
  state: 'loading' | 'ready' | 'unavailable';
};

const CODE_LENS_KINDS = new Set(['class', 'interface', 'method', 'function', 'constructor', 'property', 'enum', 'struct', 'record', 'variable']);

function isCodeLensSymbol(symbol: SourceSymbol): boolean {
  if (!CODE_LENS_KINDS.has(symbol.kind)) return false;
  return symbol.kind !== 'variable' || /^(?:public|private|protected|internal|static|readonly|const|required|volatile|new)\b/.test(symbol.detail);
}

function requestFor(symbol: SourceSymbol, filePath: string): SourceLookupRequest {
  const nameOffset = symbol.detail.lastIndexOf(symbol.name);
  return {
    symbolName: symbol.name,
    line: symbol.line,
    column: nameOffset >= 0 ? symbol.column + nameOffset : symbol.column,
    filePath
  };
}

class LensWidget extends WidgetType {
  constructor(
    readonly row: LensRow,
    readonly run: (row: LensRow) => void
  ) { super(); }

  eq(other: LensWidget): boolean {
    return other.row.key === this.row.key && other.row.title === this.row.title && other.row.state === this.row.state;
  }

  toDOM(): HTMLElement {
    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'cm-code-lens';
    button.classList.toggle('loading', this.row.state === 'loading');
    button.textContent = this.row.title;
    button.disabled = this.row.state === 'unavailable';
    button.onclick = () => this.run(this.row);
    return button;
  }

  ignoreEvent(): boolean { return false; }
}

class PeekWidget extends WidgetType {
  constructor(
    readonly request: SourceLookupRequest,
    readonly targets: SourceReferenceTarget[] | null,
    readonly open: (target: SourceReferenceTarget) => void
  ) { super(); }

  eq(other: PeekWidget): boolean {
    return other.request === this.request && other.targets === this.targets;
  }

  toDOM(): HTMLElement {
    const panel = document.createElement('span');
    panel.className = 'cm-code-lens-peek';
    const heading = document.createElement('strong');
    heading.textContent = this.targets === null
      ? `Finding references for ${this.request.symbolName}…`
      : `${this.targets.length} ${this.targets.length === 1 ? 'reference' : 'references'}`;
    panel.append(heading);
    for (const target of this.targets ?? []) {
      const button = document.createElement('button');
      button.type = 'button';
      button.textContent = `${target.relativePath || target.path}:${target.line}  ${target.excerpt.trim()}`;
      button.onclick = () => this.open(target);
      panel.append(button);
    }
    return panel;
  }

  ignoreEvent(): boolean { return false; }
}

const codeLensBaseTheme = EditorView.baseTheme({
  '.cm-code-lens': { border: '0', background: 'transparent', color: '#8aa9d6', cursor: 'pointer', padding: '0 8px 0 4px', fontSize: '11px' },
  '.cm-code-lens.loading': { color: '#8b909b', cursor: 'progress' },
  '.cm-code-lens-peek': { display: 'inline-grid', gap: '4px', padding: '8px 12px', borderBlock: '1px solid #343944', background: '#111318', fontSize: '12px' },
  '.cm-code-lens-peek button': { border: '0', background: 'transparent', color: '#c9d5e8', cursor: 'pointer', textAlign: 'left', fontFamily: 'inherit' }
});

export function codeMirrorCodeLens(options: CodeMirrorCodeLensOptions): Extension {
  const generation = Symbol(options.preview.path);

  return [
    ViewPlugin.fromClass(class {
      decorations: DecorationSet = Decoration.none;
      private rows: LensRow[] = [];
      private peek: { request: SourceLookupRequest; targets: SourceReferenceTarget[] | null } | null = null;
      private alive = true;
      private loadGeneration = 0;

      constructor(readonly view: EditorView) {
        if (options.preview.language.toLowerCase() === 'csharp' && options.onDotnetAction) {
          this.rows.push(
            { key: 'dotnet-build', action: 'build', line: 1, title: dotnetWorkspaceLensTitles.build, state: 'ready' },
            { key: 'dotnet-test', action: 'test', line: 1, title: dotnetWorkspaceLensTitles.test, state: 'ready' }
          );
        }
        if (options.enabled && options.onAnchorLookup && options.onCount) {
          this.rows.push({ key: 'reference-anchors', line: 1, title: 'Loading references…', state: 'loading' });
        }
        this.paint();
        if (options.enabled && options.onAnchorLookup && options.onCount) void this.loadReferences(generation);
      }

      update(update: ViewUpdate): void {
        if (update.docChanged) {
          this.loadGeneration += 1;
          this.rows = this.rows.filter((row) => Boolean(row.action));
          if (options.enabled && options.onAnchorLookup && options.onCount) {
            this.rows.push({ key: 'reference-anchors', line: 1, title: 'Loading references…', state: 'loading' });
          }
          this.peek = null;
          this.paint();
          if (options.enabled && options.onAnchorLookup && options.onCount) void this.loadReferences(generation);
        }
      }

      destroy(): void {
        this.alive = false;
        this.loadGeneration += 1;
      }

      private async loadReferences(token: symbol): Promise<void> {
        const load = ++this.loadGeneration;
        let symbols: SourceSymbol[] | null | undefined;
        try {
          symbols = await options.onAnchorLookup?.({ ...options.preview, content: this.view.state.doc.toString() });
        } catch {
          symbols = null;
        }
        if (!this.alive || token !== generation || load !== this.loadGeneration) return;
        if (!symbols) {
          this.rows = this.rows.filter((row) => row.key !== 'reference-anchors');
          this.paint(true);
          return;
        }
        const requests = symbols.filter(isCodeLensSymbol).slice(0, 120)
          .map((symbol) => requestFor(symbol, options.preview.path));
        this.rows = [
          ...this.rows.filter((row) => Boolean(row.action)),
          ...requests.map((request) => ({
            key: sourceCodeLensCountKey(options.preview.path, request),
            request,
            line: request.line,
            title: 'Loading references…',
            state: 'loading' as const
          }))
        ];
        this.paint(true);
        for (const request of requests) {
          const key = sourceCodeLensCountKey(options.preview.path, request);
          let count: Count | null | undefined;
          try {
            count = await options.onCount?.(request);
          } catch {
            count = null;
          }
          if (!this.alive || token !== generation || load !== this.loadGeneration) return;
          const index = this.rows.findIndex((candidate) => candidate.key === key);
          if (index < 0) return;
          this.rows[index] = {
            ...this.rows[index],
            state: count ? 'ready' : 'unavailable',
            title: count ? formatSourceCodeLensTitle(count.count, count.atLeast) : 'References unavailable'
          };
          this.paint(true);
        }
      }

      private run = (row: LensRow): void => {
        if (row.action) {
          void options.onDotnetAction?.(row.action);
          return;
        }
        if (!row.request || !options.onReferences) return;
        void this.openReferences(row.request);
      };

      private async openReferences(request: SourceLookupRequest): Promise<void> {
        this.peek = { request, targets: null };
        this.paint(true);
        let targets: SourceReferenceTarget[] = [];
        try {
          targets = await options.onReferences?.(request) ?? [];
        } catch {
          targets = [];
        }
        if (!this.alive || this.peek?.request !== request) return;
        this.peek = { request, targets };
        this.paint(true);
      }

      private paint(refresh = false): void {
        if (!this.alive) return;
        const builder = new RangeSetBuilder<Decoration>();
        const items: { at: number; side: number; decoration: Decoration }[] = [];
        for (const row of this.rows) {
          const line = this.view.state.doc.line(Math.max(1, Math.min(row.line, this.view.state.doc.lines)));
          items.push({ at: line.from, side: -1, decoration: Decoration.widget({ widget: new LensWidget(row, this.run), side: -1 }) });
        }
        if (this.peek) {
          const line = this.view.state.doc.line(Math.max(1, Math.min(this.peek.request.line, this.view.state.doc.lines)));
          items.push({
            at: line.to,
            side: 1,
            decoration: Decoration.widget({
              widget: new PeekWidget(this.peek.request, this.peek.targets, (target) => void options.onOpenReference?.(target)),
              side: 1
            })
          });
        }
        items.sort((left, right) => left.at - right.at || left.side - right.side);
        for (const item of items) builder.add(item.at, item.at, item.decoration);
        this.decorations = builder.finish();
        if (refresh) this.view.dispatch({});
      }
    }, { decorations: (plugin) => plugin.decorations }),
    codeLensBaseTheme
  ];
}
