import assert from 'node:assert/strict';
import {
  closeOpenSourceTab,
  extractSourceSymbols,
  formatSourceDiagnosticSummary,
  formatSourceRecordCount,
  formatSourceScanSummary,
  folderIdsForSourceRecord,
  getSourceScanCacheEntry,
  monacoLanguageForSource,
  parseQuickOpenQuery,
  rankSourceRecords,
  scrollTopForSourceTreeReveal,
  selectPreferredSourceRecord,
  sourceSupportsLanguageIntelligence,
  upsertSourceScanCacheEntry,
  upsertOpenSourceTab,
  upsertRecentSourceRecord,
  virtualizeSourceTreeRows
} from '../src/lib/sourceData.ts';

const project = {
  id: 'project-1',
  name: 'Project One',
  path: '/repo'
};

const records = [
  {
    path: '/repo/src/A.ts',
    relativePath: 'src/A.ts',
    fileName: 'A.ts',
    language: 'typescript',
    byteCount: 10
  },
  {
    path: '/repo/src/B.ts',
    relativePath: 'src/B.ts',
    fileName: 'B.ts',
    language: 'typescript',
    byteCount: 20
  }
];

const otherProject = {
  id: 'project-2',
  name: 'Project Two',
  path: '/repo-other'
};

assert.equal(selectPreferredSourceRecord(records, '/repo/src/B.ts')?.path, '/repo/src/B.ts');
assert.equal(
  selectPreferredSourceRecord(records, '/repo/src/Missing.ts', '/repo/src/A.ts')?.path,
  '/repo/src/A.ts'
);
assert.equal(selectPreferredSourceRecord([], '/repo/src/B.ts'), null);

const firstRecent = upsertRecentSourceRecord([], records[0], project, 1000, 5);
assert.deepEqual(firstRecent, [
  {
    ...records[0],
    projectID: 'project-1',
    projectName: 'Project One',
    openedAt: 1000
  }
]);

const secondRecent = upsertRecentSourceRecord(firstRecent, records[1], project, 2000, 5);
assert.deepEqual(
  secondRecent.map((record) => record.path),
  ['/repo/src/B.ts', '/repo/src/A.ts']
);

const bumpedRecent = upsertRecentSourceRecord(secondRecent, records[0], project, 3000, 5);
assert.deepEqual(
  bumpedRecent.map((record) => [record.path, record.openedAt]),
  [
    ['/repo/src/A.ts', 3000],
    ['/repo/src/B.ts', 2000]
  ]
);

const limitedRecent = upsertRecentSourceRecord(bumpedRecent, records[1], project, 4000, 1);
assert.deepEqual(
  limitedRecent.map((record) => record.path),
  ['/repo/src/B.ts']
);

assert.equal(formatSourceRecordCount(2, 2, false), '2');
assert.equal(formatSourceRecordCount(2_000, 2_000, true), '2,000+');
assert.equal(formatSourceRecordCount(12, 2_000, true), '12 / 2,000+');
assert.equal(formatSourceScanSummary(2, 2, false, ''), '2 indexed files');
assert.equal(formatSourceScanSummary(1, 2, false, 'b'), '1 match for "b" across 2 files');
assert.equal(
  formatSourceScanSummary(12, 2_000, true, 'service'),
  '12 matches for "service" across first 2,000 files'
);

const scanCache = upsertSourceScanCacheEntry({}, project, records, 2_000, 10_000);
assert.equal(getSourceScanCacheEntry(scanCache, project, 2_000, 10_500, 1_000)?.records.length, 2);
assert.equal(getSourceScanCacheEntry(scanCache, project, 1_000, 10_500, 1_000), null);
assert.equal(getSourceScanCacheEntry(scanCache, otherProject, 2_000, 10_500, 1_000), null);
assert.equal(getSourceScanCacheEntry(scanCache, project, 2_000, 12_000, 1_000), null);

const boundedScanCache = upsertSourceScanCacheEntry(
  upsertSourceScanCacheEntry(
    upsertSourceScanCacheEntry({}, project, [records[0]], 2_000, 10_000, 2),
    otherProject,
    [records[1]],
    2_000,
    11_000,
    2
  ),
  { ...project, id: 'project-3', path: '/third' },
  records,
  2_000,
  12_000,
  2
);
assert.deepEqual(
  Object.values(boundedScanCache).map((entry) => entry.projectPath),
  ['/repo-other', '/third']
);

const firstTab = upsertOpenSourceTab([], records[0], project, 1000, 3);
assert.deepEqual(
  firstTab.map((record) => [record.path, record.openedAt]),
  [['/repo/src/A.ts', 1000]]
);

const secondTab = upsertOpenSourceTab(firstTab, records[1], project, 2000, 3);
assert.deepEqual(
  secondTab.map((record) => record.path),
  ['/repo/src/A.ts', '/repo/src/B.ts']
);

const reselectedTab = upsertOpenSourceTab(secondTab, records[0], project, 3000, 3);
assert.deepEqual(
  reselectedTab.map((record) => [record.path, record.openedAt]),
  [
    ['/repo/src/A.ts', 3000],
    ['/repo/src/B.ts', 2000]
  ]
);

const closeInactive = closeOpenSourceTab(reselectedTab, '/repo/src/B.ts', '/repo/src/A.ts');
assert.deepEqual(closeInactive.tabs.map((record) => record.path), ['/repo/src/A.ts']);
assert.equal(closeInactive.nextActivePath, '/repo/src/A.ts');

const closeActive = closeOpenSourceTab(reselectedTab, '/repo/src/A.ts', '/repo/src/A.ts');
assert.deepEqual(closeActive.tabs.map((record) => record.path), ['/repo/src/B.ts']);
assert.equal(closeActive.nextActivePath, '/repo/src/B.ts');

const closeOnly = closeOpenSourceTab(firstTab, '/repo/src/A.ts', '/repo/src/A.ts');
assert.deepEqual(closeOnly.tabs, []);
assert.equal(closeOnly.nextActivePath, null);

const rankedByFileName = rankSourceRecords(records, 'b', 5);
assert.deepEqual(
  rankedByFileName.map((record) => record.path),
  ['/repo/src/B.ts']
);

const rankedByFileNameWithLine = rankSourceRecords(records, 'b:20', 5);
assert.deepEqual(
  rankedByFileNameWithLine.map((record) => record.path),
  ['/repo/src/B.ts']
);

const rankedEmptyQuery = rankSourceRecords(records, '', 1);
assert.deepEqual(
  rankedEmptyQuery.map((record) => record.path),
  ['/repo/src/A.ts']
);

const rankedByRelativePath = rankSourceRecords(
  [
    ...records,
    {
      path: '/repo/tools/Generate.ts',
      relativePath: 'tools/Generate.ts',
      fileName: 'Generate.ts',
      language: 'typescript',
      byteCount: 30
    }
  ],
  'tools',
  5
);
assert.deepEqual(
  rankedByRelativePath.map((record) => record.path),
  ['/repo/tools/Generate.ts']
);

assert.deepEqual(parseQuickOpenQuery('FormatDetector:20'), {
  searchQuery: 'FormatDetector',
  targetLine: 20
});
assert.deepEqual(parseQuickOpenQuery('apps/web/+page.svelte:003'), {
  searchQuery: 'apps/web/+page.svelte',
  targetLine: 3
});
assert.deepEqual(parseQuickOpenQuery('FormatDetector:'), {
  searchQuery: 'FormatDetector:',
  targetLine: null
});
assert.deepEqual(parseQuickOpenQuery('FormatDetector:0'), {
  searchQuery: 'FormatDetector:0',
  targetLine: null
});

assert.equal(monacoLanguageForSource('csharp'), 'csharp');
assert.equal(monacoLanguageForSource('tsx'), 'typescript');
assert.equal(monacoLanguageForSource('jsx'), 'javascript');
assert.equal(monacoLanguageForSource('svelte'), 'html');
assert.equal(monacoLanguageForSource('toml'), 'ini');
assert.equal(monacoLanguageForSource('plain'), 'plaintext');
assert.equal(sourceSupportsLanguageIntelligence('typescript'), true);
assert.equal(sourceSupportsLanguageIntelligence('tsx'), true);
assert.equal(sourceSupportsLanguageIntelligence('javascript'), true);
assert.equal(sourceSupportsLanguageIntelligence('csharp'), false);

assert.equal(formatSourceDiagnosticSummary([]), 'No problems');
assert.equal(
  formatSourceDiagnosticSummary([
    { severity: 'error', message: 'Broken syntax', line: 2, column: 4 },
    { severity: 'warning', message: 'Unused value', line: 6, column: 12 },
    { severity: 'warning', message: 'Deprecated API', line: 8, column: 2 }
  ]),
  '1 error, 2 warnings'
);

const typescriptSymbols = extractSourceSymbols(
  {
    path: '/repo/src/Widget.ts',
    relativePath: 'src/Widget.ts',
    fileName: 'Widget.ts',
    language: 'typescript',
    byteCount: 100,
    content: '',
    lineCount: 0
  },
  [
    'export interface WidgetProps {',
    '  name: string;',
    '}',
    'export class WidgetController {',
    '  mount() {}',
    '}',
    'export function createWidget() {',
    '  return new WidgetController();',
    '}',
    'const widgetCache = new Map<string, WidgetController>();'
  ].join('\n')
);
assert.deepEqual(
  typescriptSymbols.map((symbol) => [symbol.kind, symbol.name, symbol.line]),
  [
    ['interface', 'WidgetProps', 1],
    ['class', 'WidgetController', 4],
    ['function', 'createWidget', 7],
    ['constant', 'widgetCache', 10]
  ]
);

const csharpSymbols = extractSourceSymbols(
  {
    path: '/repo/src/Widget.cs',
    relativePath: 'src/Widget.cs',
    fileName: 'Widget.cs',
    language: 'csharp',
    byteCount: 100,
    content: '',
    lineCount: 0
  },
  [
    'namespace Demo;',
    'public sealed class WidgetController',
    '{',
    '    public Task MountAsync() => Task.CompletedTask;',
    '}'
  ].join('\n')
);
assert.deepEqual(
  csharpSymbols.map((symbol) => [symbol.kind, symbol.name, symbol.line]),
  [
    ['namespace', 'Demo', 1],
    ['class', 'WidgetController', 2],
    ['method', 'MountAsync', 4]
  ]
);

assert.deepEqual(folderIdsForSourceRecord(records[0]), ['folder:src']);
assert.deepEqual(
  folderIdsForSourceRecord({
    path: '/repo/apps/web/src/routes/+page.svelte',
    relativePath: 'apps/web/src/routes/+page.svelte',
    fileName: '+page.svelte',
    language: 'svelte',
    byteCount: 40
  }),
  ['folder:apps', 'folder:apps/web', 'folder:apps/web/src', 'folder:apps/web/src/routes']
);

const virtualRows = Array.from({ length: 100 }, (_, index) => ({
  level: 0,
  node: {
    id: `file:${index}`,
    name: `${index}.ts`,
    relativePath: `${index}.ts`,
    file: null,
    children: []
  }
}));

const firstVirtualWindow = virtualizeSourceTreeRows(virtualRows, 0, 90, 30, 2);
assert.equal(firstVirtualWindow.startIndex, 0);
assert.equal(firstVirtualWindow.endIndex, 5);
assert.equal(firstVirtualWindow.topSpacerHeight, 0);
assert.equal(firstVirtualWindow.bottomSpacerHeight, 2_850);
assert.equal(firstVirtualWindow.rows.length, 5);

const middleVirtualWindow = virtualizeSourceTreeRows(virtualRows, 1_500, 90, 30, 2);
assert.equal(middleVirtualWindow.startIndex, 48);
assert.equal(middleVirtualWindow.endIndex, 55);
assert.equal(middleVirtualWindow.topSpacerHeight, 1_440);
assert.equal(middleVirtualWindow.bottomSpacerHeight, 1_350);
assert.equal(middleVirtualWindow.rows[0].node.id, 'file:48');

const clampedVirtualWindow = virtualizeSourceTreeRows(virtualRows, 9_000, 90, 30, 2);
assert.equal(clampedVirtualWindow.startIndex, 95);
assert.equal(clampedVirtualWindow.endIndex, 100);
assert.equal(clampedVirtualWindow.bottomSpacerHeight, 0);

assert.equal(scrollTopForSourceTreeReveal(2, 0, 90, 30), 0);
assert.equal(scrollTopForSourceTreeReveal(10, 0, 90, 30), 240);
assert.equal(scrollTopForSourceTreeReveal(1, 240, 90, 30), 30);
assert.equal(scrollTopForSourceTreeReveal(-1, 240, 90, 30), 240);
