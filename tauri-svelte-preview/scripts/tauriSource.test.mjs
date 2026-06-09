import assert from 'node:assert/strict';
import {
  defaultSourceScanLimit,
  expandedSourceScanLimit,
  nativeSourceScanProgressEvent,
  createSourceScanId
} from '../src/lib/tauriSource.ts';

assert.equal(defaultSourceScanLimit, 2_000);
assert.equal(expandedSourceScanLimit, 5_000);
assert.equal(nativeSourceScanProgressEvent, 'source_scan_progress');
assert.match(createSourceScanId(), /^source-scan-\d+-[a-z0-9]+$/);
