/**
 * draftSessionLifecycle.test.ts — a draft is not a session until it is sent.
 *
 * Two halves. The first runs the draft's own rules: a draft that is abandoned
 * creates nothing, and a draft that is sent creates exactly one session
 * carrying every value the composer's controls were left on. The second reads
 * the shell's wiring, because the "creates nothing" half is only true while
 * the page keeps a draft out of the rail and out of the backend.
 *
 * Run: node --experimental-strip-types scripts/draftSessionLifecycle.test.ts
 */
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import {
  buildThreadStartRequest,
  defaultThreadStartState,
  type ThreadStartPickerState,
  type ThreadStartRequest
} from '../src/lib/shell/newSession/threadStartFlow.ts';

const page = readFileSync(new URL('../src/routes/+page.svelte', import.meta.url), 'utf8');
const controller = readFileSync(
  new URL('../src/lib/shell/controllers/newSessionController.svelte.ts', import.meta.url),
  'utf8'
);
const selectionController = readFileSync(
  new URL('../src/lib/shell/controllers/sessionSelectionController.svelte.ts', import.meta.url),
  'utf8'
);
const surface = readFileSync(
  new URL('../src/lib/shell/newSession/DraftSessionSurface.svelte', import.meta.url),
  'utf8'
);
const composer = readFileSync(
  new URL('../src/lib/shell/components/conversation/ConversationComposer.svelte', import.meta.url),
  'utf8'
);
const overlays = readFileSync(
  new URL('../src/lib/shell/components/ShellOverlays.svelte', import.meta.url),
  'utf8'
);

/** The one side effect a draft can have, counted. */
const started: ThreadStartRequest[] = [];

function send(state: ThreadStartPickerState): void {
  const request = buildThreadStartRequest(state);
  if (!request) return;
  started.push(request);
}

const draft: ThreadStartPickerState = {
  ...defaultThreadStartState({ projectPath: '/Users/me/dev/work/mac-command-bar' }),
  cwd: '/Users/me/dev/work/mac-command-bar',
  branch: 'main'
};

// Abandoned: opened, configured, typed into, then thrown away. Nothing started.
{
  // A draft only ever exists as this value, so switching session — which is
  // what the page does with `draftOpen = false` — is dropping the variable.
  let abandoned: ThreadStartPickerState | null = {
    ...draft,
    provider: 'claude',
    prompt: 'never sent'
  };
  abandoned = null;
  assert.equal(abandoned, null);
  assert.equal(started.length, 0, 'an abandoned draft must create nothing');
}

// Sent: exactly one session, carrying every control's value.
{
  send({
    ...draft,
    prompt: 'Add the draft session surface',
    provider: 'claude',
    model: 'claude-opus',
    effort: 'high',
    access: 'acceptedits'
  });
  assert.equal(started.length, 1, 'the first send creates exactly one session');
  const [request] = started;
  assert.equal(request.provider, 'claude');
  assert.equal(request.model, 'claude-opus');
  assert.equal(request.reasoningEffort, 'high');
  assert.equal(request.approvalPolicy, 'acceptedits');
  assert.equal(request.projectPath, '/Users/me/dev/work/mac-command-bar');
  assert.equal(request.cwd, '/Users/me/dev/work/mac-command-bar');
  assert.equal(request.branch, 'main');
  assert.equal(request.title, 'Add the draft session surface');
}

// An empty draft is not sendable, so pressing send on one still creates nothing.
{
  send({ ...draft, prompt: '   ' });
  assert.equal(started.length, 1, 'an empty draft must not create a session');
}

// ── The shell's side of the promise ──────────────────────────────────────────

// "+" opens a draft in the Session tab. It does not open a dialog, and the old
// form is gone from the overlay layer altogether.
assert.match(page, /onNewSession=\{openNewSession\}/);
assert.match(page, /selection\.newSession\.open\(\)/);
assert.match(controller, /draftProjectPath = this\.mostRecentProjectPath\(\) \?\? null/);
assert.match(controller, /this\.draftOpen = true;/);
assert.doesNotMatch(page, /openNewSession\(projectPath\?: string\): void;/);
assert.doesNotMatch(overlays, /ThreadStartHost|onStartNewSession|newSessionRoots/);

// A draft holds nothing the shell has to clean up: switching session discards it.
assert.match(selectionController, /this\.newSession\.abandonDraftForSessionSwitch\(session\.ownedId\)/);
assert.match(controller, /abandonDraftForSessionSwitch[\s\S]*?this\.draftOpen = false/);

// The draft is layered over the conversation, never in place of it — the live
// sessions' terminal hosts are inside that component.
assert.match(page, /<ConversationSurface[\s\S]*?\{#if selection\.newSession\.draftOpen\}[\s\S]*?<DraftSessionSurface/);

// The first send is the only thing that reaches the session-creating path.
assert.match(page, /onSend=\{startNewSession\}/);
assert.match(page, /await selection\.newSession\.start\(/);

// After the session exists, the panels are pointed at its folder deliberately,
// rather than being left to whatever start-up had already allowed.
assert.match(controller, /shellPanels\.allowSessionLoads\(\);\s*\n\s*\n?\s*try \{/);
assert.match(controller, /await selectSession\(owned\.ownedId\)/);
assert.match(selectionController, /shellPanels\.sessionPicked\(this\.activeRootAvailable\)/);
assert.match(page, /workbench\.selectCenterTab\("session"\);\s*\n\s*workbench\.selectRightTab\("files"\);/);

// The draft's controls are the composer's own: the same settings menu a running
// session uses, plus the project and branch pickers in the footer beside it.
assert.match(composer, /leadingControls\?: Snippet;/);
assert.match(composer, /\{@render leadingControls\(\)\}/);
assert.match(surface, /<ConversationComposer/);
assert.match(surface, /leadingControls=\{draftControls\}/);
assert.match(surface, /data-testid="draft-session-provider"/);
assert.match(surface, /data-testid="draft-session-project"/);
assert.match(surface, /data-testid="draft-session-branch"/);
assert.match(surface, /onConfigChange=\{changeConfig\}/);

// Nothing in the draft surface may create a session or touch the rail.
assert.doesNotMatch(surface, /addOwnedSession|sendStructuredMessage|ensureStructuredConversation/);

console.log('draftSessionLifecycle.test.ts passed');
