import assert from 'node:assert/strict';
import { createTerminalService } from '../src/lib/shell/terminalService.ts';

/**
 * Spy TerminalView — every method appends to the shared `log` so a test can
 * assert on the exact IO the service performed. No xterm, no DOM.
 */
function makeView(log, name) {
  return {
    write: (d) => log.push([name, 'write', d]),
    fit: () => log.push([name, 'fit']),
    focus: () => log.push([name, 'focus']),
    setVisible: (v) => log.push([name, 'visible', v]),
    dispose: () => log.push([name, 'dispose'])
  };
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
        return 'OLD OUTPUT';
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
    createView: (host, viewHooks) => {
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

console.log('terminalService tests passed');
