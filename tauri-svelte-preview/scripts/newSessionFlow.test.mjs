/**
 * newSessionFlow.test.mjs — the rules behind "start a new session".
 *
 * Everything under test is pure: the list of things you can launch, the command
 * line the dialog shows you before it runs anything, what counts as a usable
 * folder, and how the list of known project folders is put together. No Svelte,
 * no browser, no backend.
 *
 * Run: node --experimental-strip-types scripts/newSessionFlow.test.mjs
 */
import assert from 'node:assert/strict';

import {
  LAUNCH_CATALOG,
  agentKindFor,
  buildCommandPreview,
  buildNewSessionRequest,
  createCustomRoot,
  launchOptionFor,
  mergeKnownRoots,
  normalizeRootPath,
  parseStoredCustomRoots,
  quoteForShell,
  resolveSessionTitle,
  serializeCustomRoots,
  suggestSessionTitle,
  suggestedWorktreePath,
  validateNewSession,
  worktreeAddCommand,
  worktreeChoicesFor
} from '../src/lib/shell/newSession/newSessionFlow.ts';

// ── The launch catalog ────────────────────────────────────────────────────────

// Three ways to start: the two agents on this machine, and a plain terminal.
{
  assert.deepEqual(
    LAUNCH_CATALOG.map((option) => option.agent),
    ['claude', 'codex', 'shell']
  );

  for (const option of LAUNCH_CATALOG) {
    assert.ok(option.label.length > 0, `${option.agent} has a name to show`);
    assert.ok(option.hint.length > 0, `${option.agent} explains what it does`);
    assert.equal(typeof option.command, 'string');
    assert.ok(!option.command.includes('\n'), `${option.agent} launches with one line`);
  }
}

// The commands are the bare interactive launches, confirmed against the two
// tools' own --help: `claude` starts an interactive session by default, and
// `codex` with no subcommand forwards to the interactive CLI. No flags.
{
  assert.equal(launchOptionFor('claude').command, 'claude');
  assert.equal(launchOptionFor('codex').command, 'codex');
  // A plain terminal runs nothing at all — the shell IS the session.
  assert.equal(launchOptionFor('shell').command, '');
  assert.equal(launchOptionFor('nonsense'), null);
}

// The rail's own vocabulary for who is running: a plain terminal is not an agent.
{
  assert.equal(agentKindFor('claude'), 'claude');
  assert.equal(agentKindFor('codex'), 'codex');
  assert.equal(agentKindFor('shell'), 'other');
}

// ── The command preview ───────────────────────────────────────────────────────

// What the preview says is what will happen: the terminal opens in the folder,
// then the command runs.
{
  assert.equal(
    buildCommandPreview({ cwd: '/Users/me/dev/work/thing', command: 'claude' }),
    'cd /Users/me/dev/work/thing && claude'
  );
  // A plain terminal has nothing to run, so the preview stops at the folder.
  assert.equal(
    buildCommandPreview({ cwd: '/Users/me/dev/work/thing', command: '' }),
    'cd /Users/me/dev/work/thing'
  );
  // Whatever the user typed into the editable box is what gets previewed.
  assert.equal(
    buildCommandPreview({ cwd: '/Users/me/thing', command: 'claude --continue' }),
    'cd /Users/me/thing && claude --continue'
  );
  // Nothing to say at all without a folder.
  assert.equal(buildCommandPreview({ cwd: '   ', command: 'claude' }), '');
}

// Folders with awkward characters are quoted so the preview is copy-pasteable.
{
  assert.equal(quoteForShell('/Users/me/dev/work'), '/Users/me/dev/work');
  assert.equal(quoteForShell('/Users/me/My Projects'), "'/Users/me/My Projects'");
  assert.equal(quoteForShell("/Users/me/it's"), "'/Users/me/it'\\''s'");
  assert.equal(
    buildCommandPreview({ cwd: '/Users/me/My Projects/app', command: 'codex' }),
    "cd '/Users/me/My Projects/app' && codex"
  );
}

// ── Validation ────────────────────────────────────────────────────────────────

// A ready draft has nothing to complain about.
{
  const problems = validateNewSession({
    cwd: '/Users/me/dev/work/thing',
    command: 'claude',
    agent: 'claude',
    title: 'Claude in thing'
  });
  assert.deepEqual(problems, []);
}

// No folder, or a folder that is not a full path, stops the Start button.
{
  const noFolder = validateNewSession({ cwd: '', command: 'claude', agent: 'claude', title: '' });
  assert.equal(noFolder.length, 1);
  assert.equal(noFolder[0].field, 'cwd');
  assert.ok(noFolder[0].message.length > 0);

  const relative = validateNewSession({
    cwd: 'dev/work/thing',
    command: 'claude',
    agent: 'claude',
    title: ''
  });
  assert.equal(relative.length, 1);
  assert.equal(relative[0].field, 'cwd');
  assert.ok(/full path/i.test(relative[0].message), relative[0].message);

  // Surrounding spaces are trimmed rather than complained about.
  assert.deepEqual(
    validateNewSession({ cwd: '  /Users/me/thing  ', command: '', agent: 'shell', title: '' }),
    []
  );
}

// The command is typed into the terminal as one line. Two lines would run two
// things, so it is refused rather than silently split.
{
  const problems = validateNewSession({
    cwd: '/Users/me/thing',
    command: 'claude\nrm -rf /',
    agent: 'claude',
    title: ''
  });
  assert.equal(problems.length, 1);
  assert.equal(problems[0].field, 'command');
}

// An emptied command box is allowed — it means "just give me a terminal".
{
  assert.deepEqual(
    validateNewSession({ cwd: '/Users/me/thing', command: '', agent: 'claude', title: '' }),
    []
  );
}

// ── Titles ────────────────────────────────────────────────────────────────────

{
  assert.equal(suggestSessionTitle({ cwd: '/Users/me/dev/work/mac-command-bar', agent: 'claude' }), 'Claude in mac-command-bar');
  assert.equal(suggestSessionTitle({ cwd: '/Users/me/dev/work/mac-command-bar/', agent: 'codex' }), 'Codex in mac-command-bar');
  assert.equal(suggestSessionTitle({ cwd: '/Users/me/thing', agent: 'shell' }), 'Terminal in thing');
  assert.equal(suggestSessionTitle({ cwd: '', agent: 'shell' }), 'Terminal');

  // What the user typed wins; a blank box falls back to the suggestion.
  assert.equal(
    resolveSessionTitle({ cwd: '/Users/me/thing', agent: 'claude', title: '  My run  ' }),
    'My run'
  );
  assert.equal(
    resolveSessionTitle({ cwd: '/Users/me/thing', agent: 'claude', title: '   ' }),
    'Claude in thing'
  );
}

// ── Known project folders ─────────────────────────────────────────────────────

{
  assert.equal(normalizeRootPath('  /Users/me/thing/  '), '/Users/me/thing');
  assert.equal(normalizeRootPath('/'), '/');
  assert.equal(normalizeRootPath('   '), '');
}

// The list is the built-in folders, then the ones the user added, then the
// folders the sessions on the rail are already running in.
{
  const merged = mergeKnownRoots({
    defaults: [{ id: 'ediplatform', name: 'EdiPlatform', path: '/Users/me/dev/work/EdiPlatform' }],
    custom: [{ id: 'custom:/Users/me/side', name: 'side', path: '/Users/me/side' }],
    sessionPaths: ['/Users/me/dev/work/other']
  });
  assert.deepEqual(
    merged.map((root) => [root.path, root.source]),
    [
      ['/Users/me/dev/work/EdiPlatform', 'default'],
      ['/Users/me/side', 'custom'],
      ['/Users/me/dev/work/other', 'session']
    ]
  );
  assert.equal(merged[2].name, 'other', 'a folder from a session is named after itself');
}

// One folder, one row — no matter how many places it came from or how it was
// spelled, and no matter how many sessions share it.
{
  const merged = mergeKnownRoots({
    defaults: [{ id: 'thing', name: 'Thing', path: '/Users/me/thing' }],
    custom: [{ id: 'custom:/Users/me/thing', name: 'thing again', path: '/Users/me/thing/' }],
    sessionPaths: ['/Users/me/thing', '/Users/me/thing/', '/Users/me/second']
  });
  assert.deepEqual(
    merged.map((root) => root.path),
    ['/Users/me/thing', '/Users/me/second']
  );
  assert.equal(merged[0].name, 'Thing', 'the first spelling of a folder is the one kept');
}

// Nonsense never reaches the picker: blanks and half-written relative paths.
{
  const merged = mergeKnownRoots({
    defaults: [
      { id: 'ok', name: 'Ok', path: '/Users/me/ok' },
      { id: 'blank', name: 'Blank', path: '   ' },
      { id: 'relative', name: 'Relative', path: 'dev/work/thing' }
    ],
    custom: [],
    sessionPaths: ['', '   ', 'also/relative']
  });
  assert.deepEqual(
    merged.map((root) => root.path),
    ['/Users/me/ok']
  );
}

// Session folders arrive in whatever order the rail happens to hold them, and
// come out in a steady alphabetical order so the list does not jump about.
{
  const merged = mergeKnownRoots({
    defaults: [],
    custom: [],
    sessionPaths: ['/Users/me/zeta', '/Users/me/alpha', '/Users/me/Middle']
  });
  assert.deepEqual(
    merged.map((root) => root.name),
    ['alpha', 'Middle', 'zeta']
  );
}

// A folder the user adds by hand keeps a stable id, so re-adding it does not
// make a second row.
{
  const first = createCustomRoot('/Users/me/dev/work/thing/');
  const second = createCustomRoot('  /Users/me/dev/work/thing  ');
  assert.equal(first.path, '/Users/me/dev/work/thing');
  assert.equal(first.name, 'thing');
  assert.equal(first.id, second.id);
  assert.equal(createCustomRoot('   '), null);
  assert.equal(createCustomRoot('relative/path'), null);
}

// ── Remembering the folders the user added ────────────────────────────────────

// Round trip.
{
  const roots = [createCustomRoot('/Users/me/one'), createCustomRoot('/Users/me/two')];
  assert.deepEqual(parseStoredCustomRoots(serializeCustomRoots(roots)), roots);
}

// Anything unreadable reads back as "no folders added yet" instead of throwing.
{
  assert.deepEqual(parseStoredCustomRoots(null), []);
  assert.deepEqual(parseStoredCustomRoots(''), []);
  assert.deepEqual(parseStoredCustomRoots('not json'), []);
  assert.deepEqual(parseStoredCustomRoots('{"path":"/Users/me/one"}'), []);
  assert.deepEqual(parseStoredCustomRoots('[1, null, "x"]'), []);
  // Entries missing a usable path are dropped; the good ones survive.
  assert.deepEqual(
    parseStoredCustomRoots(
      JSON.stringify([
        { id: 'a', name: 'a', path: '' },
        { id: 'b', name: 'b', path: 'relative' },
        { id: 'c', name: 'c', path: '/Users/me/good/' }
      ])
    ),
    [{ id: 'custom:/Users/me/good', name: 'c', path: '/Users/me/good' }]
  );
  // A stored entry with no name is named after its folder.
  assert.deepEqual(parseStoredCustomRoots(JSON.stringify([{ path: '/Users/me/good' }])), [
    { id: 'custom:/Users/me/good', name: 'good', path: '/Users/me/good' }
  ]);
}

// ── Which checkout to start in ────────────────────────────────────────────────

// The project's own checkout is always offered, and always first — even when
// the desktop app could not list worktrees at all (the web build, say).
{
  const choices = worktreeChoicesFor({ projectRoot: '/Users/me/thing', worktrees: null });
  assert.equal(choices.length, 1);
  assert.equal(choices[0].path, '/Users/me/thing');
  assert.equal(choices[0].isPrimary, true);
  assert.ok(choices[0].label.length > 0);
}

// With worktrees listed, the main checkout still leads and the rest follow in
// the order git gave them. The main checkout is not listed twice.
{
  const choices = worktreeChoicesFor({
    projectRoot: '/Users/me/thing',
    worktrees: [
      {
        repo: 'thing',
        path: '/Users/me/worktrees/thing/feature',
        branch: 'tsk-1-feature',
        taskID: 'tsk-1',
        isDirty: true,
        hasUnmergedCommits: false,
        lastActivity: null,
        deleteEligibility: 'safe'
      },
      {
        repo: 'thing',
        path: '/Users/me/thing',
        branch: 'main',
        taskID: null,
        isDirty: false,
        hasUnmergedCommits: false,
        lastActivity: null,
        deleteEligibility: 'blocked'
      }
    ]
  });
  assert.deepEqual(
    choices.map((choice) => choice.path),
    ['/Users/me/thing', '/Users/me/worktrees/thing/feature']
  );
  assert.equal(choices[0].isPrimary, true);
  assert.equal(choices[0].branch, 'main', 'the branch git reported for the main checkout');
  assert.equal(choices[1].isPrimary, false);
  assert.equal(choices[1].branch, 'tsk-1-feature');
  assert.ok(/uncommitted/i.test(choices[1].note ?? ''), 'a dirty worktree says so');
  assert.equal(choices[0].note, null);
}

// ── Making a worktree is never something this dialog does ─────────────────────

// It hands over the command to run instead, pointed at the folder the machine
// keeps worktrees in: alongside the repo, never inside it.
{
  assert.equal(
    suggestedWorktreePath('/Users/me/dev/work/mac-command-bar', 'tsk-9-thing'),
    '/Users/me/dev/work/worktrees/mac-command-bar/tsk-9-thing'
  );
  assert.equal(
    worktreeAddCommand('/Users/me/dev/work/mac-command-bar', 'tsk-9-thing'),
    'git -C /Users/me/dev/work/mac-command-bar worktree add ' +
      '/Users/me/dev/work/worktrees/mac-command-bar/tsk-9-thing -b tsk-9-thing'
  );
  // An empty branch box still shows a command that reads sensibly.
  assert.equal(
    worktreeAddCommand('/Users/me/dev/work/thing', '  '),
    'git -C /Users/me/dev/work/thing worktree add ' +
      '/Users/me/dev/work/worktrees/thing/new-branch -b new-branch'
  );
  // A name with spaces becomes a branch name git will actually take.
  assert.equal(
    worktreeAddCommand('/Users/me/dev/work/thing', 'My New Branch'),
    'git -C /Users/me/dev/work/thing worktree add ' +
      '/Users/me/dev/work/worktrees/thing/my-new-branch -b my-new-branch'
  );
  assert.equal(worktreeAddCommand('', 'x'), '');
}

// ── The handover ──────────────────────────────────────────────────────────────

// What the dialog passes to whoever owns the session list: the folder to open a
// terminal in, the name for its row, which agent is running, and the line to
// type. Nothing else, and nothing half-finished.
{
  assert.deepEqual(
    buildNewSessionRequest({
      cwd: '  /Users/me/dev/work/thing/  ',
      command: '  claude  ',
      agent: 'claude',
      title: '  '
    }),
    {
      cwd: '/Users/me/dev/work/thing',
      title: 'Claude in thing',
      agent: 'claude',
      command: 'claude'
    }
  );

  // A plain terminal has no command to type. `null` is the word the session
  // record already uses for that, so it is the word used here.
  assert.deepEqual(
    buildNewSessionRequest({ cwd: '/Users/me/thing', command: '', agent: 'shell', title: 'Poke' }),
    { cwd: '/Users/me/thing', title: 'Poke', agent: 'other', command: null }
  );

  // A draft that would not pass validation never becomes a request.
  assert.equal(
    buildNewSessionRequest({ cwd: '', command: 'claude', agent: 'claude', title: '' }),
    null
  );
  assert.equal(
    buildNewSessionRequest({
      cwd: '/Users/me/thing',
      command: 'claude\nrm -rf /',
      agent: 'claude',
      title: ''
    }),
    null
  );
}

console.log('newSessionFlow.test.mjs: all assertions passed');
