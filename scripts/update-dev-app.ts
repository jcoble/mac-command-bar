#!/usr/bin/env -S node --experimental-strip-types
const { spawnSync } = require('node:child_process') as typeof import('node:child_process');
const { resolve } = require('node:path') as typeof import('node:path');

const root = resolve(__dirname, '..');
const frontend = resolve(root, 'tauri-svelte-preview');

function readGit(...args: string[]): string {
  const result = spawnSync('git', args, { cwd: root, encoding: 'utf8' });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(result.stderr.trim() || 'Git failed.');
  return result.stdout.trim();
}

function run(command: string, args: string[], cwd = root): void {
  const result = spawnSync(command, args, { cwd, stdio: 'inherit' });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}

try {
  if (readGit('branch', '--show-current') !== 'main') {
    throw new Error('Run this script from the main checkout. It will not switch branches or move your work.');
  }
  const changes = readGit('status', '--short');
  if (changes) {
    console.log('Local edits will be preserved and included in this dev run:\n' + changes);
  }
  run('git', ['pull', '--ff-only', 'origin', 'main']);
  console.log(`Building Assembly Dev from main at ${readGit('rev-parse', '--short', 'HEAD')}.`);
  run('pnpm', ['install', '--frozen-lockfile'], frontend);
  console.log('Starting Assembly with its normal session database. Keep this terminal open; press Ctrl+C to stop the dev run.');
  run('pnpm', ['app:dev']);
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
}
