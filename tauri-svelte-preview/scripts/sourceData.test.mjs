import assert from 'node:assert/strict';
import {
  closeOpenSourceTab,
  rankSourceRecords,
  selectPreferredSourceRecord,
  upsertOpenSourceTab,
  upsertRecentSourceRecord
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
