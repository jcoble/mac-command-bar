import assert from 'node:assert/strict';
import { defaultSourceScanLimit } from '../src/lib/tauriSource.ts';

assert.equal(defaultSourceScanLimit, 2_000);
