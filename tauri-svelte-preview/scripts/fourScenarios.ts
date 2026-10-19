import { execFileSync, spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const home = os.homedir();
const proofRoot = path.join(home, 'assembly-proof');
const templateDir = path.join(proofRoot, 'template');
const templateSource = path.join(home, 'assembly-qa-1500b', 'xdg-fresh');
const currentLink = path.join(proofRoot, 'current');
const ownerDataPaths = [path.join(home, '.local/share/assembly'), path.join(home, '.local/share/dev.blackcolours.MacCommandBarWebviewPreview')];

type ProcessInfo = { name: string; pid: number; startedAt: string; log: string };
type StateInput = { runDir: string; display: number; port: number; dataDir: string; worktree: string; processes: ProcessInfo[] };

export function pickFreeDisplay(isTaken: (n: number) => boolean, from = 140): number {
  for (let n = from; n < from + 60; n += 1) if (!isTaken(n)) return n;
  throw new Error(`No free X display from :${from}`);
}

export function pickFreePort(listening: Set<number>, from = 5300): number {
  let port = from;
  while (listening.has(port)) port += 1;
  return port;
}

export function parseListeningPorts(ssOutput: string): Set<number> {
  const ports = new Set<number>();
  for (const line of ssOutput.split('\n')) {
    const local = line.trim().split(/\s+/)[3];
    const port = Number(local?.slice(local.lastIndexOf(':') + 1));
    if (Number.isInteger(port)) ports.add(port);
  }
  return ports;
}

export function parseAvailableGb(freeOutput: string): number {
  const line = freeOutput.split('\n').find((l) => l.startsWith('Mem:'));
  const available = Number(line?.trim().split(/\s+/)[6]);
  if (!Number.isFinite(available)) throw new Error(`Cannot read available memory from: ${freeOutput}`);
  return available;
}

// Resolves symlinks in the longest existing part of the path, so an alias cannot reach the owner's data.
function canonical(target: string): string {
  const resolved = path.resolve(target);
  if (fs.existsSync(resolved)) return fs.realpathSync(resolved);
  const parent = path.dirname(resolved);
  return parent === resolved ? resolved : path.join(canonical(parent), path.basename(resolved));
}

export function refuseOwnerData(dataHome: string, homeDir: string): void {
  const owner = canonical(path.join(homeDir, '.local/share'));
  const resolved = canonical(dataHome);
  if (resolved === owner || resolved.startsWith(owner + path.sep)) {
    throw new Error(`Refusing: ${resolved} is the owner's data folder (${owner}).`);
  }
}

export function buildState(input: StateInput) {
  return {
    runDir: input.runDir,
    display: `:${input.display}`,
    port: input.port,
    dataDir: input.dataDir,
    worktree: input.worktree,
    processes: input.processes.map((p) => ({ ...p, pgid: p.pid }))
  };
}

type RunState = { runDir: string; processes: { pgid: number; pid: number; startedAt: string }[] };

// `stop` reads only the current link, so a second start would orphan the first run's processes.
export function refuseWhileRunning(state: RunState | undefined, isLive: (p: RunState['processes'][number]) => boolean): void {
  if (state?.processes.some(isLive)) {
    throw new Error(`A proof run is still running in ${state.runDir}. Stop it first with: node --experimental-strip-types tauri-svelte-preview/scripts/fourScenarios.ts stop`);
  }
}

export function ownerFilesIn(openPaths: string[], ownerDirs: string[]): string[] {
  return openPaths.filter((f) => ownerDirs.some((d) => f === d || f.startsWith(d + path.sep)));
}

// The group leader may exit while its children run on. A live group whose leader pid is gone is
// still ours: Linux does not reuse a pid while a process group with that id exists.
export function ownsGroup(p: { startedAt: string }, groupIsAlive: boolean, leaderStartTime: string): boolean {
  return groupIsAlive && (leaderStartTime === p.startedAt || leaderStartTime === '');
}

type PsRow = { pid: number; ppid: number; pgid: number };

// Agent sidecars and language servers start their own process groups, so signalling only the
// recorded groups would leave them running. Walk the parent links from the recorded roots.
export function descendantGroups(rows: PsRow[], roots: number[]): number[] {
  const seen = new Set(roots);
  for (let grew = true; grew; ) {
    grew = false;
    for (const r of rows) if (seen.has(r.ppid) && !seen.has(r.pid)) { seen.add(r.pid); grew = true; }
  }
  return [...new Set(rows.filter((r) => seen.has(r.pid)).map((r) => r.pgid))];
}

const run = (file: string, args: string[], env?: NodeJS.ProcessEnv) =>
  execFileSync(file, args, { encoding: 'utf8', env: env ?? process.env, stdio: ['ignore', 'pipe', 'pipe'] });
// A display that is still starting makes xwininfo fail; treat that as "not ready yet".
const probe = (args: string[]) => { try { return run('xwininfo', args); } catch { return ''; } };
const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const startTime = (pid: number) => { try { return run('ps', ['-o', 'lstart=', '-p', String(pid)]).trim(); } catch { return ''; } };
const groupAlive = (pgid: number) => { try { process.kill(-pgid, 0); return true; } catch { return false; } };
const newestMtime = (dir: string): number => {
  let newest = fs.statSync(dir).mtimeMs;
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    newest = Math.max(newest, entry.isDirectory() ? newestMtime(full) : fs.statSync(full).mtimeMs);
  }
  return newest;
};
const owned = (p: { pid: number; pgid: number; startedAt: string }) => ownsGroup(p, groupAlive(p.pgid), startTime(p.pid));
// Files held open by every process in the given groups; the owner's own app writes its data all the time,
// so timestamps can't tell us whether the test app touched it.
const openFilesInGroups = (pgids: number[]): string[] => {
  const groups = new Set(pgids.map(String));
  const pids = run('ps', ['-eo', 'pid=,pgid=']).split('\n').map((l) => l.trim().split(/\s+/)).filter((c) => groups.has(c[1])).map((c) => c[0]);
  return pids.flatMap((pid) => {
    try {
      return fs.readdirSync(`/proc/${pid}/fd`).map((fd) => { try { return fs.readlinkSync(`/proc/${pid}/fd/${fd}`); } catch { return ''; } });
    } catch { return []; }
  });
};

function launch(name: string, file: string, args: string[], cwd: string, log: string, env: NodeJS.ProcessEnv): ProcessInfo {
  const fd = fs.openSync(log, 'a');
  const child = spawn(file, args, { cwd, env, detached: true, stdio: ['ignore', fd, fd] });
  child.unref();
  if (!child.pid) throw new Error(`Could not start ${name}`);
  return { name, pid: child.pid, startedAt: startTime(child.pid), log };
}

async function start(argv: string[]) {
  const option = (flag: string) => (argv.includes(flag) ? argv[argv.indexOf(flag) + 1] : undefined);
  const worktree = path.resolve(option('--worktree') ?? path.join(path.dirname(fileURLToPath(import.meta.url)), '../..'));
  const preview = path.join(worktree, 'tauri-svelte-preview');

  const previous = path.join(currentLink, 'state.json');
  refuseWhileRunning(
    fs.existsSync(previous) ? (JSON.parse(fs.readFileSync(previous, 'utf8')) as RunState) : undefined,
    owned
  );

  const available = parseAvailableGb(run('free', ['-g']));
  if (available <= 6) throw new Error(`Only ${available} GB available; need more than 6 GB. Wait and retry.`);

  const stamp = new Date().toISOString().replace(/[:.]/g, '-');
  const runDir = path.join(proofRoot, stamp);
  const dataDir = option('--data') ? path.resolve(option('--data')!) : path.join(runDir, 'data');
  refuseOwnerData(dataDir, home);

  const display = pickFreeDisplay((n) => fs.existsSync(`/tmp/.X${n}-lock`) || fs.existsSync(`/tmp/.X11-unix/X${n}`));
  const port = pickFreePort(parseListeningPorts(run('ss', ['-H', '-ltn'])));

  fs.mkdirSync(runDir, { recursive: true });
  if (!option('--data')) {
    if (!fs.existsSync(templateDir)) {
      if (!fs.existsSync(templateSource)) {
        throw new Error(`No template data folder at ${templateDir}. Make one: start the app with XDG_DATA_HOME set to an empty folder, add Workbox Test in Settings -> Connections, quit, and copy that folder to ${templateDir}.`);
      }
      console.log(`Template ${templateDir} is missing; creating it once from ${templateSource}.`);
      fs.cpSync(templateSource, templateDir, { recursive: true, preserveTimestamps: true });
    }
    fs.cpSync(templateDir, dataDir, { recursive: true, preserveTimestamps: true });
  }
  if (!fs.existsSync(dataDir)) throw new Error(`Data folder ${dataDir} does not exist.`);

  const launchedAt = Date.now();
  const processes: ProcessInfo[] = [];
  const writeState = () => {
    const state = buildState({ runDir, display, port, dataDir, worktree, processes });
    fs.writeFileSync(path.join(runDir, 'state.json'), JSON.stringify(state, null, 2));
    fs.rmSync(currentLink, { force: true });
    fs.symlinkSync(runDir, currentLink);
  };
  const log = (name: string) => path.join(runDir, `${name}.log`);

  const x = `:${display}`;
  processes.push(launch('xvfb', 'Xvfb', [x, '-screen', '0', '1710x990x24', '-nolisten', 'tcp'], runDir, log('xvfb'), process.env));
  writeState();
  for (let i = 0; i < 20 && !probe(['-display', x, '-root']).includes('1710x990'); i += 1) await sleep(500);

  processes.push(launch('vite', 'pnpm', ['dev', '--host', '127.0.0.1', '--port', String(port), '--strictPort'], preview, log('vite'), process.env));
  writeState();

  const windowConfig = JSON.stringify({
    build: { devUrl: `http://127.0.0.1:${port}/` },
    app: { windows: [{ label: 'main', title: 'Assembly', width: 1600, height: 990, minWidth: 900, minHeight: 600, url: '/', dragDropEnabled: false }] }
  });
  const appEnv = { ...process.env, PATH: `${home}/.cargo/bin:${process.env.PATH}`, CARGO_BUILD_JOBS: '4', MSBUILDDISABLENODEREUSE: '1', XDG_DATA_HOME: dataDir, DISPLAY: x, GDK_BACKEND: 'x11', WEBKIT_DISABLE_DMABUF_RENDERER: '1' };
  processes.push(launch('app', 'pnpm', ['tauri', 'dev', '--config', 'src-tauri/tauri.dev.attach.conf.json', '--config', windowConfig], preview, log('app'), appEnv));
  writeState();

  console.log(`Started in ${runDir} (display ${x}, vite port ${port}). Waiting for the Assembly window (first build can take 10+ minutes).`);
  const app = processes[2];
  const deadline = Date.now() + 10 * 60_000;
  for (;;) {
    if (probe(['-display', x, '-root', '-tree']).includes('"Assembly"')) break;
    if (!groupAlive(app.pid)) throw new Error(`The app process exited before a window appeared. See ${app.log}`);
    if (Date.now() > deadline) throw new Error(`No Assembly window after 10 minutes. See ${app.log}`);
    await sleep(5000);
  }
  const shot = path.join(runDir, '00-started.png');
  // The window exists before the page paints; a blank capture is only a few KB.
  for (let i = 0; i < 30; i += 1) {
    await sleep(2000);
    run('assembly-native-ui', ['shot', shot], { ...process.env, DISPLAY: x });
    if (fs.statSync(shot).size > 20_000) break;
  }

  const touched = newestMtime(dataDir) > launchedAt;
  const ownerOpen = ownerFilesIn(openFilesInGroups(processes.map((p) => p.pid)), ownerDataPaths);
  console.log(`Isolation: files written under ${dataDir}: ${touched ? 'yes' : 'NO'}; owner data files open by the test app: ${ownerOpen.length ? ownerOpen.join(', ') : 'none'}`);
  if (!touched || ownerOpen.length) console.log('Isolation check FAILED. Run stop and report this.');
  console.log(`Screenshot: ${shot}\nRun folder: ${runDir}\nDisplay: ${x}\nDrive: DISPLAY=${x} assembly-native-ui shot|click|key ...\nNext: see docs/operations/four-scenario-check.md. Stop with: node --experimental-strip-types tauri-svelte-preview/scripts/fourScenarios.ts stop`);
}

async function stop() {
  const state = JSON.parse(fs.readFileSync(path.join(currentLink, 'state.json'), 'utf8')) as ReturnType<typeof buildState>;
  const mine = [...state.processes].reverse().filter(owned);
  const rows = run('ps', ['-eo', 'pid=,ppid=,pgid=']).trim().split('\n').map((l) => {
    const [pid, ppid, pgid] = l.trim().split(/\s+/).map(Number);
    return { pid, ppid, pgid };
  });
  // Collected before any signal, while the parent links still exist.
  const groups = [...new Set([...mine.map((p) => p.pgid), ...descendantGroups(rows, mine.map((p) => p.pid))])];
  for (const g of groups) { try { process.kill(-g, 'SIGTERM'); } catch { /* already gone */ } }
  for (let i = 0; i < 10 && groups.some(groupAlive); i += 1) await sleep(1000);
  for (const g of groups.filter(groupAlive)) { try { process.kill(-g, 'SIGKILL'); } catch { /* already gone */ } }
  await sleep(500);
  for (const p of state.processes) {
    const gone = !groupAlive(p.pgid);
    console.log(`${p.name} (process group ${p.pgid}): ${mine.includes(p) ? 'stopped' : 'was not running'}${gone ? ', gone' : ', STILL RUNNING'}`);
  }
  const checked = new Set([...groups, ...state.processes.map((p) => p.pgid)].map(String));
  const left = run('ps', ['-eo', 'pid,pgid,cmd']).split('\n').filter((l) => checked.has(l.trim().split(/\s+/)[1]));
  console.log(left.length ? `STILL RUNNING:\n${left.join('\n')}` : 'ps check: no processes left in the stopped groups');
}

function offlineSsh() {
  const state = JSON.parse(fs.readFileSync(path.join(currentLink, 'state.json'), 'utf8')) as ReturnType<typeof buildState>;
  const dir = path.join(state.runDir, 'offline-bin');
  fs.mkdirSync(dir, { recursive: true });
  fs.writeFileSync(path.join(dir, 'ssh'), '#!/bin/sh\necho "ssh: offline for the test" >&2\nexit 255\n', { mode: 0o755 });
  console.log(dir);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const [command, ...rest] = process.argv.slice(2);
  const commands: Record<string, () => unknown> = { start: () => start(rest), stop, 'offline-ssh': offlineSsh };
  if (!commands[command]) { console.error('usage: fourScenarios.ts start [--worktree <path>] [--data <existing data dir>] | stop | offline-ssh'); process.exit(2); }
  Promise.resolve(commands[command]()).catch((error) => { console.error(error instanceof Error ? error.message : error); process.exit(1); });
}
