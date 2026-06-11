import assert from 'node:assert/strict';
import {
  closeOpenSourceTab,
  extractSourceSemanticTokens,
  extractSourceSymbols,
  findSourceDefinitionTargets,
  findSourceReferenceTargets,
  findSourceSearchMatches,
  formatSourceContextGitSummary,
  formatSourceContextIdentity,
  formatSourceContextRootLabel,
  formatSourceContextRuntime,
  formatSourceIndexSummary,
  formatSourceDiagnosticSummary,
  formatSourceRecordCount,
  formatSourceScanSummary,
  gitCommitGraphKind,
  gitRefLabels,
  folderIdsForSourceRecord,
  getSourceScanCacheEntry,
  monacoLanguageForSource,
  parseQuickOpenQuery,
  rankSourceRecords,
  removeSourceScanCacheEntries,
  scrollTopForSourceTreeReveal,
  selectPreferredSourceRecord,
  selectBackgroundIndexProjects,
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

const scanCache = upsertSourceScanCacheEntry({}, project, records, 2_000, 10_000);
assert.equal(getSourceScanCacheEntry(scanCache, project, 2_000, 10_500, 1_000)?.records.length, 2);
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
  'variable'
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
    ['variable', 6, 27, 8],
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
assert.equal(gitCommitGraphKind('', 0), 'commit');
assert.deepEqual(
  uniqueTaskIDsFromGitMetadata(
    [{ taskID: 'TSK-127' }, { taskID: null }],
    [{ taskID: 'tsk-126' }],
    [{ taskID: 'TSK-127' }]
  ),
  ['TSK-127', 'TSK-126']
);
assert.equal(taskReferenceUrl('TSK-127', { 'TSK-127': 'https://example.test/task' }), 'https://example.test/task');
assert.equal(taskReferenceUrl('tsk-192', {}), 'https://www.notion.so/search?q=TSK-192');
assert.equal(taskReferenceUrl('not-a-task', {}), null);
