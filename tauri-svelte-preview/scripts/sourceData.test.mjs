import assert from 'node:assert/strict';
import {
  applySourceTextEdits,
  buildGitTaskSourceGroups,
  closeAllCleanOpenSourceTabs,
  closeOpenSourceTab,
  closeOtherCleanOpenSourceTabs,
  extractSourceSemanticTokens,
  extractSourceSymbols,
  findSourceDefinitionTargets,
  findAdjacentSourceDiagnostic,
  findSourceReferenceTargets,
  findSourceSearchMatches,
  formatGitBranchHealthSummary,
  formatSourceContextGitSummary,
  formatSourceContextIdentity,
  formatSourceContextRootLabel,
  formatSourceContextRuntime,
  formatSourceIndexSummary,
  formatSourceDiagnosticSummary,
  formatSourceScanHealth,
  formatSourceRecordCount,
  formatSourceScanStats,
  formatSourceScanSummary,
  gitCommitGraphKind,
  gitCommitTopologyLabel,
  gitRefLabels,
  folderIdsForSourceRecord,
  getSourceScanCacheEntry,
  monacoLanguageForSource,
  navigateSourceHistoryBack,
  navigateSourceHistoryForward,
  parseQuickOpenQuery,
  pushSourceNavigationHistory,
  rankSourceRecords,
  removeSourceScanCacheEntries,
  scrollTopForSourceTreeReveal,
  selectPreferredSourceRecord,
  selectBackgroundIndexProjects,
  shouldRepairSuspiciousSourceScan,
  isSuspiciousSourceScanResult,
  sourceLanguageForPath,
  sourceSemanticTokenLegend,
  sourceSupportsLanguageIntelligence,
  taskReferenceUrl,
  textMatchesSearchTokens,
  uniqueTaskIDsFromGitMetadata,
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

const recordC = {
  path: '/repo/src/C.ts',
  relativePath: 'src/C.ts',
  fileName: 'C.ts',
  language: 'typescript',
  byteCount: 30
};

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

const navigationStart = { path: '/repo/src/A.ts', line: 12 };
const navigationTarget = { path: '/repo/src/B.ts', line: 4 };
const pushedNavigation = pushSourceNavigationHistory([], navigationStart, navigationTarget, 20);
assert.deepEqual(pushedNavigation, [navigationStart]);
assert.deepEqual(
  pushSourceNavigationHistory(pushedNavigation, navigationTarget, navigationTarget, 20),
  pushedNavigation
);
const limitedNavigation = pushSourceNavigationHistory(
  pushedNavigation,
  { path: '/repo/src/C.ts', line: 3 },
  { path: '/repo/src/D.ts', line: 1 },
  1
);
assert.deepEqual(limitedNavigation, [{ path: '/repo/src/C.ts', line: 3 }]);
const backNavigation = navigateSourceHistoryBack(
  [navigationStart],
  [],
  navigationTarget
);
assert.deepEqual(backNavigation, {
  target: navigationStart,
  backStack: [],
  forwardStack: [navigationTarget]
});
assert.deepEqual(
  navigateSourceHistoryForward(
    backNavigation.backStack,
    backNavigation.forwardStack,
    backNavigation.target
  ),
  {
    target: navigationTarget,
    backStack: [navigationStart],
    forwardStack: []
  }
);

assert.equal(formatSourceRecordCount(2, 2, false), '2');
assert.equal(formatSourceRecordCount(2_000, 2_000, true), '2,000+');
assert.equal(formatSourceRecordCount(12, 2_000, true), '12 / 2,000+');
assert.equal(
  formatSourceContextRootLabel('/Users/blackcolours/dev/work/EdiPlatform'),
  'main checkout'
);
assert.equal(
  formatSourceContextRootLabel(
    '/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-126-m3-design-polish'
  ),
  'worktree:tsk-126-m3-design-polish'
);
assert.equal(formatSourceContextRootLabel('/tmp/scratch'), 'scratch');
assert.equal(formatSourceContextRuntime(''), 'browser preview');
assert.equal(formatSourceContextRuntime('  tauri source scan  '), 'tauri source scan');
assert.equal(formatSourceContextGitSummary(null, true, ''), 'git loading');
assert.equal(formatSourceContextGitSummary(null, false, 'permission denied'), 'git unavailable');
assert.equal(
  formatSourceContextGitSummary(
    {
      branch: 'cdx/tsk-89-practice-playwright-cli-tests',
      ahead: 1,
      behind: 2,
      files: [{ relativePath: 'src/A.ts' }, { relativePath: 'src/B.ts' }]
    },
    false,
    ''
  ),
  'cdx/tsk-89-practice-playwright-cli-tests · ahead 1 · behind 2 · 2 changes'
);
assert.deepEqual(
  formatSourceContextIdentity(
    {
      id: 'ediplatform-worktree',
      name: 'EdiPlatform',
      path: '/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-126-m3-design-polish'
    },
    'cdx/tsk-126-m3-design-polish · 4 changes',
    'tauri source scan'
  ),
  {
    projectName: 'EdiPlatform',
    rootLabel: 'worktree:tsk-126-m3-design-polish',
    rootPath: '/Users/blackcolours/dev/work/worktrees/EdiPlatform/tsk-126-m3-design-polish',
    gitSummary: 'cdx/tsk-126-m3-design-polish · 4 changes',
    runtime: 'tauri source scan',
    summary:
      'EdiPlatform · worktree:tsk-126-m3-design-polish · cdx/tsk-126-m3-design-polish · 4 changes · tauri source scan'
  }
);
assert.equal(formatSourceScanSummary(2, 2, false, ''), '2 indexed files');
assert.equal(formatSourceScanSummary(1, 2, false, 'b'), '1 match for "b" across 2 files');
assert.equal(
  formatSourceScanSummary(12, 2_000, true, 'service'),
  '12 matches for "service" across first 2,000 files'
);
assert.equal(
  formatSourceScanStats({
    visitedEntries: 1_234,
    matchedFiles: 42,
    skippedDirectories: 8,
    unsupportedFiles: 912,
    unreadableEntries: 3
  }),
  '1,234 entries checked · 8 dirs skipped · 912 unsupported · 3 unreadable'
);
assert.equal(formatSourceScanStats(null), '');

const scanStats = {
  visitedEntries: 12,
  matchedFiles: 2,
  skippedDirectories: 1,
  unsupportedFiles: 4,
  unreadableEntries: 0
};
const scanCache = upsertSourceScanCacheEntry({}, project, records, 2_000, 10_000, 8, false, scanStats);
assert.equal(getSourceScanCacheEntry(scanCache, project, 2_000, 10_500, 1_000)?.records.length, 2);
assert.deepEqual(getSourceScanCacheEntry(scanCache, project, 2_000, 10_500, 1_000)?.stats, scanStats);
assert.equal(getSourceScanCacheEntry(scanCache, project, 1_000, 10_500, 1_000)?.limit, 2_000);
assert.equal(getSourceScanCacheEntry(scanCache, otherProject, 2_000, 10_500, 1_000), null);
assert.equal(getSourceScanCacheEntry(scanCache, project, 2_000, 12_000, 1_000), null);

assert.deepEqual(
  selectBackgroundIndexProjects([project, otherProject], 'project-1', scanCache, 10_500, 1_000, 2_000).map(
    (indexProject) => indexProject.id
  ),
  ['project-2']
);
assert.deepEqual(
  selectBackgroundIndexProjects([project, otherProject], 'project-1', scanCache, 12_000, 1_000, 2_000).map(
    (indexProject) => indexProject.id
  ),
  ['project-2']
);
assert.equal(
  formatSourceIndexSummary(scanCache['/repo::2000'], false, ''),
  'Index ready: 2 files'
);
assert.equal(
  formatSourceIndexSummary({ ...scanCache['/repo::2000'], truncated: true, records }, false, ''),
  'Index ready: 2+ files'
);
assert.equal(formatSourceIndexSummary(null, true, ''), 'Indexing in background');
assert.equal(formatSourceIndexSummary(null, false, 'No access'), 'Index failed: No access');
assert.equal(formatSourceIndexSummary(null, false, ''), 'Index not ready');
assert.equal(shouldRepairSuspiciousSourceScan(2, false, 25_000, 2), true);
assert.equal(shouldRepairSuspiciousSourceScan(0, false, 25_000, 2), false);
assert.equal(shouldRepairSuspiciousSourceScan(3, false, 25_000, 2), false);
assert.equal(shouldRepairSuspiciousSourceScan(2, true, 25_000, 2), false);
assert.equal(shouldRepairSuspiciousSourceScan(2, false, 2, 2), false);
assert.equal(isSuspiciousSourceScanResult(2, false, 25_000, 2), true);
assert.equal(isSuspiciousSourceScanResult(0, false, 25_000, 2), false);
assert.equal(isSuspiciousSourceScanResult(3, false, 25_000, 2), false);
assert.equal(isSuspiciousSourceScanResult(2, true, 25_000, 2), false);
assert.equal(isSuspiciousSourceScanResult(2, false, 2, 2), false);
assert.deepEqual(
  formatSourceScanHealth({
    totalCount: 220,
    filteredCount: 220,
    truncated: false,
    requestedLimit: 25_000,
    suspiciousThreshold: 2,
    query: '',
    scanning: false,
    loading: false,
    error: ''
  }),
  {
    status: 'ready',
    needsAttention: false,
    summary: 'Index healthy: 220 files indexed',
    action: null
  }
);
assert.deepEqual(
  formatSourceScanHealth({
    totalCount: 2,
    filteredCount: 2,
    truncated: false,
    requestedLimit: 25_000,
    suspiciousThreshold: 2,
    query: '',
    scanning: false,
    loading: false,
    error: ''
  }),
  {
    status: 'suspicious',
    needsAttention: true,
    summary:
      'Only 2 files indexed from a 25,000-file scan. Confirm the project root is the repository root, then reset the index.',
    action: 'Reset index'
  }
);
assert.equal(
  formatSourceScanHealth({
    totalCount: 2,
    filteredCount: 1,
    truncated: false,
    requestedLimit: 25_000,
    suspiciousThreshold: 2,
    query: 'format',
    scanning: false,
    loading: false,
    error: ''
  }).needsAttention,
  false
);
assert.equal(
  formatSourceScanHealth({
    totalCount: 2,
    filteredCount: 2,
    truncated: false,
    requestedLimit: 25_000,
    suspiciousThreshold: 2,
    query: '',
    scanning: true,
    loading: true,
    error: ''
  }).status,
  'scanning'
);
assert.deepEqual(
  formatSourceScanHealth({
    totalCount: 0,
    filteredCount: 0,
    truncated: false,
    requestedLimit: 25_000,
    suspiciousThreshold: 2,
    query: '',
    scanning: false,
    loading: false,
    error: 'No access'
  }),
  {
    status: 'error',
    needsAttention: true,
    summary: 'Scan failed: No access',
    action: 'Check root'
  }
);

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
assert.deepEqual(
  Object.values(removeSourceScanCacheEntries(boundedScanCache, { ...project, path: '/repo/' })).map(
    (entry) => entry.projectPath
  ),
  ['/repo-other', '/third']
);
assert.deepEqual(
  Object.values(removeSourceScanCacheEntries(boundedScanCache, otherProject)).map(
    (entry) => entry.projectPath
  ),
  ['/third']
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

const threeTabs = upsertOpenSourceTab(
  upsertOpenSourceTab(reselectedTab, recordC, project, 4000, 4),
  records[1],
  project,
  5000,
  4
);
const closeOtherClean = closeOtherCleanOpenSourceTabs(
  threeTabs,
  '/repo/src/A.ts',
  new Set(['/repo/src/C.ts'])
);
assert.deepEqual(
  closeOtherClean.tabs.map((record) => record.path),
  ['/repo/src/A.ts', '/repo/src/C.ts']
);
assert.equal(closeOtherClean.nextActivePath, '/repo/src/A.ts');
assert.equal(closeOtherClean.closedCount, 1);
assert.equal(closeOtherClean.retainedDirtyCount, 1);

const closeAllClean = closeAllCleanOpenSourceTabs(
  threeTabs,
  '/repo/src/A.ts',
  new Set(['/repo/src/C.ts'])
);
assert.deepEqual(
  closeAllClean.tabs.map((record) => record.path),
  ['/repo/src/C.ts']
);
assert.equal(closeAllClean.nextActivePath, '/repo/src/C.ts');
assert.equal(closeAllClean.closedCount, 2);
assert.equal(closeAllClean.retainedDirtyCount, 1);

const sortedDiagnostics = [
  { severity: 'warning', message: 'middle', line: 4, column: 2 },
  { severity: 'error', message: 'first', line: 2, column: 10 },
  { severity: 'hint', message: 'last', line: 8, column: 1 }
];
assert.equal(findAdjacentSourceDiagnostic([], 1, 1, 1), null);
assert.equal(findAdjacentSourceDiagnostic(sortedDiagnostics, 1, 1, 1)?.message, 'first');
assert.equal(findAdjacentSourceDiagnostic(sortedDiagnostics, 4, 2, 1)?.message, 'last');
assert.equal(findAdjacentSourceDiagnostic(sortedDiagnostics, 9, 1, 1)?.message, 'first');
assert.equal(findAdjacentSourceDiagnostic(sortedDiagnostics, 4, 2, -1)?.message, 'first');
assert.equal(findAdjacentSourceDiagnostic(sortedDiagnostics, 1, 1, -1)?.message, 'last');

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

const sourceSearchPreviews = [
  {
    ...records[0],
    content: ['export function createWidget() {', '  return "Widget";'].join('\n'),
    lineCount: 2
  },
  {
    ...records[1],
    content: ['export const ignored = true;', 'export const WidgetName = "B";'].join('\n'),
    lineCount: 2
  }
];
const sourceSearchMatches = findSourceSearchMatches(
  sourceSearchPreviews,
  'widget',
  2
);
assert.deepEqual(
  sourceSearchMatches.map((match) => [
    match.path,
    match.line,
    match.column,
    match.excerpt
  ]),
  [
    ['/repo/src/A.ts', 1, 23, 'export function createWidget() {'],
    ['/repo/src/A.ts', 2, 11, 'return "Widget";']
  ]
);
assert.deepEqual(findSourceSearchMatches([], '   ', 10), []);
assert.deepEqual(findSourceSearchMatches(sourceSearchPreviews, 'widget', 0), []);

const definitionPreviews = [
  {
    path: '/repo/src/FormatResolver.cs',
    relativePath: 'src/FormatResolver.cs',
    fileName: 'FormatResolver.cs',
    language: 'csharp',
    byteCount: 100,
    content: [
      'namespace Demo;',
      'public sealed class FormatResolver',
      '{',
      '    private readonly FormatDetector _detector;',
      '    public Task ResolveAsync() => _detector.DetectAsync();',
      '}'
    ].join('\n'),
    lineCount: 6
  },
  {
    path: '/repo/src/FormatDetector.cs',
    relativePath: 'src/FormatDetector.cs',
    fileName: 'FormatDetector.cs',
    language: 'csharp',
    byteCount: 100,
    content: [
      'namespace Demo;',
      'public sealed class FormatDetector',
      '{',
      '    public Task DetectAsync() => Task.CompletedTask;',
      '}'
    ].join('\n'),
    lineCount: 5
  },
  {
    path: '/repo/src/widgets.ts',
    relativePath: 'src/widgets.ts',
    fileName: 'widgets.ts',
    language: 'typescript',
    byteCount: 100,
    content: [
      'export interface WidgetProps {',
      '  name: string;',
      '}',
      'export function createWidget() {',
      '  return { name: "demo" };',
      '}'
    ].join('\n'),
    lineCount: 6
  }
];

assert.deepEqual(
  findSourceDefinitionTargets(definitionPreviews, 'FormatDetector', 10).map((target) => [
    target.path,
    target.symbolName,
    target.kind,
    target.line,
    target.column,
    target.detail
  ]),
  [
    [
      '/repo/src/FormatDetector.cs',
      'FormatDetector',
      'class',
      2,
      21,
      'public sealed class FormatDetector'
    ]
  ]
);
assert.deepEqual(
  findSourceDefinitionTargets(definitionPreviews, 'createWidget', 10).map((target) => [
    target.path,
    target.kind,
    target.line
  ]),
  [['/repo/src/widgets.ts', 'function', 4]]
);
assert.deepEqual(
  findSourceDefinitionTargets(definitionPreviews, 'formatdetector', 1).map((target) => target.path),
  ['/repo/src/FormatDetector.cs']
);
assert.deepEqual(findSourceDefinitionTargets(definitionPreviews, '   ', 10), []);

const referenceTargets = findSourceReferenceTargets(definitionPreviews, 'FormatDetector', 10);
assert.deepEqual(
  referenceTargets.map((target) => [target.path, target.line, target.column, target.excerpt]),
  [
    ['/repo/src/FormatResolver.cs', 4, 22, 'private readonly FormatDetector _detector;'],
    ['/repo/src/FormatDetector.cs', 2, 21, 'public sealed class FormatDetector']
  ]
);
assert.deepEqual(findSourceReferenceTargets(definitionPreviews, 'FormatDetector', 1), [
  referenceTargets[0]
]);
assert.deepEqual(findSourceReferenceTargets(definitionPreviews, '   ', 10), []);

assert.deepEqual(parseQuickOpenQuery('FormatDetector:20'), {
  searchQuery: 'FormatDetector',
  targetLine: 20
});
assert.deepEqual(parseQuickOpenQuery('apps/web/+page.svelte:003'), {
  searchQuery: 'apps/web/+page.svelte',
  targetLine: 3
});
assert.deepEqual(parseQuickOpenQuery('FormatDetector:'), {
  searchQuery: 'FormatDetector',
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
assert.equal(sourceSupportsLanguageIntelligence('csharp'), true);
assert.equal(sourceSupportsLanguageIntelligence('rust'), true);
assert.equal(sourceSupportsLanguageIntelligence('svelte'), true);
assert.equal(sourceSupportsLanguageIntelligence('python'), true);
assert.equal(sourceSupportsLanguageIntelligence('go'), true);
assert.equal(sourceSupportsLanguageIntelligence('markdown'), false);
assert.equal(sourceLanguageForPath('/repo/src/App.cs'), 'csharp');
assert.equal(sourceLanguageForPath('/repo/src/App.svelte'), 'svelte');
assert.equal(sourceLanguageForPath('/repo/src/App.unknown'), 'plain');
assert.equal(
  applySourceTextEdits('alpha\nbeta\ngamma\n', [
    {
      startLine: 2,
      startColumn: 1,
      endLine: 2,
      endColumn: 5,
      newText: 'BETA'
    },
    {
      startLine: 3,
      startColumn: 6,
      endLine: 3,
      endColumn: 6,
      newText: '!'
    }
  ]),
  'alpha\nBETA\ngamma!\n'
);
assert.equal(
  applySourceTextEdits('call(foo, bar)', [
    {
      startLine: 1,
      startColumn: 6,
      endLine: 1,
      endColumn: 9,
      newText: 'baz'
    },
    {
      startLine: 1,
      startColumn: 11,
      endLine: 1,
      endColumn: 14,
      newText: 'qux'
    }
  ]),
  'call(baz, qux)'
);

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
    ['method', 'mount', 5],
    ['function', 'createWidget', 7],
    ['constant', 'widgetCache', 10]
  ]
);

assert.deepEqual(sourceSemanticTokenLegend.tokenTypes, [
  'namespace',
  'class',
  'interface',
  'type',
  'enum',
  'function',
  'method',
  'property',
  'variable',
  'parameter',
  'enumMember',
  'typeParameter',
  'keyword',
  'string',
  'number',
  'operator',
  'comment'
]);

const typescriptSemanticTokens = extractSourceSemanticTokens(
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
  typescriptSemanticTokens.map((token) => [token.tokenType, token.line, token.startColumn, token.length]),
  [
    ['interface', 1, 18, 11],
    ['class', 4, 14, 16],
    ['method', 5, 3, 5],
    ['function', 7, 17, 12],
    ['variable', 10, 7, 11]
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
    '    private readonly FormatDetector _detector;',
    '    public WidgetController(FormatDetector detector) => _detector = detector;',
    '    public FormatDetector Detector { get; }',
    '    public Task MountAsync() => Task.CompletedTask;',
    '}'
  ].join('\n')
);
assert.deepEqual(
  csharpSymbols.map((symbol) => [symbol.kind, symbol.name, symbol.line]),
  [
    ['namespace', 'Demo', 1],
    ['class', 'WidgetController', 2],
    ['variable', '_detector', 4],
    ['constructor', 'WidgetController', 5],
    ['property', 'Detector', 6],
    ['method', 'MountAsync', 7]
  ]
);

const csharpSemanticTokens = extractSourceSemanticTokens(
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
    '    private readonly FormatDetector _detector;',
    '    public WidgetController(FormatDetector detector) => _detector = detector;',
    '    public FormatDetector Detector { get; }',
    '    public Task MountAsync() => Task.CompletedTask;',
    '}'
  ].join('\n')
);
assert.deepEqual(
  csharpSemanticTokens.map((token) => [token.tokenType, token.line, token.startColumn, token.length]),
  [
    ['namespace', 1, 11, 4],
    ['class', 2, 21, 16],
    ['variable', 4, 37, 9],
    ['method', 5, 12, 16],
    ['property', 6, 27, 8],
    ['method', 7, 17, 10]
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

assert.equal(
  textMatchesSearchTokens('context bottom', 'Move context to bottom', 'Context cards'),
  true
);
assert.equal(textMatchesSearchTokens('git push', 'Push repository', 'Git actions'), true);
assert.equal(
  textMatchesSearchTokens('context bottom', 'Move context to side', 'Context cards'),
  false
);
assert.equal(textMatchesSearchTokens('', 'Any command'), true);

assert.deepEqual(gitRefLabels('HEAD -> main, origin/main, tag: v0.1.0'), [
  'HEAD -> main',
  'origin/main',
  'tag: v0.1.0'
]);
assert.deepEqual(gitRefLabels(''), []);
assert.equal(gitCommitGraphKind('HEAD -> main, origin/main', 3), 'head');
assert.equal(gitCommitGraphKind('origin/main', 0), 'branch');
assert.equal(gitCommitGraphKind('', 0, 2), 'merge');
assert.equal(gitCommitGraphKind('', 0, 0), 'root');
assert.equal(gitCommitGraphKind('', 0), 'commit');
assert.equal(gitCommitTopologyLabel('HEAD -> main', 0), 'HEAD');
assert.equal(gitCommitTopologyLabel('', 0, 2), 'MERGE');
assert.equal(gitCommitTopologyLabel('', 0, 0), 'ROOT');
assert.deepEqual(
  formatGitBranchHealthSummary({
    branch: 'cdx/tsk-127-source-center',
    ahead: 2,
    behind: 1,
    stagedCount: 3,
    unstagedCount: 4,
    untrackedCount: 1,
    rootLabel: 'worktree:tsk-127',
    lastCommitSha: 'abc1234'
  }),
  {
    branch: 'cdx/tsk-127-source-center',
    sync: 'ahead 2 / behind 1',
    dirty: 'staged 3 / unstaged 4 / untracked 1',
    detail:
      'worktree:tsk-127 · cdx/tsk-127-source-center · ahead 2 / behind 1 · staged 3 / unstaged 4 / untracked 1 · abc1234',
    tone: 'dirty',
    chips: [
      { label: 'Branch', value: 'cdx/tsk-127-source-center', tone: 'dirty' },
      { label: 'Sync', value: 'ahead 2 / behind 1', tone: 'warning' },
      { label: 'Worktree', value: 'staged 3 / unstaged 4 / untracked 1', tone: 'dirty' },
      { label: 'Root', value: 'worktree:tsk-127', tone: 'muted' },
      { label: 'Head', value: 'abc1234', tone: 'muted' }
    ]
  }
);
assert.deepEqual(
  formatGitBranchHealthSummary({ branch: 'main', ahead: 0, behind: 0, changedCount: 0 }),
  {
    branch: 'main',
    sync: 'up to date',
    dirty: 'clean',
    detail: 'main · up to date · clean',
    tone: 'clean',
    chips: [
      { label: 'Branch', value: 'main', tone: 'clean' },
      { label: 'Sync', value: 'up to date', tone: 'clean' },
      { label: 'Worktree', value: 'clean', tone: 'clean' }
    ]
  }
);
assert.deepEqual(
  uniqueTaskIDsFromGitMetadata(
    [{ taskID: 'TSK-127' }, { taskID: null }],
    [{ taskID: 'tsk-126' }],
    [{ taskID: 'TSK-127' }]
  ),
  ['TSK-127', 'TSK-126']
);
assert.deepEqual(
  buildGitTaskSourceGroups(
    [
      { taskID: 'tsk-127', sourceLabel: 'repo', sourceDetail: 'main checkout' },
      { taskID: 'not-a-task', sourceLabel: 'repo', sourceDetail: 'ignored' }
    ],
    [{ taskID: 'TSK-127', sourceLabel: 'commit', sourceDetail: 'b022003' }],
    [{ taskID: 'TSK-128', sourceLabel: 'worktree', sourceDetail: 'tsk-128-editor' }]
  ),
  [
    {
      taskID: 'TSK-127',
      sourceSummary: 'repo, commit',
      detailSummary: 'repo: main checkout · commit: b022003',
      sources: [
        { taskID: 'TSK-127', sourceLabel: 'repo', sourceDetail: 'main checkout' },
        { taskID: 'TSK-127', sourceLabel: 'commit', sourceDetail: 'b022003' }
      ]
    },
    {
      taskID: 'TSK-128',
      sourceSummary: 'worktree',
      detailSummary: 'worktree: tsk-128-editor',
      sources: [{ taskID: 'TSK-128', sourceLabel: 'worktree', sourceDetail: 'tsk-128-editor' }]
    }
  ]
);
assert.equal(taskReferenceUrl('TSK-127', { 'TSK-127': 'https://example.test/task' }), 'https://example.test/task');
assert.equal(taskReferenceUrl('tsk-192', {}), 'https://www.notion.so/search?q=TSK-192');
assert.equal(taskReferenceUrl('not-a-task', {}), null);
