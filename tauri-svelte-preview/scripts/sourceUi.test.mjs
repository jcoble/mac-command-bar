import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const pageSource = await readFile(new URL('../src/routes/+page.svelte', import.meta.url), 'utf8');
const editorSource = await readFile(
  new URL('../src/lib/MonacoSourceEditor.svelte', import.meta.url),
  'utf8'
);
const appearanceSource = await readFile(
  new URL('../src/lib/sourcePreviewAppearance.ts', import.meta.url),
  'utf8'
);

function blockFor(selector) {
  const escapedSelector = selector.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const match = new RegExp(`${escapedSelector}\\s*\\{(?<body>[^}]*)\\}`, 'm').exec(pageSource);
  assert.ok(match?.groups?.body, `Missing style block for ${selector}`);
  return match.groups.body;
}

function assertDeclaration(selector, declaration) {
  assert.match(blockFor(selector), new RegExp(`(^|\\n)\\s*${declaration.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\s*;`), `${selector} should include ${declaration}`);
}

assertDeclaration('.source-browser-stack', 'overflow: hidden');
assertDeclaration('.source-list-panel', 'overflow: hidden');
assertDeclaration('.file-tree', 'overflow-y: auto');
assertDeclaration('.file-tree', 'overflow-x: hidden');
assertDeclaration('.file-tree', 'scrollbar-gutter: stable');
assertDeclaration('.file-tree', 'scrollbar-width: thin');
assert.ok(pageSource.includes('.file-tree::-webkit-scrollbar'), 'Tree view should style WebKit scrollbars');
assert.ok(pageSource.includes('function cancelSourceScan()'), 'Source scans should expose a cancel action');
assert.ok(
  pageSource.includes('onclick={scanning ? cancelSourceScan : () => scanProject(selectedProject, undefined, { force: true })}'),
  'Scan button should become a cancel button while a scan is running'
);
assert.ok(pageSource.includes("aria-label={scanning ? 'Stop source scan' : 'Scan source files'}"), 'Scan button should announce stop state');
assert.ok(pageSource.includes("<span>{scanning ? 'Stop' : 'Scan'}</span>"), 'Scan button label should switch to Stop while scanning');
assert.ok(pageSource.includes('virtualizeSourceTreeRows'), 'Source tree should use virtualized row slicing');
assert.ok(pageSource.includes('bind:this={fileTreeElement}'), 'File tree should bind the scroll container for viewport measurement');
assert.ok(pageSource.includes('onscroll={handleFileTreeScroll}'), 'File tree should track scroll position for virtualization');
assert.ok(pageSource.includes('class="tree-virtual-spacer"'), 'File tree should render spacer rows for offscreen content');
assertDeclaration('.tree-virtual-spacer', 'height: var(--tree-spacer-height)');
assert.ok(pageSource.includes('scrollTopForSourceTreeReveal'), 'Source tree should calculate scroll reveal positions');
assert.ok(pageSource.includes('let pendingTreeRevealPath'), 'Source tree should track a pending selected file reveal');
assert.ok(pageSource.includes('function requestSourceTreeReveal'), 'Source tree should expose a selected file reveal request');
assert.ok(pageSource.includes('requestSourceTreeReveal(record)'), 'Selecting a source record should request tree reveal');
assert.ok(pageSource.includes('let pendingTreeFocusRowIndex'), 'Source tree should track pending keyboard focus');
assert.ok(pageSource.includes('function focusTreeRowAtIndex'), 'Source tree should expose indexed row focus');
assert.ok(pageSource.includes('function handleTreeRowKeydown'), 'Source tree should handle keyboard navigation');
assert.ok(pageSource.includes('data-tree-row-index={virtualizedTreeRows.startIndex + virtualTreeRowIndex}'), 'Rendered tree rows should expose their absolute row index');
assert.ok(pageSource.includes('onkeydown={(event) => handleTreeRowKeydown(row, event)}'), 'Rendered tree rows should bind keyboard handling');
assert.ok(pageSource.includes('class="icon-button quick-open-trigger"'), 'Project controls should expose a visible quick-open trigger');
assert.ok(pageSource.includes('onclick={openQuickOpen}'), 'Quick-open trigger should call the existing quick-open opener');
assert.ok(pageSource.includes('cancelSourceScanFromTauri'), 'Stop should call the native scan cancellation command');
assert.ok(pageSource.includes('sourceScanProgress'), 'Source preview should track native scan progress');
assert.ok(pageSource.includes('nativeSourceScanProgressEvent'), 'Source preview should subscribe to native scan progress events');
assert.ok(pageSource.includes('scanSummaryLabel'), 'Source tree should expose an explicit scan summary label');
assert.ok(pageSource.includes('class="scan-summary"'), 'Source tree should render the scan summary below the heading');
assertDeclaration('.scan-summary', 'overflow: hidden');
assertDeclaration('.scan-summary', 'text-overflow: ellipsis');
assert.ok(pageSource.includes('expandedSourceScanLimit'), 'Source preview should expose an expanded scan limit');
assert.ok(pageSource.includes('{#if scanLimitReached && !scanning}'), 'Truncated source scans should expose a rescan-more action');
assert.ok(pageSource.includes('class="scan-more-button"'), 'Expanded source scans should use a compact action button');
assert.ok(
  pageSource.includes('scanProject(selectedProject, selectedRecord?.path, { force: true, limit: expandedSourceScanLimit })'),
  'Expanded source scans should rescan the active project at the larger limit'
);
assertDeclaration('.scan-more-button', 'white-space: nowrap');
assert.ok(editorSource.includes('basic-languages/dart/dart.contribution'), 'Editor should load Dart highlighting');
assert.ok(editorSource.includes('basic-languages/hcl/hcl.contribution'), 'Editor should load HCL highlighting');
assert.ok(editorSource.includes('basic-languages/lua/lua.contribution'), 'Editor should load Lua highlighting');
assert.ok(editorSource.includes('basic-languages/php/php.contribution'), 'Editor should load PHP highlighting');
assert.ok(editorSource.includes('basic-languages/protobuf/protobuf.contribution'), 'Editor should load Protobuf highlighting');
assert.ok(editorSource.includes('editable?: boolean'), 'Editor should expose an editable mode prop');
assert.ok(editorSource.includes('content?: string'), 'Editor should allow external draft content');
assert.ok(
  editorSource.includes('onContentChange?: (content: string) => void'),
  'Editor should report Monaco content changes to the parent'
);
assert.ok(
  editorSource.includes('onDidChangeModelContent'),
  'Editor should subscribe to Monaco content changes'
);
assert.ok(pageSource.includes('sourceDraftContentByPath'), 'Source page should track draft content per path');
assert.ok(pageSource.includes('savedSourceContentByPath'), 'Source page should track saved content per path');
assert.ok(pageSource.includes('writeSourceToTauri'), 'Source page should use the native write command');
assert.ok(pageSource.includes('function saveSelectedSourceFile'), 'Source page should expose a save action');
assert.ok(pageSource.includes('function revertSelectedSourceFile'), 'Source page should expose a revert action');
assert.ok(
  pageSource.includes('class:dirty={isSourcePathDirty(tab.path)}'),
  'Source tabs should show dirty state per path'
);
assert.ok(pageSource.includes('class="tab-dirty-dot"'), 'Dirty tabs should include a compact dirty marker');
assert.ok(pageSource.includes('aria-label="Save source file"'), 'Editor toolbar should expose save');
assert.ok(pageSource.includes('aria-label="Revert source file"'), 'Editor toolbar should expose revert');
assert.ok(
  pageSource.includes('content={selectedSourceDraftContent}'),
  'Monaco should receive the active source draft content'
);
assert.ok(pageSource.includes('editable={true}'), 'Monaco should run in editable mode');
assert.ok(
  pageSource.includes('onContentChange={updateSelectedSourceDraft}'),
  'Monaco should update the active draft content'
);
assert.ok(
  editorSource.includes('vs/language/typescript/monaco.contribution'),
  'Editor should load the Monaco TypeScript language service'
);
assert.ok(
  editorSource.includes('vs/language/typescript/ts.worker?worker'),
  'Editor should route TypeScript language requests to the TypeScript worker'
);
assert.ok(
  editorSource.includes('function configureTypeScriptLanguageService'),
  'Editor should configure TypeScript diagnostics'
);
assert.ok(
  editorSource.includes('onDidChangeMarkers'),
  'Editor should subscribe to Monaco diagnostics marker changes'
);
assert.ok(
  editorSource.includes('onDiagnosticsChange?: (diagnostics: SourceDiagnostic[]) => void'),
  'Editor should report diagnostics to the parent'
);
assert.ok(
  editorSource.includes('onSymbolsChange?: (symbols: SourceSymbol[]) => void'),
  'Editor should report symbols to the parent'
);
assert.ok(editorSource.includes('intelligenceCommand?: SourceEditorIntelligenceCommand | null'), 'Editor should accept language-intelligence commands');
assert.ok(
  editorSource.includes('onDefinitionLookup?: (symbolName: string) => void'),
  'Editor should report the current word for project definition lookup'
);
assert.ok(editorSource.includes('getWordAtPosition'), 'Editor should read the symbol under the cursor');
assert.ok(editorSource.includes('editor.action.showHover'), 'Editor should expose a hover command');
assert.ok(editorSource.includes('editor.action.revealDefinition'), 'Editor should expose go-to-definition');
assert.ok(pageSource.includes('sourceDiagnostics'), 'Source page should track diagnostics');
assert.ok(pageSource.includes('sourceSymbols'), 'Source page should track symbols');
assert.ok(pageSource.includes('sourceDefinitionTargets'), 'Source page should track project definition lookup targets');
assert.ok(pageSource.includes('formatSourceDiagnosticSummary'), 'Source page should summarize diagnostics');
assert.ok(pageSource.includes('sourceSupportsLanguageIntelligence'), 'Source page should gate Monaco language actions');
assert.ok(pageSource.includes('requestSourceIntelligenceAction'), 'Source page should dispatch language actions');
assert.ok(pageSource.includes('findSourceDefinitionsFromTauri'), 'Source page should call native project definition lookup');
assert.ok(pageSource.includes('findSourceDefinitionTargets'), 'Source page should fall back to browser definition lookup');
assert.ok(pageSource.includes('function runSourceDefinitionLookup'), 'Source page should expose project definition lookup');
assert.ok(pageSource.includes('function handleEditorDefinitionLookup'), 'Source page should receive editor definition lookup requests');
assert.ok(pageSource.includes('onDefinitionLookup={handleEditorDefinitionLookup}'), 'Editor should be wired to project definition lookup');
assert.ok(pageSource.includes('source-intelligence-panel'), 'Source page should render language intelligence panel');
assert.ok(pageSource.includes('aria-label="Show hover"'), 'Editor controls should expose hover');
assert.ok(pageSource.includes('aria-label="Go to definition"'), 'Editor controls should expose definition');
assert.ok(pageSource.includes('Problems'), 'Language panel should include Problems');
assert.ok(pageSource.includes('Symbols'), 'Language panel should include Symbols');
assert.ok(pageSource.includes('selectSourceSymbol'), 'Symbol rows should reveal source lines');
assert.ok(pageSource.includes('class="definition-results"'), 'Language panel should show project definition lookup results');
assertDeclaration('.definition-results', 'overflow-y: auto');
assertDeclaration('.definition-results', 'scrollbar-width: thin');
assert.ok(
  appearanceSource.includes('encodedTokensColors'),
  'Theme should include an encoded token palette for sharper Monaco token color separation'
);
assert.ok(
  appearanceSource.includes("token: 'keyword.public'"),
  'Theme should target C# modifier keyword tokens'
);
assert.ok(
  appearanceSource.includes("token: 'delimiter.bracket'"),
  'Theme should target Monaco bracket delimiter tokens'
);
assert.ok(
  editorSource.includes('sourceSemanticTokenLegend'),
  'Editor should import the source semantic-token legend'
);
assert.ok(
  editorSource.includes('extractSourceSemanticTokens'),
  'Editor should extract semantic tokens for Monaco'
);
assert.ok(
  editorSource.includes('registerDocumentSemanticTokensProvider'),
  'Editor should register a semantic-token provider'
);
assert.ok(
  /["']semanticHighlighting\.enabled["']:\s*true/.test(editorSource),
  'Editor should enable semantic highlighting explicitly'
);
assert.ok(pageSource.includes('backgroundIndexingProjectIDs'), 'Source page should track projects indexing in the background');
assert.ok(pageSource.includes('function indexProjectsInBackground'), 'Source page should start background project indexing');
assert.ok(pageSource.includes('selectBackgroundIndexProjects'), 'Source page should use the background index selection helper');
assert.ok(pageSource.includes('formatSourceIndexSummary'), 'Source page should format visible project index status');
assert.ok(pageSource.includes('class="index-summary"'), 'Source Browser should render an index status line');
assertDeclaration('.index-summary', 'overflow: hidden');
assertDeclaration('.index-summary', 'text-overflow: ellipsis');
assert.ok(pageSource.includes('readProjectGitStatusFromTauri'), 'Source page should load native project Git status');
assert.ok(pageSource.includes('projectGitStatus'), 'Source page should track selected project Git status');
assert.ok(pageSource.includes('gitStatusByRelativePath'), 'Source page should map Git file status by relative path');
assert.ok(pageSource.includes('class="project-git-pill"'), 'Topbar should render a compact project Git branch/status pill');
assert.ok(pageSource.includes('class="git-status-badge"'), 'Tree and tabs should render file-level Git status badges');
assertDeclaration('.git-status-badge', 'font-family: ui-monospace, SFMono-Regular, "SF Mono", Menlo, Monaco, Consolas, monospace');
assertDeclaration('.project-git-pill', 'overflow: hidden');
assert.ok(pageSource.includes('searchSourceFilesFromTauri'), 'Source page should call native global source search');
assert.ok(pageSource.includes('sourceSearchQuery'), 'Source page should track a global source search query');
assert.ok(pageSource.includes('sourceSearchResults'), 'Source page should track global source search results');
assert.ok(pageSource.includes('function runGlobalSourceSearch'), 'Source page should expose a global source search action');
assert.ok(pageSource.includes('function selectSourceSearchResult'), 'Source page should open search results at their line');
assert.ok(pageSource.includes('class="global-search-panel"'), 'Source page should render a global source search panel');
assert.ok(pageSource.includes('class="source-search-results"'), 'Source page should render source search results');
assertDeclaration('.source-search-results', 'overflow-y: auto');
assertDeclaration('.source-search-results', 'scrollbar-width: thin');
