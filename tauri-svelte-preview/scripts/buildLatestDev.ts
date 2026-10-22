// Build latest (dev): pull main, build, deploy the backend to every saved machine, then restart
// Assembly on the new build. The dev app starts this in its own process group, so it keeps
// going when the app quits. Every line is prefixed "[step/4]" for the app's progress display
// and also written to the log file.
import { spawn } from 'node:child_process';
import { appendFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { createInterface } from 'node:readline';
import { fileURLToPath } from 'node:url';
import { deployRemoteBackendDev } from './deployRemoteBackendDev.ts';

const steps = 4;

/** Runs a command and resolves with its stdout; streams each output line to `onLine`. */
export type Runner = (file: string, args: string[], cwd: string, onLine?: (line: string) => void) => Promise<string>;
type Command = { file: string; args: string[]; cwd: string };
export type BuildLatestOptions = {
  checkoutRoot: string;
  appPid: number;
  logPath: string;
  hosts: string[];
  run: Runner;
  deploy: (host: string, log: (line: string) => void) => Promise<void>;
  relaunch: (command: Command) => void;
  out: (line: string) => void;
};

/** Waits for the running app to quit (ten minutes at most), then starts the new build. */
export function relaunchCommand(checkoutRoot: string, appPid: number, logPath: string): Command {
  const script = 'i=0; while kill -0 "$1" 2>/dev/null; do i=$((i+1)); if [ "$i" -gt 600 ]; then '
    + 'echo "Assembly did not quit within 10 minutes; start it with pnpm app:dev" >>"$2"; exit 1; fi; sleep 1; done; '
    + 'exec pnpm app:dev >>"$2" 2>&1';
  return { file: '/bin/sh', args: ['-c', script, 'sh', String(appPid), logPath], cwd: checkoutRoot };
}

/**
 * The variables `cargo run` (under tauri dev) set for the running app, which this script inherits.
 * Build scripts such as ring's rerun when they change, so left in, this build and the next
 * tauri dev build would each recompile part of the tree instead of reusing the cache.
 */
export function cargoRunVariables(env: Record<string, string | undefined>): string[] {
  return Object.keys(env).filter((key) => /^CARGO_(PKG_|MANIFEST_|TERM_PROGRESS_|CRATE_NAME$|BIN_NAME$|PRIMARY_PACKAGE$)/.test(key));
}

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/** Resolves true once the restart is handed off; false when it refused or a step failed. */
export async function buildLatestDev(options: BuildLatestOptions): Promise<boolean> {
  const { checkoutRoot: root, run, logPath } = options;
  const preview = path.join(root, 'tauri-svelte-preview');
  let step = 1;
  const say = (line: string) => options.out(`[${step}/${steps}] ${line}`);
  const git = (args: string[], onLine?: (line: string) => void) => run('git', ['-C', root, ...args], root, onLine);
  try {
    say('Pulling main');
    const branch = (await git(['branch', '--show-current'])).trim();
    if (branch !== 'main') throw new Error(`the checkout is on ${branch || 'a detached HEAD'}, not main. Switch it to main first.`);
    const changes = (await git(['status', '--porcelain'])).trimEnd();
    if (changes) {
      changes.split('\n').slice(0, 20).forEach((line) => say(line));
      throw new Error('the checkout has uncommitted changes. Commit or stash them first.');
    }
    await git(['pull', '--ff-only', 'origin', 'main'], say);
    const commit = (await git(['rev-parse', '--short', 'HEAD'])).trim();
    say(`Now at ${commit}`);

    step = 2;
    say('Building Assembly');
    // No terminal to answer pnpm's "remove node_modules?" question, which it would otherwise abort on.
    await run('pnpm', ['install', '--frozen-lockfile', '--config.confirmModulesPurge=false'], preview, say);
    await run('cargo', ['build', '--manifest-path', 'src-tauri/Cargo.toml'], preview, say);

    step = 3;
    const failed: string[] = [];
    for (const host of options.hosts) {
      say(`Deploying the backend to ${host}`);
      try {
        await options.deploy(host, (line) => say(`${host}: ${line}`));
      } catch (error) {
        failed.push(host);
        say(`Deploy to ${host} failed: ${message(error)}`);
      }
    }
    if (options.hosts.length === 0) say('No saved machines to deploy to');
    else if (failed.length) say(`Deploy failed on ${failed.join(', ')}. Log: ${logPath}`);
    else say(`Deployed to ${options.hosts.length} machine${options.hosts.length === 1 ? '' : 's'}`);

    step = 4;
    say(`Restarting Assembly on ${commit}`);
    options.relaunch(relaunchCommand(root, options.appPid, logPath));
    return true;
  } catch (error) {
    say(`Failed: ${message(error).replace(/\.$/, '')}. Log: ${logPath}`);
    return false;
  }
}

const streamingRunner: Runner = (file, args, cwd, onLine) => new Promise((resolve, reject) => {
  const child = spawn(file, args, { cwd, stdio: ['ignore', 'pipe', 'pipe'] });
  let stdout = '';
  const tail: string[] = [];
  child.stdout.on('data', (chunk: Buffer) => { stdout += chunk.toString(); });
  for (const stream of [child.stdout, child.stderr]) {
    createInterface({ input: stream }).on('line', (line) => {
      onLine?.(line);
      tail.push(line);
      if (tail.length > 5) tail.shift();
    });
  }
  child.on('error', reject);
  child.on('close', (code) => {
    if (code === 0) resolve(stdout);
    else reject(new Error(`${file} exited with ${code}${tail.length && !onLine ? `: ${tail.join(' / ')}` : ''}`));
  });
});

function argValue(args: string[], name: string): string {
  const index = args.indexOf(name);
  if (index < 0 || !args[index + 1]) throw new Error(`usage: buildLatestDev.ts --pid <app pid> --log <file> [--dry-run] [ssh-host ...]`);
  return args.splice(index, 2)[1];
}

async function main(): Promise<void> {
  const args = process.argv.slice(2);
  const dryRun = args.includes('--dry-run');
  if (dryRun) args.splice(args.indexOf('--dry-run'), 1);
  const appPid = Number(argValue(args, '--pid'));
  const logPath = argValue(args, '--log');
  const checkoutRoot = path.resolve(fileURLToPath(new URL('../..', import.meta.url)));
  for (const key of cargoRunVariables(process.env)) delete process.env[key];
  // The app may quit while this runs; the log keeps the rest.
  process.stdout.on('error', () => {});
  writeFileSync(logPath, '');
  const out = (line: string) => {
    process.stdout.write(`${line}\n`);
    appendFileSync(logPath, `${line}\n`);
  };
  // A dry run reads the checkout for real and prints every command that would change something.
  const readOnly = (args: string[]) => args.includes('--show-current') || args.includes('--porcelain') || args.includes('rev-parse');
  const ok = await buildLatestDev({
    checkoutRoot, appPid, logPath, hosts: args, out,
    run: dryRun
      ? async (file, commandArgs, cwd, onLine) => {
        if (file === 'git' && readOnly(commandArgs)) return streamingRunner(file, commandArgs, cwd);
        onLine?.(`would run in ${cwd}: ${[file, ...commandArgs].join(' ')}`);
        return '';
      }
      : streamingRunner,
    deploy: dryRun
      ? async (host, log) => log(`would run deployRemoteBackendDev(${host})`)
      : (host, log) => deployRemoteBackendDev(host, { checkoutRoot, log }),
    relaunch: dryRun
      ? (command) => out(`would start detached in ${command.cwd}: ${command.file} ${command.args.map((arg) => JSON.stringify(arg)).join(' ')}`)
      : (command) => spawn(command.file, command.args, { cwd: command.cwd, detached: true, stdio: 'ignore' }).unref()
  });
  process.exitCode = ok ? 0 : 1;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch((error: unknown) => {
    process.stderr.write(`${message(error)}\n`);
    process.exitCode = 1;
  });
}
