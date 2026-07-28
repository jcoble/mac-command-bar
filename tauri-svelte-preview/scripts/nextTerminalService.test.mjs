import assert from 'node:assert/strict';
import {
  createTerminalService,
  isTerminalInputCommand,
  tauriTerminalBackend
} from '../src/lib/shell/terminalService.ts';

/** Let a scheduled one-shot (and the microtasks it queues) actually run. */
const settle = () => new Promise((resolve) => setTimeout(resolve, 30));

/**
 * Spy TerminalView — every method appends to the shared `log` so a test can
 * assert on the exact IO the service performed. No xterm, no DOM.
 */
function makeView(log, name, opts = {}) {
  // `cols`/`rows` mirror a real xterm grid: `resize` sets it outright (works
  // while hidden), `fit` adopts whatever the host measures — which the test
  // supplies as `fitTo`, defaulting to "the grid never changes".
  const view = {
    cols: 80,
    rows: 24,
    write: (d) => log.push([name, 'write', d]),
    fit: () => {
      const fitTo = opts.fitTo ?? null;
      if (fitTo) {
        view.cols = fitTo.cols;
        view.rows = fitTo.rows;
      }
      log.push([name, 'fit', view.cols, view.rows]);
      opts.hooks?.onResize(view.cols, view.rows);
    },
    focus: () => log.push([name, 'focus']),
    setVisible: (v) => log.push([name, 'visible', v]),
    dispose: () => log.push([name, 'dispose'])
  };
  if (opts.noResize !== true) {
    view.resize = (cols, rows) => {
      view.cols = cols;
      view.rows = rows;
      log.push([name, 'resize', cols, rows]);
    };
  }
  return view;
}

/**
 * Fake TerminalBackend: records every call and hands back canned values.
 * `emit` drives the single output listener the service registers in attach().
 */
function makeBackend(log) {
  let listener = null;
  // Each start mints a FRESH id (pty-1, pty-2, ...) so a test can hold several
  // sessions at once without them colliding in the service's 1:1 map.
  let minted = 0;
  // When set, backend.close REJECTS with it (a dead IPC channel, a PTY the OS
  // already reaped, ...) — the case that used to lose the successor.
  let closeError = null;
  // What readScrollback hands back. Overridable so a test can exercise the
  // replay-tail cap with a scrollback bigger than the view could ever hold.
  let scrollback = 'OLD OUTPUT';
  return {
    backend: {
      start: async (req) => {
        log.push(['start', req]);
        minted += 1;
        return {
          sessionId: `pty-${minted}`,
          cwd: req.cwd,
          shell: '/bin/zsh',
          cols: 96,
          rows: 28,
          pid: 1,
          startedAt: 0,
          exited: false,
          exitCode: null,
          signal: null
        };
      },
      write: async (id, d) => {
        log.push(['write', id, d]);
        return true;
      },
      resize: async (id, cols, rows) => {
        log.push(['resize', id, cols, rows]);
        return true;
      },
      close: async (id) => {
        log.push(['close', id]);
        if (closeError) throw closeError;
        return true;
      },
      readScrollback: async (id) => {
        log.push(['readScrollback', id]);
        return scrollback;
      },
      list: async () => [],
      listen: async (h) => {
        log.push(['listen']);
        listener = h;
        return () => {
          log.push(['unlisten']);
          listener = null;
        };
      }
    },
    emit: (p) => listener?.(p),
    listening: () => listener !== null,
    failClose: (error) => {
      closeError = error;
    },
    setScrollback: (value) => {
      scrollback = value;
    }
  };
}

const ownedA = {
  ownedId: 'a',
  agent: 'claude',
  viaCmux: false,
  source: 'scanned',
  title: 't',
  projectPath: '/p',
  cwd: '/p',
  resumeCommand: 'claude --resume n1',
  nativeSessionId: 'n1',
  ptySessionId: null,
  state: 'background'
};

{
  // 1 + 2: start ensures a view, binds the session, replays the resume command;
  // output routes to the (hidden) view; a terminated payload fires onExit.
  const log = [];
  const { backend, emit } = makeBackend(log);
  const exits = [];
  const svc = createTerminalService({
    backend,
    createView: () => makeView(log, 'viewA'),
    onExit: (ownedId, payload) => exits.push([ownedId, payload.exitCode])
  });
  await svc.attach();
  const pty = await svc.startOwned(ownedA, {});
  assert.equal(pty, 'pty-1');
  assert.deepEqual(
    log.find((e) => e[0] === 'start')[1],
    { cwd: '/p', ownedId: 'a' },
    'start request carries cwd + ownedId only'
  );
  assert.deepEqual(
    log.find((e) => e[0] === 'write'),
    ['write', 'pty-1', 'claude --resume n1\r'],
    'resume command is written to the freshly bound PTY'
  );
  emit({ sessionId: 'pty-1', data: 'hello', terminated: false, exitCode: null, signal: null });
  assert.ok(
    log.some((e) => e[0] === 'viewA' && e[1] === 'write' && e[2] === 'hello'),
    'output routes to the view via feedSession'
  );
  emit({ sessionId: 'pty-1', data: '', terminated: true, exitCode: 0, signal: null });
  assert.deepEqual(exits, [['a', 0]], 'terminated payload reports the exit');
  assert.equal(log.filter((e) => e[0] === 'listen').length, 1, 'exactly one backend listener');
}

{
  // 1: attach() is idempotent — a second call must NOT register a second listener.
  const log = [];
  const { backend, emit } = makeBackend(log);
  const svc = createTerminalService({ backend, createView: () => makeView(log, 'viewIdem') });
  await svc.attach();
  await svc.attach();
  assert.equal(log.filter((e) => e[0] === 'listen').length, 1, 'attach registers ONE listener');
  await svc.startOwned({ ...ownedA, ownedId: 'i', resumeCommand: null }, {});
  emit({ sessionId: 'pty-1', data: 'x', terminated: false, exitCode: null, signal: null });
  assert.equal(
    log.filter((e) => e[0] === 'viewIdem' && e[1] === 'write' && e[2] === 'x').length,
    1,
    'output is written exactly once'
  );
}

{
  // 3 + 6: adoptExisting hydrates scrollback; dispose never closes the PTY.
  const log = [];
  const { backend, listening } = makeBackend(log);
  const svc = createTerminalService({ backend, createView: () => makeView(log, 'viewB') });
  await svc.attach();
  const ok = await svc.adoptExisting({ ...ownedA, ownedId: 'b', ptySessionId: 'pty-9' }, {});
  assert.equal(ok, true);
  assert.ok(
    log.some((e) => e[0] === 'viewB' && e[1] === 'write' && e[2] === 'OLD OUTPUT'),
    'the adopted view is hydrated from backend scrollback'
  );
  svc.dispose();
  assert.ok(!log.some((e) => e[0] === 'close'), 'dispose must not close PTYs');
  assert.ok(log.some((e) => e[0] === 'viewB' && e[1] === 'dispose'), 'dispose tears down views');
  assert.equal(listening(), false, 'dispose unlistens');
}

{
  // 3: adoptExisting without a ptySessionId creates nothing.
  const log = [];
  const { backend } = makeBackend(log);
  const svc = createTerminalService({ backend, createView: () => makeView(log, 'viewNone') });
  await svc.attach();
  const ok = await svc.adoptExisting({ ...ownedA, ownedId: 'z', ptySessionId: null }, {});
  assert.equal(ok, false);
  assert.ok(!log.some((e) => e[0] === 'viewNone'), 'no view is created without a PTY id');
  assert.ok(!log.some((e) => e[0] === 'readScrollback'), 'no scrollback read without a PTY id');
}

{
  // 4: the view created for owned X writes/resizes X's PTY.
  const log = [];
  const { backend } = makeBackend(log);
  let hooks = null;
  const svc = createTerminalService({
    backend,
    createView: (_host, viewHooks) => {
      hooks = viewHooks;
      return makeView(log, 'viewD');
    }
  });
  await svc.attach();
  await svc.startOwned({ ...ownedA, ownedId: 'd', resumeCommand: null }, {});
  hooks.onData('ls\r');
  hooks.onResize(120, 40);
  await Promise.resolve();
  assert.ok(
    log.some((e) => e[0] === 'write' && e[1] === 'pty-1' && e[2] === 'ls\r'),
    'view input reaches the owned session PTY'
  );
  assert.ok(
    log.some((e) => e[0] === 'resize' && e[1] === 'pty-1' && e[2] === 120 && e[3] === 40),
    'view resize reaches the owned session PTY'
  );
}

{
  // 5: closeOwned is the only close path.
  const log = [];
  const { backend } = makeBackend(log);
  const svc = createTerminalService({ backend, createView: () => makeView(log, 'viewC') });
  await svc.attach();
  await svc.startOwned({ ...ownedA, ownedId: 'c', resumeCommand: null }, {});
  await svc.closeOwned('c');
  assert.ok(log.some((e) => e[0] === 'close' && e[1] === 'pty-1'), 'closeOwned kills the PTY');
  assert.ok(log.some((e) => e[0] === 'viewC' && e[1] === 'dispose'), 'closeOwned drops the view');
  // The ownedId -> ptyId entry is gone: a second close is a no-op on the backend.
  await svc.closeOwned('c');
  assert.equal(log.filter((e) => e[0] === 'close').length, 1, 'stale map entries are cleaned');
}

{
  // C1: the manager auto-shows only the FIRST view and only hides views that
  // already exist, so a view created while another session is active would
  // never be told to hide — and its `inset: 0` host would cover the active
  // terminal. The service makes the hide explicit; assert it here.
  const log = [];
  const { backend } = makeBackend(log);
  let created = 0;
  const svc = createTerminalService({
    backend,
    createView: () => makeView(log, `v${(created += 1)}`)
  });
  await svc.attach();
  await svc.startOwned({ ...ownedA, ownedId: 'a', resumeCommand: null }, {});
  assert.ok(
    log.some((e) => e[0] === 'v1' && e[1] === 'visible' && e[2] === true),
    'the first view is shown'
  );

  await svc.startOwned({ ...ownedA, ownedId: 'b', resumeCommand: null }, {});
  assert.deepEqual(
    log.filter((e) => e[0] === 'v2' && e[1] === 'visible'),
    [['v2', 'visible', false]],
    'a view created while another is active is hidden the moment it exists'
  );
  assert.ok(
    !log.some((e) => e[0] === 'v1' && e[1] === 'visible' && e[2] === false),
    'and the active view is left alone'
  );

  await svc.adoptExisting({ ...ownedA, ownedId: 'c', ptySessionId: 'pty-9' }, {});
  assert.deepEqual(
    log.filter((e) => e[0] === 'v3' && e[1] === 'visible'),
    [['v3', 'visible', false]],
    'an adopted view gets the same treatment'
  );
}

{
  // I5: a TOMBSTONE (exited, but the backend still holds the record) adopts
  // like any survivor — its final scrollback renders — and dismissing it reaps
  // the backend session instead of leaking it.
  const log = [];
  const { backend } = makeBackend(log);
  const svc = createTerminalService({ backend, createView: () => makeView(log, 'tomb') });
  await svc.attach();
  const ok = await svc.adoptExisting(
    { ...ownedA, ownedId: 't', ptySessionId: 'pty-dead', state: 'exited' },
    {}
  );
  assert.equal(ok, true, 'a tombstone re-attaches');
  assert.ok(
    log.some((e) => e[0] === 'tomb' && e[1] === 'write' && e[2] === 'OLD OUTPUT'),
    'the tombstone view is hydrated from the surviving scrollback'
  );
  const closed = await svc.closeOwned('t');
  assert.ok(
    log.some((e) => e[0] === 'close' && e[1] === 'pty-dead'),
    'dismissing a tombstone closes the backend session'
  );
  assert.equal(closed.successor, null, 'nothing is left to show');
  assert.equal(closed.error, null, 'a clean close reports no error');
}

{
  // I5b: dismissing a session whose view was never built still reaps its
  // backend record, using the caller's stored ptySessionId as a hint — but a
  // hint that belongs to a DIFFERENT owned session is ignored.
  const log = [];
  const { backend } = makeBackend(log);
  const svc = createTerminalService({ backend, createView: () => makeView(log, 'viewH') });
  await svc.attach();
  await svc.closeOwned('ghost', 'pty-ghost');
  assert.ok(
    log.some((e) => e[0] === 'close' && e[1] === 'pty-ghost'),
    'the hint closes a PTY the service never mapped'
  );
  await svc.startOwned({ ...ownedA, ownedId: 'owner', resumeCommand: null }, {});
  await svc.closeOwned('impostor', 'pty-1');
  assert.equal(
    log.filter((e) => e[0] === 'close' && e[1] === 'pty-1').length,
    0,
    "a hint pointing at another session's live PTY is refused"
  );
}

{
  // I6: closeOwned reports the successor the MANAGER chose and showed (the
  // most-recently-inserted survivor), so the caller cannot pick a different one
  // and end up showing/fitting/focusing a second terminal per close.
  const log = [];
  const { backend } = makeBackend(log);
  let created = 0;
  const svc = createTerminalService({
    backend,
    createView: () => makeView(log, `s${(created += 1)}`)
  });
  await svc.attach();
  for (const ownedId of ['a', 'b', 'c']) {
    await svc.startOwned({ ...ownedA, ownedId, resumeCommand: null }, {});
  }
  svc.show('a');
  const first = await svc.closeOwned('a');
  assert.equal(first.successor, 'c', 'the successor is the most-recently-inserted survivor');
  assert.ok(
    log.some((e) => e[0] === 's3' && e[1] === 'visible' && e[2] === true),
    'and it is the view the manager actually showed'
  );
  assert.equal((await svc.closeOwned('c')).successor, 'b', 'the next close reports the next one');
  assert.equal((await svc.closeOwned('b')).successor, null, 'no views left = no successor');
}

{
  // R1: a REJECTING backend close must still report the successor. Losing it to
  // a throw left the caller with `null`, i.e. the opaque empty-state overlay
  // painted over the terminal the manager had just shown.
  const log = [];
  const { backend, failClose } = makeBackend(log);
  let created = 0;
  const svc = createTerminalService({
    backend,
    createView: () => makeView(log, `f${(created += 1)}`)
  });
  await svc.attach();
  for (const ownedId of ['a', 'b']) {
    await svc.startOwned({ ...ownedA, ownedId, resumeCommand: null }, {});
  }
  svc.show('a');
  const boom = new Error('ipc channel closed');
  failClose(boom);
  const result = await svc.closeOwned('a');
  assert.equal(result.successor, 'b', 'a failed close still reports the surviving view');
  assert.equal(result.error, boom, 'and hands the rejection back instead of throwing it');
  assert.ok(
    log.some((e) => e[0] === 'f2' && e[1] === 'visible' && e[2] === true),
    'the survivor is the view the manager actually showed'
  );
  assert.ok(
    log.some((e) => e[0] === 'f1' && e[1] === 'dispose'),
    'the closed view is dropped even though the backend rejected'
  );
  // The mapping went with it: a second dismiss must not re-issue the close.
  const before = log.filter((e) => e[0] === 'close').length;
  const again = await svc.closeOwned('a');
  assert.equal(log.filter((e) => e[0] === 'close').length, before, 'no retry, no double-kill');
  assert.equal(again.error, null, 'and a no-op close reports no error');
}

{
  // F1: a PTY whose VIEW could not be built must still report its exit.
  // `setPty` runs before `ensureViewFor`, so the service knows the owner even
  // when the manager never got a record — resolving the exit only through the
  // manager left the rail row "live" over a dead process, forever.
  const log = [];
  const { backend, emit } = makeBackend(log);
  const exits = [];
  const svc = createTerminalService({
    backend,
    createView: () => {
      throw new Error('xterm failed to open');
    },
    onExit: (ownedId, payload) => exits.push([ownedId, payload.exitCode])
  });
  await svc.attach();
  await assert.rejects(
    () => svc.startOwned({ ...ownedA, ownedId: 'orphan', resumeCommand: null }, {}),
    /xterm failed to open/,
    'a view that cannot be built still surfaces the failure to the caller'
  );
  emit({ sessionId: 'pty-1', data: '', terminated: true, exitCode: 3, signal: null });
  assert.deepEqual(exits, [['orphan', 3]], 'the exit is reported from the service PTY map');

  // ...and once the session is closed, its PTY is nobody's business again.
  await svc.closeOwned('orphan');
  emit({ sessionId: 'pty-1', data: '', terminated: true, exitCode: 3, signal: null });
  assert.equal(exits.length, 1, 'a closed session does not report a late exit');
}

{
  // F2: switching between two SAME-SIZED views must cost zero backend calls.
  // `showView` fits on every switch and the fit reports geometry every time, so
  // an ungated hook spent one `resize_terminal_session` per switch forever.
  const log = [];
  const { backend } = makeBackend(log);
  const views = new Map();
  // Every host measures the SAME pane — the steady state a user switching back
  // and forth actually lives in. Mutating it later simulates a window resize.
  const pane = { cols: 120, rows: 40 };
  const svc = createTerminalService({
    backend,
    createView: (_host, hooks) => {
      const name = `g${views.size + 1}`;
      const view = makeView(log, name, { hooks, fitTo: pane });
      views.set(name, view);
      return view;
    }
  });
  await svc.attach();
  await svc.startOwned({ ...ownedA, ownedId: 'a', resumeCommand: null }, {});
  await svc.startOwned({ ...ownedA, ownedId: 'b', resumeCommand: null }, {});
  svc.show('a');
  svc.show('b');
  const settled = log.filter((e) => e[0] === 'resize').length;
  // Each PTY opened at 96x28 and the pane is 120x40, so exactly one resize per
  // session is legitimate — after that the size is known.
  assert.equal(settled, 2, 'one resize per session while its size actually changes');

  const before = log.length;
  svc.show('a');
  svc.show('b');
  svc.show('a');
  svc.show('b');
  assert.equal(
    log.slice(before).filter((e) => e[0] === 'resize').length,
    0,
    'steady-state switching between same-sized views issues NO backend resize'
  );
  assert.ok(
    log.slice(before).some((e) => e[1] === 'fit'),
    'the views are still fitted — the gate is on the backend call, not on fit'
  );
  assert.equal(
    log.slice(before).filter((e) => e[0] === 'write' || e[0] === 'close' || e[0] === 'start')
      .length,
    0,
    'and a switch performs no other backend IO'
  );

  // A pane that GENUINELY changes size still reaches the backend.
  pane.cols = 100;
  pane.rows = 30;
  const beforeGrow = log.length;
  svc.show('a');
  assert.deepEqual(
    log.slice(beforeGrow).filter((e) => e[0] === 'resize'),
    [['resize', 'pty-1', 100, 30]],
    'a real geometry change is not swallowed by the gatekeeper'
  );
}

{
  // F3: a survivor re-attached into a HIDDEN host must be sized to the PTY's
  // real grid BEFORE its scrollback is written, or the replay wraps at 80 cols.
  const log = [];
  const { backend } = makeBackend(log);
  const svc = createTerminalService({
    backend,
    createView: (_host, hooks) => makeView(log, 'hidden', { hooks })
  });
  await svc.attach();
  await svc.adoptExisting(
    { ...ownedA, ownedId: 'r', ptySessionId: 'pty-live' },
    {},
    { cols: 146, rows: 46 }
  );
  const order = log.filter((e) => e[0] === 'hidden').map((e) => e[1]);
  assert.deepEqual(
    order.slice(0, 2),
    ['resize', 'write'],
    'the view is sized BEFORE the hydrating write — order is the whole fix'
  );
  assert.deepEqual(
    log.find((e) => e[0] === 'hidden' && e[1] === 'resize'),
    ['hidden', 'resize', 146, 46],
    'and it is sized to the PTY geometry the backend reported'
  );

  // The PTY is already that size, so showing it later costs nothing.
  const before = log.length;
  svc.show('r');
  assert.equal(
    log.slice(before).filter((e) => e[0] === 'resize').length,
    0,
    'the adopted size seeds the gatekeeper: the first show is free'
  );
}

{
  // F3b: no size, or a view with no `resize` method, must still work — the
  // TerminalView member is OPTIONAL, so neither may throw.
  const log = [];
  const { backend } = makeBackend(log);
  const svc = createTerminalService({
    backend,
    createView: () => makeView(log, 'plain', { noResize: true })
  });
  await svc.attach();
  const ok = await svc.adoptExisting(
    { ...ownedA, ownedId: 'p', ptySessionId: 'pty-live' },
    {},
    { cols: 146, rows: 46 }
  );
  assert.equal(ok, true, 'a view without resize() still adopts');
  assert.ok(
    log.some((e) => e[0] === 'plain' && e[1] === 'write' && e[2] === 'OLD OUTPUT'),
    'and is still hydrated'
  );

  const log2 = [];
  const second = makeBackend(log2);
  const svc2 = createTerminalService({
    backend: second.backend,
    createView: () => makeView(log2, 'nosize')
  });
  await svc2.attach();
  await svc2.adoptExisting({ ...ownedA, ownedId: 'q', ptySessionId: 'pty-live' }, {});
  assert.ok(
    !log2.some((e) => e[0] === 'nosize' && e[1] === 'resize'),
    'no size supplied = no forced resize'
  );
}

{
  // F4: the dev HUD's headline must EXCLUDE interactive PTY input. `claude`
  // enables DECSET 1004 focus reporting, so xterm legitimately writes
  // `\x1b[I` / `\x1b[O` on every focus change — one `write_terminal_session` per
  // session switch, forever, exactly like a keystroke. Counting those as
  // backend chatter made the HUD read "+1 invoke per switch" and look like a
  // storm. Typing does the same thing.
  assert.equal(
    isTerminalInputCommand('write_terminal_session'),
    true,
    'a PTY write is interactive input, not shell overhead'
  );
  for (const command of [
    'start_terminal_session',
    'resize_terminal_session',
    'close_terminal_session',
    'read_terminal_session_scrollback',
    'list_terminal_sessions',
    'listen:terminal_output',
    'list_agent_sessions',
    'bridge:agent-sessions'
  ]) {
    assert.equal(isTerminalInputCommand(command), false, `${command} belongs in the headline`);
  }

  // ...and it is keyed on the name the production wrapper ACTUALLY counts, so
  // the two cannot drift apart. (Outside Tauri these resolve without any IPC.)
  const counted = [];
  const wrapped = tauriTerminalBackend((command) => counted.push(command));
  await wrapped.write('pty-1', '[O');
  assert.deepEqual(counted, ['write_terminal_session'], 'a write counts under that exact name');
  await wrapped.resize('pty-1', 80, 24);
  assert.deepEqual(
    counted.map(isTerminalInputCommand),
    [true, false],
    'the write lands in the input bucket and the resize in the headline'
  );
}

{
  // F5: after a live re-attach, the replayed scrollback cannot rebuild a
  // full-screen TUI frame (the backend trims its 256 KB buffer mid-escape, and
  // the bytes that drew the input box are long gone) — so the service nudges the
  // PTY's size down a row and straight back, and the program repaints itself.
  const log = [];
  const { backend } = makeBackend(log);
  const svc = createTerminalService({
    backend,
    createView: (_host, hooks) => makeView(log, 'live', { hooks }),
    repaintNudgeMs: 5
  });
  await svc.attach();
  await svc.adoptExisting(
    { ...ownedA, ownedId: 'n', ptySessionId: 'pty-live' },
    {},
    { cols: 146, rows: 46 }
  );
  assert.equal(
    log.filter((e) => e[0] === 'resize').length,
    0,
    'the nudge is scheduled, not fired inline — the TUI has to be reading first'
  );

  await settle();
  assert.deepEqual(
    log.filter((e) => e[0] === 'resize'),
    [
      ['resize', 'pty-live', 146, 45],
      ['resize', 'pty-live', 146, 46]
    ],
    'exactly ONE nudge: down a row, then straight back to the real geometry'
  );

  const after = log.length;
  await settle();
  svc.show('n');
  assert.equal(
    log.slice(after).filter((e) => e[0] === 'resize').length,
    0,
    'it fires once, and leaves the gate holding the size the PTY really is'
  );
}

{
  // F5b: a TOMBSTONE gets no nudge — there is no process left to signal.
  const log = [];
  const { backend } = makeBackend(log);
  const svc = createTerminalService({
    backend,
    createView: (_host, hooks) => makeView(log, 'dead', { hooks }),
    repaintNudgeMs: 5
  });
  await svc.attach();
  await svc.adoptExisting(
    { ...ownedA, ownedId: 'd', ptySessionId: 'pty-dead', state: 'exited' },
    {},
    { cols: 146, rows: 46 }
  );
  await settle();
  assert.equal(
    log.filter((e) => e[0] === 'resize').length,
    0,
    'an exited session is never resized'
  );
}

{
  // F5c: a close (or an exit, or dispose) that beats the timer CANCELS it —
  // nudging a PTY nobody owns any more would be IO from a dead shell.
  const log = [];
  const { backend, emit } = makeBackend(log);
  const svc = createTerminalService({
    backend,
    createView: (_host, hooks) => makeView(log, 'gone', { hooks }),
    repaintNudgeMs: 5
  });
  await svc.attach();
  await svc.adoptExisting(
    { ...ownedA, ownedId: 'c', ptySessionId: 'pty-live' },
    {},
    { cols: 146, rows: 46 }
  );
  await svc.closeOwned('c');
  await settle();
  assert.equal(
    log.filter((e) => e[0] === 'resize').length,
    0,
    'a close before the nudge fires cancels it'
  );

  // Same for a PTY that exits during the delay.
  await svc.adoptExisting(
    { ...ownedA, ownedId: 'x', ptySessionId: 'pty-exiting' },
    {},
    { cols: 146, rows: 46 }
  );
  emit({ sessionId: 'pty-exiting', data: '', terminated: true, exitCode: 0, signal: null });
  await settle();
  assert.equal(
    log.filter((e) => e[0] === 'resize').length,
    0,
    'and an exit during the delay cancels it too'
  );
}

{
  // F6: the backend ring is 16 MB but the view keeps 20000 lines, so a replay
  // is capped to the 4 MB TAIL before it ever reaches the view.
  const CAP = 4 * 1024 * 1024;
  const log = [];
  const { backend, setScrollback } = makeBackend(log);
  const big = 'A'.repeat(5 * 1024 * 1024) + 'TAIL-SENTINEL';
  setScrollback(big);
  const svc = createTerminalService({ backend, createView: () => makeView(log, 'viewBig') });
  await svc.attach();
  await svc.adoptExisting({ ...ownedA, ownedId: 'big', ptySessionId: 'pty-big' }, {});
  const written = log.find((e) => e[0] === 'viewBig' && e[1] === 'write')?.[2];
  assert.equal(written?.length, CAP, 'the replay is capped at 4 MB');
  assert.ok(written.endsWith('TAIL-SENTINEL'), 'and it is the TAIL that survives');
  assert.ok(big.endsWith(written), 'the replay is an exact suffix of the scrollback');
  svc.dispose();

  // ...and a cut that would land between the halves of a surrogate pair drops
  // the orphaned low half instead of writing a lone unpaired code unit.
  const log2 = [];
  const { backend: backend2, setScrollback: setScrollback2 } = makeBackend(log2);
  // len = 2*CAP + 2, so the raw slice point (len - CAP) lands on an ODD offset
  // inside the emoji run — i.e. on a low surrogate.
  setScrollback2('B' + '\u{1F600}'.repeat(CAP) + 'C');
  const svc2 = createTerminalService({ backend: backend2, createView: () => makeView(log2, 'v2') });
  await svc2.attach();
  await svc2.adoptExisting({ ...ownedA, ownedId: 'pair', ptySessionId: 'pty-pair' }, {});
  const written2 = log2.find((e) => e[0] === 'v2' && e[1] === 'write')?.[2];
  assert.equal(written2?.length, CAP - 1, 'the orphaned low surrogate is dropped');
  assert.equal(written2.charCodeAt(0), 0xd83d, 'the replay starts on a HIGH surrogate');
  assert.ok(written2.endsWith('\u{1F600}C'), 'and still ends at the true tail');
  svc2.dispose();
}

console.log('terminalService tests passed');
