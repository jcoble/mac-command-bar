import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

type Baseline = { fileSize: Record<string, number>; providerBranches: Record<string, number> };
type StagedFile = { path: string; source: string; addedLines?: number[] };
const baselinePath = 'tauri-svelte-preview/scripts/commitChecks.baseline.json';
const providerFolder = 'tauri-svelte-preview/src-tauri/src/agent_conversation/providers/';
const providerName = `(?:['"](?:codex|claude|antigravity)['"]|(?:\\w+::)+(?:Codex|Claude|Antigravity))`;
const providerBranch = new RegExp(
  `[!=]==?\\s*${providerName}|${providerName}\\s*[!=]==?|\\bcase\\s+${providerName}\\s*:|${providerName}[^=\\n]*=>|\\bmatches!\\s*\\([^\\n]*${providerName}`
);

function isSource(path: string): boolean {
  return /\.(rs|ts|svelte)$/.test(path)
    && !/(^|\/)(node_modules|target|build|generated|__generated__|\.svelte-kit)\//.test(path)
    && !/\.(generated|gen)\.(rs|ts|svelte)$/.test(path);
}

function isGenerated(source: string): boolean {
  return /@generated|auto[- ]generated|automatically generated|code generated/i.test(source.split('\n').slice(0, 5).join('\n'));
}

function lineCount(source: string): number {
  return source ? source.split('\n').length - Number(source.endsWith('\n')) : 0;
}

function isTestPath(path: string): boolean {
  return /\.test\.[cm]?[jt]s$/.test(path) || path.endsWith('_test.dart') || /(^|\/)tests?\//.test(path);
}

function providerBranchCount(path: string, source: string): number {
  if (path.startsWith(providerFolder) || isTestPath(path)) return 0;
  const production = path.endsWith('.rs') ? withoutTestModules(source) : source;
  return production.split('\n').filter((line) => providerBranch.test(line)).length;
}

// Drops each `#[cfg(test)] mod name { ... }` block; code after it is still production code.
function withoutTestModules(source: string): string {
  const kept: string[] = [];
  let afterAttribute = false;
  let inModule = false;
  let depth = 0;
  for (const line of source.split('\n')) {
    if (!inModule && afterAttribute && /^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*\{/.test(line)) {
      inModule = true;
      kept.pop(); // the #[cfg(test)] line
    }
    afterAttribute = !inModule && /^\s*#\[cfg\(test\)\]\s*$/.test(line);
    if (!inModule) { kept.push(line); continue; }
    const code = line.replace(/"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])'|\/\/.*$/g, '');
    depth += (code.match(/\{/g) ?? []).length - (code.match(/\}/g) ?? []).length;
    if (depth <= 0) { inModule = false; depth = 0; }
  }
  return kept.join('\n');
}

export function checkFiles(files: StagedFile[], baseline: Baseline): string[] {
  const errors: string[] = [];
  for (const { path, source } of files) {
    if (!isSource(path) || isGenerated(source)) continue;
    const count = lineCount(source);
    const recorded = baseline.fileSize[path];
    const limit = recorded ?? 600;
    if (count > limit) errors.push(`${path}: file size is ${count} lines; the limit is ${limit}.`);
    else if (recorded !== undefined && count < recorded) {
      errors.push(`${path}: lower the file size baseline to ${count} lines${count <= 600 ? ' (or remove its entry)' : ''}.`);
    }
    const branches = providerBranchCount(path, source);
    const allowed = baseline.providerBranches[path] ?? 0;
    if (branches > allowed) errors.push(`${path}: provider branch count is ${branches}; the limit outside provider folders is ${allowed}.`);
  }
  return errors;
}

function hasRustTestChange(file: StagedFile): boolean {
  if (!file.path.endsWith('.rs')) return false;
  const added = new Set(file.addedLines ?? []);
  let awaitingModule = false;
  let depth = 0;
  let insideTests = false;
  for (const [index, line] of file.source.split('\n').entries()) {
    if (added.has(index + 1) && /#\s*\[\s*(?:test|tokio::test)(?:\s*\([^\]]*\))?\s*\]/.test(line)) return true;
    if (/^\s*#\[cfg\(test\)\]/.test(line)) awaitingModule = true;
    if (awaitingModule && /\bmod\s+\w+\s*\{/.test(line)) {
      insideTests = true;
      awaitingModule = false;
      depth = 0;
    } else if (awaitingModule && line.trim() && !/^\s*#\[/.test(line)) awaitingModule = false; // the attribute was on another item
    if (!insideTests) continue;
    if (added.has(index + 1) && line.trim() && !/^\s*(?:\/\/|[{}]\s*$)/.test(line)) return true;
    const code = line.replace(/"(?:\\.|[^"\\])*"|\/\/.*$/g, '');
    depth += (code.match(/\{/g) ?? []).length - (code.match(/\}/g) ?? []).length;
    if (depth <= 0) insideTests = false;
  }
  return false;
}

export function checkFixCommit(message: string, files: StagedFile[]): string[] {
  if (!/^fix\b/i.test(message.split('\n')[0])) return [];
  const reason = message.split('\n').slice(1).find((line) => /^No-test:\s*\S/.test(line));
  if (reason) return [reason.trim()];
  if (files.some((file) => isTestPath(file.path) || hasRustTestChange(file))) return [];
  return ['commit message: fix commits must change a test or include No-test: <reason>.'];
}

function addedLineNumbers(diff: string): number[] {
  const added: number[] = [];
  let lineNumber = 0;
  for (const line of diff.split('\n')) {
    const hunk = /^@@ .* \+(\d+)(?:,\d+)? @@/.exec(line);
    if (hunk) lineNumber = Number(hunk[1]);
    else if (line.startsWith('+') && !line.startsWith('+++')) added.push(lineNumber++);
    else if (line.startsWith(' ')) lineNumber++;
  }
  return added;
}

function main(): void {
  const repo = fileURLToPath(new URL('../..', import.meta.url));
  const git = (...args: string[]) => execFileSync('git', args, { cwd: repo, encoding: 'utf8', maxBuffer: 32 * 1024 * 1024 });
  if (process.argv.includes('--write-baseline')) {
    const baseline: Baseline = { fileSize: {}, providerBranches: {} };
    const paths = git('ls-tree', '-r', '--name-only', '-z', 'origin/main').split('\0').filter(isSource).sort();
    for (const path of paths) {
      const source = git('show', `origin/main:${path}`);
      if (isGenerated(source)) continue;
      const count = lineCount(source);
      if (count > 600) baseline.fileSize[path] = count;
      const branches = providerBranchCount(path, source);
      if (branches) baseline.providerBranches[path] = branches;
    }
    writeFileSync(join(repo, baselinePath), JSON.stringify(baseline, null, 2) + '\n');
    console.log(`Wrote ${baselinePath} from origin/main.`);
    return;
  }
  const paths = git('diff', '--cached', '--name-only', '--diff-filter=ACMR', '-z').split('\0').filter(Boolean);
  const messageArg = process.argv.indexOf('--commit-msg');
  if (messageArg !== -1) {
    // Read the message file with the same comment stripping as the existing hook.
    const raw = readFileSync(resolve(process.argv[messageArg + 1]), 'utf8');
    const cleaned = execFileSync('git', ['stripspace', '--strip-comments'], { input: raw, encoding: 'utf8', cwd: repo });
    const files = paths.map((path) => ({
      path,
      source: path.endsWith('.rs') ? git('show', `:${path}`) : '',
      addedLines: path.endsWith('.rs') ? addedLineNumbers(git('diff', '--cached', '-U0', '--', path)) : []
    }));
    const messages = checkFixCommit(cleaned, files);
    for (const message of messages) console.error(message);
    if (messages.some((message) => !message.startsWith('No-test:'))) process.exitCode = 1;
    return;
  }
  const baseline: Baseline = JSON.parse(git('show', `:${baselinePath}`));
  const files = paths.filter(isSource).map((path) => ({ path, source: git('show', `:${path}`) }));
  const errors = checkFiles(files, baseline);
  for (const error of errors) console.error(error);
  if (errors.length) process.exitCode = 1;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { main(); }
  catch (error) {
    console.error(`commit checks: ${error instanceof Error ? error.message.split('\n')[0] : String(error)}`);
    process.exitCode = 1;
  }
}
