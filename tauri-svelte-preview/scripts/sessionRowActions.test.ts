/**
 * Pins the left-rail row quick-jump: the pure surface mapping, the thin
 * dispatch that runs it, and the wiring that makes the three buttons real.
 *
 * Run: node --experimental-strip-types scripts/sessionRowActions.test.ts
 */
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import {
  SESSION_ROW_SURFACES,
  planSessionRowJump,
  registerSessionRowJumpTarget,
  sessionRowJump
} from '../src/lib/shell/components/sessionRowJump.ts';
import { sessionRowMenuItems } from '../src/lib/shell/components/sessionRowMenu.ts';

const row = readFileSync(
  new URL('../src/lib/shell/components/WorktreeAgentRow.svelte', import.meta.url),
  'utf8'
);
const rail = readFileSync(
  new URL('../src/lib/shell/components/SessionRail.svelte', import.meta.url),
  'utf8'
);
const card = readFileSync(
  new URL('../src/lib/shell/components/SessionHoverCard.svelte', import.meta.url),
  'utf8'
);
const hoverActions = readFileSync(
  new URL('../src/lib/components/ui/hover-actions/hover-actions.svelte', import.meta.url),
  'utf8'
);
const hoverActionButton = readFileSync(
  new URL('../src/lib/components/ui/hover-actions/hover-action-button.svelte', import.meta.url),
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

interface RecordingTarget {
  calls: string[];
  selectSession(ownedId: string): Promise<void>;
  showCenterPanel(ownedId: string, id: string): void;
  showSidebarView(ownedId: string, id: string): void;
}

/** A target that records what it was asked to do, in order. */
function recordingTarget(): RecordingTarget {
  const calls: string[] = [];
  return {
    calls,
    selectSession: async (ownedId: string) => {
      calls.push(`select:${ownedId}`);
    },
    showCenterPanel: (_ownedId: string, id: string) => {
      calls.push(`center:${id}`);
    },
    showSidebarView: (_ownedId: string, id: string) => {
      calls.push(`view:${id}`);
    }
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
  if (plan === null) throw new Error(`a known surface must plan a jump: ${surface}`);
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
  await sessionRowJump('s1', 'session'),
  false,
  'with nothing registered the jump reports that it did not happen'
);

let delayedActiveOwnedId = 'old-session';
const delayedTabWrites: string[] = [];
const releaseDelayed = registerSessionRowJumpTarget({
  selectSession: async (ownedId: string) => {
    await Promise.resolve();
    delayedActiveOwnedId = ownedId;
  },
  showCenterPanel: (ownedId: string, id: string) => {
    delayedTabWrites.push(`${ownedId}:${id}:${delayedActiveOwnedId}`);
  },
  showSidebarView: () => undefined
});
assert.equal(await sessionRowJump('new-session', 'editor'), true);
assert.deepEqual(
  delayedTabWrites,
  ['new-session:editor:new-session'],
  'a delayed activation finishes before the tab is written for the jumped-to session'
);
releaseDelayed();

let activeOwnedId = 'active-session';
let activationCalls = 0;
const releaseActive = registerSessionRowJumpTarget({
  selectSession: async (ownedId: string) => {
    if (activeOwnedId === ownedId) return;
    activationCalls += 1;
    activeOwnedId = ownedId;
  },
  showCenterPanel: () => undefined,
  showSidebarView: () => undefined
});
assert.equal(await sessionRowJump('active-session', 'session'), true);
assert.equal(activationCalls, 0, 'jumping within the active session does not activate it again');
releaseActive();

const target = recordingTarget();
const release = registerSessionRowJumpTarget(target);

assert.equal(await sessionRowJump('s1', 'session'), true);
assert.deepEqual(
  target.calls,
  ['select:s1', 'center:session'],
  'the session is activated BEFORE the surface is selected'
);

target.calls.length = 0;
assert.equal(await sessionRowJump('s2', 'editor'), true);
assert.deepEqual(target.calls, ['select:s2', 'center:editor']);

target.calls.length = 0;
assert.equal(await sessionRowJump('s3', 'source-control'), true);
assert.deepEqual(target.calls, ['select:s3', 'view:source-control']);

target.calls.length = 0;
assert.equal(await sessionRowJump('', 'session'), false, 'an unplannable jump does nothing');
assert.equal(await sessionRowJump('s1', 'nowhere'), false);
assert.deepEqual(target.calls, [], 'and it never touches the target');

const second = recordingTarget();
const releaseSecond = registerSessionRowJumpTarget(second);
target.calls.length = 0;
assert.equal(await sessionRowJump('s4', 'session'), true);
assert.deepEqual(second.calls, ['select:s4', 'center:session'], 'the newest host wins');
assert.deepEqual(target.calls, [], 'the replaced host is not called');

// A stale release must not unregister the host that replaced it.
release();
second.calls.length = 0;
assert.equal(await sessionRowJump('s5', 'session'), true);
assert.deepEqual(second.calls, ['select:s5', 'center:session']);

releaseSecond();
assert.equal(await sessionRowJump('s6', 'session'), false, 'releasing the host stops the jumps');

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
  /import \{ HoverActionButton, HoverActions \} from '\$lib\/components\/ui\/hover-actions/,
  'the row hover buttons are the kit pattern, not hand-rolled markup'
);
assert.doesNotMatch(
  row,
  /import \{ IconButton \}/,
  'the row no longer styles icon buttons itself'
);
for (const label of ['Open session', 'Open editor', 'Open source control']) {
  assert.ok(row.includes(label), `the row must label its jump button "${label}"`);
}

assert.match(row, /SessionHoverCard/, 'hover details use the dedicated rail card');
assert.match(
  row,
  /\.session-title\s*\{[\s\S]*?min-width:\s*0;[\s\S]*?overflow:\s*hidden;[\s\S]*?text-overflow:\s*ellipsis;[\s\S]*?white-space:\s*nowrap;/,
  'long row titles keep one fixed-width ellipsis line'
);
assert.match(row, /class="action-reserve"/, 'line one always reserves the action cluster width');
assert.match(
  row,
  /\.action-reserve\s*\{[\s\S]*?width:\s*0;[\s\S]*?flex:\s*0 0 0px;[\s\S]*?\.row:hover \.action-reserve[\s\S]*?width:\s*119px;/,
  'action space is collapsed at rest and opens to 119px on hover'
);
assert.match(
  row,
  /class="absolute top-\[2px\] z-\[2\]"/,
  'the actions occupy line one without a hard-coded right offset'
);
assert.match(
  row,
  /\[data-slot='hover-actions'\]\)\s*\{\s*right:\s*calc\(11px \+ var\(--rail-status-room\)\);/,
  'the cluster hugs the rail edge, offset only by what the row state still needs'
);
assert.match(
  row,
  /\.row\[data-presence='working'\] \{ --rail-status-room: 19px; \}/,
  'a working row keeps just the spinner to the right of the cluster'
);
assert.match(
  row,
  /\.row:hover \.age-text,[\s\S]*?\.row:focus-within \.age-text \{ display: none; \}/,
  'hovering trades the elapsed time for the actions'
);
assert.match(
  row,
  /\[data-slot='hover-actions'\] svg\)[\s\S]*?width:\s*18px;\s*height:\s*18px;/,
  'row action glyphs are 18px'
);
assert.equal(
  (row.match(/size="sm"/g) ?? []).length,
  6,
  'the three jumps, Settle, Revert and Resume all use 28px kit buttons'
);
assert.match(
  row,
  /data-testid="worktree-agent-settle"[\s\S]*?label="Settle"[\s\S]*?onSettle\?\.\(\)/,
  'the hover cluster settles a session where its age sits at rest'
);
assert.match(
  row,
  /\{#if shelf === 'settled'\}[\s\S]*?data-testid="worktree-agent-unsettle"[\s\S]*?onUnsettle\?\.\(\)/,
  'a settled row offers the way back instead'
);
assert.doesNotMatch(
  row,
  /linear-gradient\(to right, transparent, var\(--color-hover\)\)/,
  'line-one actions need no opaque title-covering gradient'
);
assert.doesNotMatch(row, /:has\(|has-\[/, 'rail actions do not add relational selectors');

// The card is a still picture: what it shows is read once, on the way open.
assert.match(
  row,
  /function takeCardView\(\): HoverCardView \{/,
  'the row snapshots the card into plain values rather than binding it to stores'
);
assert.match(
  row,
  /\{#if cardPlacement && cardView\}/,
  'the card renders only when a snapshot was taken for it'
);
assert.match(
  row,
  /<SessionHoverCard \{\.\.\.cardView\} \/>/,
  'the card is fed the snapshot alone, so nothing live reaches an open surface'
);
assert.match(
  row,
  /function hideOverlay\(\): void \{[\s\S]*?cardView = null;/,
  'closing the card drops its snapshot'
);
for (const live of ['model={modelText}', 'statusTone={presence}', '{usage}', 'lastActivity={activity}']) {
  assert.ok(
    !row.includes(live),
    `the card must not take ${live} live while it is open`
  );
}
assert.match(row, /onmouseenter=\{showOverlay\}/, 'the action cluster opens on pointer hover');
assert.match(row, /onfocusin=\{showOverlay\}/, 'the action cluster opens on keyboard focus');
assert.match(row, /data-presence=\{presence\}/, 'presence stays a compact row state');
assert.match(row, /label="Resume session"/, 'stopped rows swap elapsed time for Resume');
assert.match(
  row,
  /const presenceIsRestart = \$derived\(presence === 'stopped'\)/,
  'only genuinely stopped rows offer Resume, so suspended idle rows do not'
);
assert.match(
  row,
  /\.row\[data-presence='stopped'\]:hover \.status,[\s\S]*?\.row\[data-presence='stopped'\]:hover \.resume-slot/,
  'elapsed time and Resume trade opacity inside the same stopped-row slot'
);
assert.match(
  row,
  /session\.pendingPermission === true[\s\S]*?session\.pendingInput === true/,
  'both permission and structured-input requests mark the row as needing the human'
);
assert.match(row, />\s*Needs you\s*</, 'the human-waiting state uses the approved concise label');
assert.match(
  rail,
  /data-testid="session-rail-needs-you-count"/,
  'group headings expose a needs-you count chip'
);
assert.match(
  rail,
  /`mcb\.rail\.order\.\$\{groupKey\}`/,
  'each group persists its owned-id order under the documented localStorage key'
);
assert.match(rail, /setDragImage\(/, 'dragging uses an explicit quiet drag image');
assert.match(
  rail,
  /dragState\.groupKey !== groupKey/,
  'drag-over accepts reordering only inside the source group'
);
assert.match(rail, /event\.key === 'Escape'/, 'Escape clears the active reorder state');
assert.match(
  row,
  /height:\s*4px;[\s\S]*?background:\s*var\(--color-accent\);/,
  'the insertion target is a 4px accent line'
);
assert.match(row, /const POPOUT_DIAG_DISABLED = false;/, 'the snapshot hover popout is enabled');
assert.match(row, /const TICKER_DIAG_DISABLED = true;/, 'the isolated ticker stays disabled');
assert.match(row, /Idle — resumes on send/, 'suspended rows explain that only a send resumes them');
assert.match(
  row,
  /\{suspended \? 'Suspended' : presenceLabel\}/,
  'a suspended row says so in words, with no dot and no motion'
);
assert.doesNotMatch(row, /No project recorded/i, 'the rail never renders the placeholder project sentence');

assert.match(
  shellPage,
  /registerSessionRowJumpTarget\(\{[\s\S]*?selectSession[\s\S]*?showCenterPanel[\s\S]*?showSidebarView[\s\S]*?\}\)/,
  'the page registers the one host that can activate a session and a surface'
);
assert.match(
  shellPage,
  /selectSession: async \(ownedId\) => \{[\s\S]*?rail\.activeOwnedId === ownedId[\s\S]*?await selectOwned\(ownedId\)/,
  'the page skips activation when the jumped-to session is already active'
);
assert.match(
  shellPage,
  /async function selectOwned\([\s\S]*?loadConversationForRead\(ownedId\)/,
  'a rail selection reads the stored transcript'
);
const selectionBlock = shellPage.match(
  /async function selectOwned\([\s\S]*?\n  \}\n\n  function handoffInput/
);
assert.ok(selectionBlock, 'the selection path stays explicit');
assert.doesNotMatch(selectionBlock[0], /ensureStructuredConversation\(/, 'selection never ensures or starts a runtime');

// --- the row's right-click roster ------------------------------------------

const workingMenu = sessionRowMenuItems({
  status: 'working',
  sessionId: 'sess-1',
  worktreePath: '/work/one'
});

assert.deepEqual(
  workingMenu.map((item) => item.id),
  [
    'rename',
    'move-to-project',
    'mark-done',
    'archive',
    'copy-session-id',
    'copy-worktree-path',
    'continue-in-new-session',
    'open-in-editor',
    'reveal-in-finder',
    'delete'
  ],
  'a working row offers the approved menu plus the move it can actually make'
);

assert.deepEqual(
  workingMenu.filter((item) => !item.enabled).map((item) => item.id),
  ['rename', 'move-to-project', 'continue-in-new-session', 'reveal-in-finder'],
  'items with no command behind them show but stay switched off'
);

for (const item of workingMenu) {
  if (item.enabled) continue;
  assert.ok(item.disabledReason, `${item.id} must say why it is off`);
}

assert.ok(
  sessionRowMenuItems({ status: 'done', sessionId: 's', worktreePath: '/w' })
    .some((item) => item.id === 'reopen'),
  'a finished row can go back to Working'
);
assert.ok(
  sessionRowMenuItems({ status: 'settled', sessionId: 's', worktreePath: '/w' })
    .some((item) => item.id === 'unsettle'),
  'a settled row can come back to Done'
);
assert.ok(
  sessionRowMenuItems({ status: 'settled', sessionId: 's', worktreePath: '/w' })
    .every((item) => item.id !== 'archive'),
  'a settled row is not offered Archive again'
);

const withoutPaths = sessionRowMenuItems({ status: 'working', sessionId: null, worktreePath: null });
for (const id of ['copy-session-id', 'copy-worktree-path'] as const) {
  const item = withoutPaths.find((candidate) => candidate.id === id);
  assert.equal(item?.enabled, false, `${id} is off when there is nothing to copy`);
}

assert.match(row, /import \* as ContextMenu/, 'the row menu is the kit context menu');
assert.match(
  row,
  /sessionRowMenuItems\(\{/,
  'the row draws the roster this module decides'
);

// --- the kit hover-action pattern ------------------------------------------

assert.match(
  hoverActions,
  /group-hover:opacity-100/,
  'the cluster is revealed by the row it sits in, with CSS alone'
);
assert.doesNotMatch(
  hoverActions,
  /bg-popover|bg-card|shadow|rounded/,
  'no pill, panel or shadow behind the cluster'
);
assert.match(
  hoverActions,
  /opacity-0 pointer-events-none/,
  'the buttons stay in the page at rest rather than being built on hover'
);
for (const tone of ['primary', 'info', 'success']) {
  assert.ok(
    hoverActionButton.includes(`${tone}:`),
    `a hover action can say it leads to the ${tone} kind of surface`
  );
}
assert.match(
  hoverActionButton,
  /import \{ IconButton \}/,
  'a hover action is still a kit icon button, so it keeps its label and tooltip'
);

// --- the scroll chain ------------------------------------------------------

assert.match(
  rail,
  /data-testid="session-rail" class="session-scroll"/,
  'Working, Done and Settled share one scroll area'
);
assert.match(
  rail,
  /\.session-scroll \{[^}]*overflow-y: scroll;[^}]*scrollbar-gutter: stable;/,
  'the scrollbar keeps its own gutter so it never covers a row'
);
assert.match(
  rail,
  /\.section-heading \{[^}]*position: sticky;/,
  'a section heading stays in view while its rows scroll under it'
);
assert.doesNotMatch(rail, /paneStack|createPaneStack|dv-pane-header/, 'the rail has no splitters');

// --- the one clock and the one spinner -------------------------------------

assert.match(
  row,
  /watchRailElapsed/,
  'elapsed text comes from the shared rail ticker, not a timer per row'
);
assert.doesNotMatch(row, /setInterval/, 'the row starts no interval of its own');
assert.match(
  row,
  /const spinning = \$derived\(isWorking && onScreen\)/,
  'the spinner needs a working row that is on screen'
);
assert.match(
  row,
  /\.spinner\.spinning \{ animation: spin 900ms linear infinite; \}/,
  'only the spinning class animates, so a row at rest has no motion'
);
assert.match(
  row,
  /railElapsedCadenceFor\(ageMs \?\? 0, isWorking\)/,
  'the row asks for the slow cadence once its age is past the seconds range'
);
assert.match(
  row,
  /if \(TICKER_DIAG_DISABLED \|\| !onScreen \|\| !hasAge\) return;/,
  'a row off screen holds no clock at all, and the ticker stays behind its off switch'
);

// --- motion: plenty of it, and all of it ends ------------------------------

// The one loop in the rail is the working spinner. Everything else is a
// transition or a one-shot keyframe, which is free once it has finished.
assert.deepEqual(
  (row.match(/infinite/g) ?? []).length,
  1,
  'the row has exactly one looping animation'
);
assert.match(
  row,
  /\.spinner\.spinning \{ animation: spin 900ms linear infinite; \}/,
  'and that loop is the working spinner'
);
for (const [name, source] of [
  ['the rail', rail],
  ['the hover-action cluster', hoverActions],
  ['the hover card', card]
] as const) {
  assert.ok(!source.includes('infinite'), `${name} has no animation that never ends`);
}

// Animating a layout property makes the browser re-lay-out every frame. These
// files may animate paint and compositor properties only.
const LAYOUT_PROPERTIES = /transition:\s*(width|height|top|left|right|bottom|margin|padding|inset)\b/;
for (const [name, source] of [
  ['the row', row],
  ['the rail', rail],
  ['the hover card', card]
] as const) {
  assert.doesNotMatch(source, LAYOUT_PROPERTIES, `${name} animates no layout property`);
}

// Interaction motion that makes the rail feel answered rather than static.
assert.match(
  row,
  /\.session-row \{ transition: background-color \d+ms/,
  'the row fill answers the pointer'
);
assert.match(
  row,
  /\.session-row::before \{ transition: transform \d+ms/,
  'the selection bar sweeps in rather than appearing'
);
assert.match(row, /@keyframes card-in/, 'the hover card fades in on open');
assert.match(
  rail,
  /\.section-heading\[aria-expanded='true'\] :global\(\.chevron\) \{ transform: rotate\(90deg\); \}/,
  'the section chevron turns to say the section answered'
);
assert.match(rail, /@keyframes section-in/, 'and its rows settle in when it opens');
assert.match(
  hoverActions,
  /motion-safe:transition-\[opacity,transform\]/,
  'the hover cluster fades and settles in'
);

// All of it steps aside for a reader who asked for less motion.
for (const [name, source] of [['the row', row], ['the rail', rail]] as const) {
  assert.match(
    source,
    /@media \(prefers-reduced-motion: no-preference\) \{/,
    `${name} only animates when motion is welcome`
  );
}

// --- the age and the provider mark ----------------------------------------

assert.match(
  row,
  /data-testid="worktree-agent-age"/,
  'the age has its own element, outside every status branch, so it never disappears'
);
assert.match(
  row,
  /session\.startedAtMs/,
  'the age counts from when the session started, not from the current turn'
);
assert.doesNotMatch(row, /class="model-chip"/, 'the model name has left the row');
assert.match(
  row,
  /const ProviderIcon = \$derived\(AGENT_ICONS\[session\.agent\]\)/,
  'the row marks the provider with the shared glyph'
);
assert.match(
  card,
  /data-testid="session-hover-card-agent"/,
  'the hover card is where the agent and its model are spelled out'
);

console.log('sessionRowActions: ok');
