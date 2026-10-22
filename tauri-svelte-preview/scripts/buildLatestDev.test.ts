import assert from 'node:assert/strict';
import test from 'node:test';
import { buildLatestDev, cargoRunVariables, relaunchCommand, type BuildLatestOptions } from './buildLatestDev.ts';

type Call = [string, string[], string];

function fixture(overrides: { branch?: string; status?: string; fail?: string; failHosts?: string[] } = {}) {
  const calls: Call[] = [];
  const lines: string[] = [];
  const deployed: string[] = [];
  const relaunched: Array<ReturnType<typeof relaunchCommand>> = [];
  const options: BuildLatestOptions = {
    checkoutRoot: '/checkout',
    appPid: 4242,
    logPath: '/logs/build-latest.log',
    hosts: ['box-a', 'box-b'],
    run: async (file, args, cwd, onLine) => {
      calls.push([file, args, cwd]);
      if (overrides.fail && [file, ...args].join(' ').startsWith(overrides.fail)) throw new Error(`${file} exited with 101`);
      if (args.includes('--show-current')) return `${overrides.branch ?? 'main'}\n`;
      if (args.includes('--porcelain')) return overrides.status ?? '';
      if (args.includes('rev-parse')) return 'abc1234\n';
      onLine?.(`${file} output`);
      return '';
    },
    deploy: async (host, log) => {
      deployed.push(host);
      if (overrides.failHosts?.includes(host)) throw new Error('ssh: connect timed out');
      log('Verify service: 1.0s');
    },
    relaunch: (command) => relaunched.push(command),
    out: (line) => lines.push(line)
  };
  return { options, calls, lines, deployed, relaunched };
}

test('refuses a checkout that is not on main, before changing anything', async () => {
  const { options, calls, lines, deployed, relaunched } = fixture({ branch: 'tsk-1-feature' });
  assert.equal(await buildLatestDev(options), false);
  assert.deepEqual(calls.map(([file, args]) => [file, ...args].join(' ')), ['git -C /checkout branch --show-current']);
  assert.match(lines.at(-1) ?? '', /^\[1\/4\] Failed: the checkout is on tsk-1-feature, not main/);
  assert.deepEqual(deployed, []);
  assert.deepEqual(relaunched, []);
});

test('refuses a checkout with uncommitted changes and names them', async () => {
  const { options, calls, lines, relaunched } = fixture({ status: ' M src/app.ts\n?? notes.txt\n' });
  assert.equal(await buildLatestDev(options), false);
  assert.equal(calls.length, 2);
  assert.match(lines.join('\n'), /\[1\/4\] Failed: the checkout has uncommitted changes/);
  assert.match(lines.join('\n'), /\[1\/4\]  M src\/app\.ts/);
  assert.deepEqual(relaunched, []);
});

test('pulls, builds, deploys to every saved machine, then hands off the restart', async () => {
  const { options, calls, lines, deployed, relaunched } = fixture();
  assert.equal(await buildLatestDev(options), true);
  assert.deepEqual(calls, [
    ['git', ['-C', '/checkout', 'branch', '--show-current'], '/checkout'],
    ['git', ['-C', '/checkout', 'status', '--porcelain'], '/checkout'],
    ['git', ['-C', '/checkout', 'pull', '--ff-only', 'origin', 'main'], '/checkout'],
    ['git', ['-C', '/checkout', 'rev-parse', '--short', 'HEAD'], '/checkout'],
    ['pnpm', ['install', '--frozen-lockfile', '--config.confirmModulesPurge=false'], '/checkout/tauri-svelte-preview'],
    ['cargo', ['build', '--manifest-path', 'src-tauri/Cargo.toml'], '/checkout/tauri-svelte-preview']
  ]);
  assert.deepEqual(deployed, ['box-a', 'box-b']);
  assert.deepEqual(relaunched, [relaunchCommand('/checkout', 4242, '/logs/build-latest.log')]);
  assert.ok(lines.includes('[2/4] cargo output'), 'build output is shown under step 2');
  assert.ok(lines.includes('[3/4] box-a: Verify service: 1.0s'));
  assert.equal(lines.at(-1), '[4/4] Restarting Assembly on abc1234');
  for (const line of lines) assert.match(line, /^\[[1-4]\/4\] /);
});

test('a failed build deploys nothing and leaves the running app alone', async () => {
  const { options, lines, deployed, relaunched } = fixture({ fail: 'cargo build' });
  assert.equal(await buildLatestDev(options), false);
  assert.deepEqual(deployed, []);
  assert.deepEqual(relaunched, []);
  assert.equal(lines.at(-1), '[2/4] Failed: cargo exited with 101. Log: /logs/build-latest.log');
});

test('a failed pull stops before the build', async () => {
  const { options, calls, relaunched } = fixture({ fail: 'git -C /checkout pull' });
  assert.equal(await buildLatestDev(options), false);
  assert.ok(!calls.some(([file]) => file === 'cargo'));
  assert.deepEqual(relaunched, []);
});

test('one machine failing to deploy is reported and the restart still happens', async () => {
  const { options, lines, deployed, relaunched } = fixture({ failHosts: ['box-a'] });
  assert.equal(await buildLatestDev(options), true);
  assert.deepEqual(deployed, ['box-a', 'box-b']);
  assert.ok(lines.includes('[3/4] Deploy to box-a failed: ssh: connect timed out'));
  assert.ok(lines.includes('[3/4] Deploy failed on box-a. Log: /logs/build-latest.log'));
  assert.equal(relaunched.length, 1);
});

test('no saved machines still restarts', async () => {
  const { options, lines, relaunched } = fixture();
  options.hosts = [];
  assert.equal(await buildLatestDev(options), true);
  assert.ok(lines.includes('[3/4] No saved machines to deploy to'));
  assert.equal(relaunched.length, 1);
});

test('the restart waits for this app to quit, then starts pnpm app:dev into the log', () => {
  const command = relaunchCommand('/checkout', 4242, '/logs/build-latest.log');
  assert.equal(command.file, '/bin/sh');
  assert.equal(command.cwd, '/checkout');
  assert.deepEqual(command.args.slice(2), ['sh', '4242', '/logs/build-latest.log']);
  const script = command.args[1];
  assert.equal(command.args[0], '-c');
  assert.match(script, /while kill -0 "\$1"/);
  assert.match(script, /exec pnpm app:dev >>"\$2" 2>&1$/);
  assert.match(script, /-gt 600/, 'gives up after ten minutes if the app never quits');
});

test('drops the variables cargo run set for the app, so the build matches tauri dev and reuses its cache', () => {
  const env = {
    CARGO_PKG_NAME: 'mac-command-bar-webview-preview', CARGO_PKG_VERSION_MAJOR: '0', CARGO_MANIFEST_DIR: '/checkout/src-tauri',
    CARGO_MANIFEST_PATH: '/checkout/src-tauri/Cargo.toml', CARGO_CRATE_NAME: 'app', CARGO_BIN_NAME: 'app', CARGO_PRIMARY_PACKAGE: '1',
    CARGO_TERM_PROGRESS_WHEN: 'always', CARGO_TERM_PROGRESS_WIDTH: '80',
    CARGO_HOME: '/home/me/.cargo', CARGO_TARGET_DIR: '/cache/target', CARGO_BUILD_JOBS: '3', TAURI_ENV_TARGET_TRIPLE: 'x86_64', PATH: '/bin'
  };
  assert.deepEqual(cargoRunVariables(env).sort(), [
    'CARGO_BIN_NAME', 'CARGO_CRATE_NAME', 'CARGO_MANIFEST_DIR', 'CARGO_MANIFEST_PATH', 'CARGO_PKG_NAME', 'CARGO_PKG_VERSION_MAJOR',
    'CARGO_PRIMARY_PACKAGE', 'CARGO_TERM_PROGRESS_WHEN', 'CARGO_TERM_PROGRESS_WIDTH'
  ]);
});

test('Stop ends the run and what it is running, and nothing is relaunched', async () => {
  const { execFileSync, spawn } = await import('node:child_process');
  const fs = await import('node:fs');
  const os = await import('node:os');
  const path = await import('node:path');
  const repo = fs.mkdtempSync(path.join(os.tmpdir(), 'build-latest-stop-'));
  try {
    const scripts = path.join(repo, 'tauri-svelte-preview/scripts');
    fs.mkdirSync(scripts, { recursive: true });
    for (const name of ['buildLatestDev.ts', 'deployRemoteBackendDev.ts']) fs.copyFileSync(new URL(`./${name}`, import.meta.url), path.join(scripts, name));
    const git = (...args: string[]) => execFileSync('git', ['-C', repo, ...args], { stdio: 'pipe' });
    git('init', '-q', '-b', 'main');
    git('-c', 'user.name=Test', '-c', 'user.email=test@example.invalid', 'add', '.');
    git('-c', 'user.name=Test', '-c', 'user.email=test@example.invalid', 'commit', '-qm', 'scripts');
    // The pull hangs on a "network" that never answers, the way a stuck pull or deploy would.
    git('remote', 'add', 'origin', 'ssh://example.invalid/repo.git');
    const log = path.join(repo, 'build-latest.log');
    const run = spawn(process.execPath, ['--experimental-strip-types', path.join(scripts, 'buildLatestDev.ts'), '--pid', '1', '--log', log], {
      detached: true, stdio: ['ignore', 'pipe', 'pipe'], env: { ...process.env, GIT_SSH_COMMAND: 'sleep 60;:' }
    });
    let output = '';
    await new Promise<void>((resolve) => run.stdout.on('data', (chunk: Buffer) => {
      output += chunk.toString();
      if (output.includes('[1/4] Pulling main')) resolve();
    }));
    // What the app's Stop does: SIGTERM to the run's process group.
    process.kill(-run.pid!, 'SIGTERM');
    const code = await new Promise((resolve) => run.on('close', (exitCode, signal) => resolve(exitCode ?? signal)));
    assert.notEqual(code, 0);
    assert.match(fs.readFileSync(log, 'utf8'), /Stopped\.\n$/);
    assert.throws(() => process.kill(-run.pid!, 0), /ESRCH/, 'nothing in the run is left, the hung pull included');
  } finally {
    fs.rmSync(repo, { recursive: true, force: true });
  }
});
