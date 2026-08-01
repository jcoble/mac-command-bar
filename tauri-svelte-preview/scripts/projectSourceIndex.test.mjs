import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
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
