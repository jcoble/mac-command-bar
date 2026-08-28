import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import {
  forgetAllProjectSourceRecords,
  forgetProjectSourceRecords,
  projectSourceRecords,
  setProjectSourceRecords
} from '../src/lib/shell/projectSourceIndex.ts';

const record = {
  path: '/repo/src/example.ts',
  relativePath: 'src/example.ts',
  fileName: 'example.ts',
  language: 'typescript',
  byteCount: 42
};

forgetAllProjectSourceRecords();
assert.deepEqual(projectSourceRecords('/repo'), []);

setProjectSourceRecords('/repo/', [record]);
assert.deepEqual(
  projectSourceRecords('/repo'),
  [record],
  'the editor can read the explorer scan through a normalized project root'
);

assert.deepEqual(projectSourceRecords('/other'), [], 'project indexes stay isolated');

forgetProjectSourceRecords('/repo');
assert.deepEqual(projectSourceRecords('/repo'), [], 'a failed rescan drops stale records');

// Only the three most recently indexed projects are kept: one record per file
// in a repository is far too much to retain for every root visited in a run.
{
  forgetAllProjectSourceRecords();
  const recordIn = (root) => [{ ...record, path: `${root}/example.ts` }];
  for (const root of ['/a', '/b', '/c', '/d']) setProjectSourceRecords(root, recordIn(root));
  assert.deepEqual(projectSourceRecords('/a'), [], 'the fourth project drops the oldest index');
  assert.deepEqual(projectSourceRecords('/d'), recordIn('/d'), 'the newest project is kept');
  assert.deepEqual(projectSourceRecords('/b'), recordIn('/b'));

  // Re-indexing a project makes it the most recent, so the next one past the
  // cap evicts whatever has gone longest without a scan.
  setProjectSourceRecords('/b', recordIn('/b'));
  setProjectSourceRecords('/e', recordIn('/e'));
  assert.deepEqual(projectSourceRecords('/c'), [], 'the least recently indexed project goes');
  assert.deepEqual(projectSourceRecords('/b'), recordIn('/b'), 're-indexing keeps a project');
  assert.deepEqual(projectSourceRecords('/d'), recordIn('/d'));
  assert.deepEqual(projectSourceRecords('/e'), recordIn('/e'));
  forgetAllProjectSourceRecords();
}

const here = path.dirname(fileURLToPath(import.meta.url));
const explorerService = fs.readFileSync(
  path.join(here, '..', 'src/lib/shell/explorer/explorerService.ts'),
  'utf8'
);
const sourceIntelligence = fs.readFileSync(
  path.join(here, '..', 'src/lib/shell/editor/sourceIntelligence.ts'),
  'utf8'
);

assert.match(
  explorerService,
  /setProjectSourceRecords\(target,\s*result\.records\)/,
  'a successful Explorer scan must publish its records to the shared index'
);
assert.match(
  sourceIntelligence,
  /projectSourceRecords\(projectRoot\)/,
  'source intelligence must consume the shared project records'
);
assert.match(
  sourceIntelligence,
  /if \(request\.filePath\)[\s\S]*semanticTargetsFor\(request\)[\s\S]*if \(cached\) return cached/,
  'a finished semantic CodeLens count must provide its targets without another lookup'
);
assert.match(
  sourceIntelligence,
  /resolveSemanticReferences\(preview, request, root\)[\s\S]*indexedReferences\(symbolName\)/,
  'a cold or unavailable language server must still fall back to the shared index'
);

test('activate_serves_cached_root_without_rescan', async () => {
  const rune = (value) => value;
  rune.raw = (value) => value;
  globalThis.$state = rune;
  globalThis.window = {};

  const recordsByRoot = new Map([
    ['/root-a', [{ ...record, path: '/root-a/a.ts', relativePath: 'a.ts', fileName: 'a.ts' }]],
    ['/root-b', [{ ...record, path: '/root-b/b.ts', relativePath: 'b.ts', fileName: 'b.ts' }]]
  ]);
  const scannedRoots = [];
  globalThis.fetch = async (_url, options) => {
    const { root } = JSON.parse(options.body);
    scannedRoots.push(root);
    return new Response(
      JSON.stringify({ records: recordsByRoot.get(root), limit: 10_000, truncated: false }),
      { headers: { 'content-type': 'application/json' } }
    );
  };

  forgetAllProjectSourceRecords();
  const { activate } = await import('../src/lib/shell/explorer/explorerService.ts');
  const { explorer, explorerRecords } = await import(
    '../src/lib/shell/explorer/explorerStore.svelte.ts'
  );
  const waitForScan = async () => {
    while (explorer.scanning) await new Promise((resolve) => setImmediate(resolve));
  };

  activate('/root-a');
  await waitForScan();
  activate('/root-b');
  await waitForScan();
  activate('/root-a');
  await waitForScan();

  assert.deepEqual(scannedRoots, ['/root-a', '/root-b']);
  assert.equal(explorer.root, '/root-a');
  assert.deepEqual(explorerRecords(), recordsByRoot.get('/root-a'));
});
