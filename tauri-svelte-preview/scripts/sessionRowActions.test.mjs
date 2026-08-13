/**
 * Pins the left-rail row quick-jump: the pure surface mapping, the thin
 * dispatch that runs it, and the wiring that makes the three buttons real.
 *
 * Run: node --experimental-strip-types scripts/sessionRowActions.test.mjs
 */
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import {
  SESSION_ROW_SURFACES,
  planSessionRowJump,
  registerSessionRowJumpTarget,
  sessionRowJump
} from '../src/lib/shell/components/sessionRowJump.ts';

const row = readFileSync(
  new URL('../src/lib/shell/components/WorktreeAgentRow.svelte', import.meta.url),
  'utf8'
);
const paneview = readFileSync(
  new URL('../src/lib/shell/components/SessionsPaneview.svelte', import.meta.url),
  'utf8'
);
const list = readFileSync(
  new URL('../src/lib/shell/components/MyWorkSessionList.svelte', import.meta.url),
  'utf8'
);
const centerDock = readFileSync(
  new URL('../src/lib/shell/layout/centerDock.ts', import.meta.url),
  'utf8'
);
const sidebarViews = readFileSync(
  new URL('../src/lib/shell/layout/sidebarViews.ts', import.meta.url),
  'utf8'
);
const shellPage = readFileSync(
  new URL('../src/routes/next/+page.svelte', import.meta.url),
  'utf8'
);

/** A target that records what it was asked to do, in order. */
function recordingTarget() {
  const calls = [];
  return {
    calls,
    selectSession: (ownedId) => calls.push(`select:${ownedId}`),
    showCenterPanel: (id) => calls.push(`center:${id}`),
    showSidebarView: (id) => calls.push(`view:${id}`)
  };
}

// --- the pure mapping ------------------------------------------------------

assert.deepEqual(
  [...SESSION_ROW_SURFACES],
  ['session', 'editor', 'source-control'],
  'the row offers exactly three jump surfaces'
);

assert.deepEqual(planSessionRowJump('s1', 'session'), {
  ownedId: 's1',
  centerPanelId: 'session',
  sidebarViewId: null
});

assert.deepEqual(planSessionRowJump('s1', 'editor'), {
  ownedId: 's1',
  centerPanelId: 'editor',
  sidebarViewId: null
});

assert.deepEqual(
  planSessionRowJump('s1', 'source-control'),
  { ownedId: 's1', centerPanelId: null, sidebarViewId: 'source-control' },
  'source control is a tool-column view, not a center tab'
);

assert.deepEqual(
  planSessionRowJump('  s1  ', 'session'),
  { ownedId: 's1', centerPanelId: 'session', sidebarViewId: null },
  'the session id is trimmed'
);

assert.equal(planSessionRowJump('', 'session'), null, 'no session, no jump');
assert.equal(planSessionRowJump('   ', 'session'), null);
assert.equal(planSessionRowJump(null, 'session'), null);
assert.equal(planSessionRowJump('s1', 'terminal'), null, 'an unknown surface plans nothing');

assert.deepEqual(
  planSessionRowJump('s1', 'editor'),
  planSessionRowJump('s1', 'editor'),
  'the mapping is pure: same input, same plan'
);

// Every planned id must be one the shell can actually open.
for (const surface of SESSION_ROW_SURFACES) {
  const plan = planSessionRowJump('s1', surface);
  if (plan.centerPanelId !== null) {
    assert.ok(
      centerDock.includes(`'${plan.centerPanelId}'`),
      `center dock must know the panel id ${plan.centerPanelId}`
    );
  }
  if (plan.sidebarViewId !== null) {
    assert.ok(
      sidebarViews.includes(`id: '${plan.sidebarViewId}'`),
      `the tool column must know the view id ${plan.sidebarViewId}`
    );
  }
}

// --- the dispatch ----------------------------------------------------------

assert.equal(
  sessionRowJump('s1', 'session'),
  false,
  'with nothing registered the jump reports that it did not happen'
);

const target = recordingTarget();
const release = registerSessionRowJumpTarget(target);

assert.equal(sessionRowJump('s1', 'session'), true);
assert.deepEqual(
  target.calls,
  ['select:s1', 'center:session'],
  'the session is activated BEFORE the surface is selected'
);

target.calls.length = 0;
assert.equal(sessionRowJump('s2', 'editor'), true);
assert.deepEqual(target.calls, ['select:s2', 'center:editor']);

target.calls.length = 0;
assert.equal(sessionRowJump('s3', 'source-control'), true);
assert.deepEqual(target.calls, ['select:s3', 'view:source-control']);

target.calls.length = 0;
assert.equal(sessionRowJump('', 'session'), false, 'an unplannable jump does nothing');
assert.equal(sessionRowJump('s1', 'nowhere'), false);
assert.deepEqual(target.calls, [], 'and it never touches the target');

const second = recordingTarget();
const releaseSecond = registerSessionRowJumpTarget(second);
target.calls.length = 0;
assert.equal(sessionRowJump('s4', 'session'), true);
assert.deepEqual(second.calls, ['select:s4', 'center:session'], 'the newest host wins');
assert.deepEqual(target.calls, [], 'the replaced host is not called');

// A stale release must not unregister the host that replaced it.
release();
second.calls.length = 0;
assert.equal(sessionRowJump('s5', 'session'), true);
assert.deepEqual(second.calls, ['select:s5', 'center:session']);

releaseSecond();
assert.equal(sessionRowJump('s6', 'session'), false, 'releasing the host stops the jumps');

// --- the wiring ------------------------------------------------------------

assert.ok(
  row.includes('sessionRowJump(session.ownedId, surface)'),
  'the row dispatches its jumps through the shared mapping'
);
for (const surface of SESSION_ROW_SURFACES) {
  assert.ok(
    row.includes(`jump(event, '${surface}')`),
    `the row must offer a ${surface} jump`
  );
}

assert.match(
  row,
  /import \{ IconButton \}/,
  'the row quick-jump buttons come from the kit, not hand-rolled markup'
);
assert.match(
  row,
  /size="xs"/,
  'kit icon buttons in the row are the 24px size'
);
for (const label of ['Open session', 'Open editor', 'Open source control']) {
  assert.ok(row.includes(label), `the row must label its jump button "${label}"`);
}

assert.match(row, /SessionHoverCard/, 'hover details use the dedicated rail card');
assert.match(row, /onmouseenter=\{showOverlay\}/, 'the action cluster opens on pointer hover');
assert.match(row, /onfocusin=\{showOverlay\}/, 'the action cluster opens on keyboard focus');
assert.match(row, /data-presence=\{presence\}/, 'presence stays a compact row state');
assert.match(row, /label="Start session"/, 'stopped rows keep Start as the first hover action');
assert.match(
  row,
  /const presenceIsRestart = \$derived\(presence === 'stopped'\)/,
  'only genuinely stopped rows offer Start, so suspended idle rows do not'
);
assert.match(row, /Idle — resumes on click/, 'suspended rows explain that a click resumes them');
assert.match(
  row,
  /\.presence\.idle\.suspended \.presence-dot \{ opacity: 0\.6; \}/,
  'suspended rows keep the solid idle dot and dim it to 60 percent'
);
assert.doesNotMatch(row, /No project recorded/i, 'the rail never renders the placeholder project sentence');

assert.match(
  shellPage,
  /registerSessionRowJumpTarget\(\{[\s\S]*?selectSession[\s\S]*?showCenterPanel[\s\S]*?showSidebarView[\s\S]*?\}\)/,
  'the page registers the one host that can activate a session and a surface'
);
assert.match(
  shellPage,
  /if \(activation\.kind === 'view'\) \{[\s\S]*?if \(switching\) \{[\s\S]*?ensureStructuredConversation\(/,
  'a rail selection change notifies the backend even when the conversation is already connected'
);
const viewActivationBlock = shellPage.match(
  /if \(activation\.kind === 'view'\) \{[\s\S]*?\n      \}/
);
assert.ok(viewActivationBlock, 'the connected view-only activation branch stays explicit');
assert.equal(
  viewActivationBlock[0].match(/ensureStructuredConversation\(/g)?.length,
  1,
  'one selection change can issue at most one connected-session activation notification'
);

// --- the scroll chain ------------------------------------------------------

const paneSlot = paneview.match(/\.pane-slot \{[^}]*\}/);
assert.ok(paneSlot, 'SessionsPaneview must still style its parked pane slots');
assert.doesNotMatch(
  paneSlot[0],
  /overflow:\s*hidden/,
  'a clipped pane slot hides rows from the scrollable pane body'
);
assert.match(
  paneSlot[0],
  /min-height:\s*100%/,
  'the pane slot fills its pane and grows past it so the pane body scrolls'
);

assert.match(
  list,
  /data-testid="my-work-session-list"[^>]*class="[^"]*overflow-y-auto/,
  'the ungrouped My Work list scrolls on its own'
);

console.log('sessionRowActions: ok');
