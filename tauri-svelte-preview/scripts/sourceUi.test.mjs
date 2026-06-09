import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const pageSource = await readFile(new URL('../src/routes/+page.svelte', import.meta.url), 'utf8');
const editorSource = await readFile(
  new URL('../src/lib/MonacoSourceEditor.svelte', import.meta.url),
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
