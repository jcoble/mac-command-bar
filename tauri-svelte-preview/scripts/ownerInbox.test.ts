import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { workflowMachine } from '../src/lib/shell/workflows/workflowTypes.ts';

{ // inbox_routes_answers_to_owning_machine
  // An answer goes to the machine that owns the run: this Mac's engine, or that remote profile.
  assert.equal(workflowMachine('mac'), null);
  assert.equal(workflowMachine('workbox-test'), 'workbox-test');
}
{ // an_inbox_action_error_survives_the_live_reload
  // Every run change reloads the Inbox, so a reload may clear only its own failure, not an action's refusal.
  const source = readFileSync(new URL('../src/lib/shell/workflows/components/OwnerInbox.svelte', import.meta.url), 'utf8');
  const load = /async function load\([\s\S]*?\n  }\n/.exec(source)?.[0];
  assert.ok(load, 'the Inbox has a load function');
  assert.doesNotMatch(load, /\berror = /, 'a reload leaves the last action error alone');
}
console.log('ownerInbox tests passed');
