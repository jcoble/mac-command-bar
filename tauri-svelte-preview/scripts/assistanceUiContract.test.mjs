import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';

const root = process.cwd();
const read = (relative) => fs.readFileSync(path.join(root, relative), 'utf8');
const overlays = read('src/lib/shell/components/ShellOverlays.svelte');
const host = read('src/lib/shell/assistance/AssistanceHost.svelte');
const proposal = read('src/lib/shell/assistance/AssistanceProposal.svelte');
const action = read('src/lib/shell/assistance/AssistanceAction.svelte');
const types = read('src/lib/shell/assistance/assistanceTypes.ts');

assert.equal((overlays.match(/<AssistanceHost\b/g) ?? []).length, 1, 'exactly one assistance host is mounted');
for (const label of ['Provenance', 'Confidence', 'Apply selected', 'Dismiss', 'Retry', 'Continue without AI']) {
  assert.match(proposal, new RegExp(label.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')));
}
assert.match(proposal, /checkbox/);
assert.match(host, /AssistanceAction/);
assert.match(action, /data-testid/);
for (const surface of ['conversation', 'git', 'diff', 'problems', 'run-configuration', 'browser', 'context', 'form', 'save', 'worktree']) {
  assert.match(types, new RegExp(`['"]${surface}['"]`), `${surface} has a typed contextual surface`);
}
assert.doesNotMatch(host, /\binvoke\s*\(|fetch\s*\(|writeFile|readFile/);
assert.match(proposal, /preview|confirm|revalidat|audit/i, 'consequential actions advertise the existing safety boundary');

console.log('assistanceUiContract: all tests passed');
