import assert from 'node:assert/strict';
import {
  defaultSourceScanLimit,
  expandedSourceScanLimit,
  nativeSourceScanProgressEvent,
  createSourceScanId,
  findSourceDefinitionsFromTauri,
  readProjectGitStatusFromTauri,
  searchSourceFilesFromTauri,
  writeSourceToTauri
} from '../src/lib/tauriSource.ts';

assert.equal(defaultSourceScanLimit, 2_000);
assert.equal(expandedSourceScanLimit, 5_000);
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
