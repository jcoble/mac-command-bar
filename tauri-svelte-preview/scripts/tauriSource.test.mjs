import assert from 'node:assert/strict';
import {
  defaultSourceScanLimit,
  expandedSourceScanLimit,
  nativeSourceScanProgressEvent,
  fetchGitRepositoryFromTauri,
  createSourceScanId,
  findSourceDefinitionsFromTauri,
  findSourceReferencesFromTauri,
  findSourceLspDefinitionsFromTauri,
  findSourceLspHoverFromTauri,
  findSourceLspReferencesFromTauri,
  findSourceLspSymbolsFromTauri,
  readSourceLspDiagnosticsFromTauri,
  readSourceLspStatusFromTauri,
  listAgentSessionsFromTauri,
  listGitRepositorySummariesFromTauri,
  listProjectWorktreesFromTauri,
  listRuntimeContextsFromTauri,
  openTerminalCommandFromTauri,
  openTerminalPathFromTauri,
  openPathFromTauri,
  readGitCommitHistoryFromTauri,
  readProjectGitStatusFromTauri,
  readSourceGitDiffFromTauri,
  revealPathFromTauri,
  searchSourceFilesFromTauri,
  stageGitPathsFromTauri,
  commitGitRepositoryFromTauri,
  pullGitRepositoryFromTauri,
  pushGitRepositoryFromTauri,
  unstageGitPathsFromTauri,
  writeSourceToTauri
} from '../src/lib/tauriSource.ts';

assert.equal(defaultSourceScanLimit, 5_000);
assert.equal(expandedSourceScanLimit, 10_000);
assert.equal(nativeSourceScanProgressEvent, 'source_scan_progress');
assert.match(createSourceScanId(), /^source-scan-\d+-[a-z0-9]+$/);
assert.equal(
  await writeSourceToTauri(
    {
      path: '/tmp/App.ts',
      relativePath: 'App.ts',
      fileName: 'App.ts',
      language: 'typescript',
      byteCount: 12
    },
    'export {};'
  ),
  null
);
assert.equal(await readProjectGitStatusFromTauri('/tmp/repo'), null);
assert.equal(await readSourceGitDiffFromTauri('/tmp/repo', '/tmp/repo/src/App.ts'), null);
assert.equal(await stageGitPathsFromTauri('/tmp/repo', ['src/App.ts']), null);
assert.equal(await unstageGitPathsFromTauri('/tmp/repo', ['src/App.ts']), null);
assert.equal(await commitGitRepositoryFromTauri('/tmp/repo', 'test commit'), null);
assert.equal(await fetchGitRepositoryFromTauri('/tmp/repo'), null);
assert.equal(await pullGitRepositoryFromTauri('/tmp/repo'), null);
assert.equal(await pushGitRepositoryFromTauri('/tmp/repo'), null);
assert.equal(await readGitCommitHistoryFromTauri('/tmp/repo'), null);
assert.equal(await openPathFromTauri('/tmp/repo'), false);
assert.equal(await revealPathFromTauri('/tmp/repo'), false);
assert.equal(await openTerminalPathFromTauri('/tmp/repo'), false);
assert.equal(await openTerminalCommandFromTauri('/tmp/repo', 'codex resume session-123'), false);
assert.equal(await listAgentSessionsFromTauri(), null);
assert.equal(
  await listGitRepositorySummariesFromTauri([{ id: 'repo', name: 'Repo', path: '/tmp/repo' }]),
  null
);
assert.equal(await listProjectWorktreesFromTauri('/tmp/repo'), null);
assert.equal(
  await listRuntimeContextsFromTauri([
    {
      id: 'repo',
      name: 'Repo',
      path: '/tmp/repo'
    }
  ]),
  null
);
assert.equal(
  await findSourceDefinitionsFromTauri(
    [
      {
        path: '/tmp/App.ts',
        relativePath: 'App.ts',
        fileName: 'App.ts',
        language: 'typescript',
        byteCount: 12
      }
    ],
    'App'
  ),
  null
);
assert.equal(
  await readSourceLspStatusFromTauri('/tmp/repo', 'typescript'),
  null
);
assert.equal(
  await findSourceLspDefinitionsFromTauri(
    {
      path: '/tmp/App.ts',
      relativePath: 'App.ts',
      fileName: 'App.ts',
      language: 'typescript',
      byteCount: 12,
      content: 'export class App {}',
      lineCount: 1
    },
    {
      root: '/tmp/repo',
      line: 1,
      column: 14,
      limit: 20
    }
  ),
  null
);
assert.equal(
  await findSourceLspReferencesFromTauri(
    {
      path: '/tmp/App.ts',
      relativePath: 'App.ts',
      fileName: 'App.ts',
      language: 'typescript',
      byteCount: 12,
      content: 'export class App {}',
      lineCount: 1
    },
    {
      root: '/tmp/repo',
      line: 1,
      column: 14,
      limit: 50
    }
  ),
  null
);
assert.equal(
  await findSourceLspHoverFromTauri(
    {
      path: '/tmp/App.ts',
      relativePath: 'App.ts',
      fileName: 'App.ts',
      language: 'typescript',
      byteCount: 12,
      content: 'export class App {}',
      lineCount: 1
    },
    {
      root: '/tmp/repo',
      line: 1,
      column: 14
    }
  ),
  null
);
assert.equal(
  await findSourceLspSymbolsFromTauri(
    {
      path: '/tmp/App.ts',
      relativePath: 'App.ts',
      fileName: 'App.ts',
      language: 'typescript',
      byteCount: 12,
      content: 'export class App {}',
      lineCount: 1
    },
    {
      root: '/tmp/repo',
      line: 1,
      column: 1,
      limit: 100
    }
  ),
  null
);
assert.equal(
  await readSourceLspDiagnosticsFromTauri(
    {
      path: '/tmp/App.ts',
      relativePath: 'App.ts',
      fileName: 'App.ts',
      language: 'typescript',
      byteCount: 12,
      content: 'export class App {}',
      lineCount: 1
    },
    {
      root: '/tmp/repo',
      line: 1,
      column: 14
    }
  ),
  null
);
assert.equal(
  await findSourceReferencesFromTauri(
    [
      {
        path: '/tmp/App.ts',
        relativePath: 'App.ts',
        fileName: 'App.ts',
        language: 'typescript',
        byteCount: 12
      }
    ],
    'App'
  ),
  null
);
assert.equal(
  await searchSourceFilesFromTauri(
    [
      {
        path: '/tmp/App.ts',
        relativePath: 'App.ts',
        fileName: 'App.ts',
        language: 'typescript',
        byteCount: 12
      }
    ],
    'app'
  ),
  null
);
