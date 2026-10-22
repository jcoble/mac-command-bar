// The read-only merge gate for a workflow PR. Run from the project folder:
//   node --experimental-strip-types mergeGate.ts "<pr>" <head> [--accepted <file>] [--budget <lines>]
// It stops at the first failure with exit 1 and a REASON line: the PR head is not <head>, a check on it is
// not green, the branch is behind main, the review bot has not reviewed <head>, or a bot thread is open.
// Exit 7 prints one TAP line per change that needs the owner: a CODEOWNERS path (read from main) or a rule
// CODEOWNERS can't express. Exit 0: ready to merge. It never merges and never marks the PR ready.
import { execFileSync } from 'node:child_process';
import { acceptedIds, botCommentBodies, gh, openThreads, reviewState, reviewThreads, type Thread } from './prBotReview.ts';

type Check = { name?: string; context?: string; status?: string; conclusion?: string | null; state?: string };
type PrFile = { filename: string; previous_filename?: string; status: string; additions: number; deletions: number; patch?: string };
export type Gate = {
  pr: { headRefOid: string; statusCheckRollup: Check[] };
  compare: { behind_by: number; main: string };
  botComments: string[];
  threads: Thread[];
  files: PrFile[];
  codeowners: string;
  // Base (main) and head text of each changed package.json and Cargo.toml; '' when the file is missing.
  contents: Record<string, { base: string; head: string }>;
};

const GREEN = ['SUCCESS', 'NEUTRAL', 'SKIPPED'];
// How each lockfile names a package; a name on an added line that no removed line has is a new dependency.
const LOCKFILES: Record<string, RegExp> = {
  'Cargo.lock': /^name = "([^"]+)"$/,
  'pnpm-lock.yaml': /^ {2}'?((?:@[^/'\s]+\/)?[^@'\s]+)@\S*:$/,
  'package-lock.json': /^\s*"(?:.*\/)?node_modules\/([^"]+)": \{$/,
  'pubspec.lock': /^ {2}(\w+):$/,
};
// Added lines that skip or narrow tests, by file type.
const SKIPS: Array<[RegExp, RegExp]> = [
  [/\.(ts|tsx|js|jsx|mjs|cjs|svelte)$/, /\.(skip|only)\(|\bit\.todo\(/],
  [/\.rs$/, /#\[ignore/],
  [/\.cs$/, /\[(Fact|Theory)\(Skip/],
  [/\.dart$/, /\bskip:/],
  [/\.(csproj|props|targets)$/, /<IsTestProject>false/],
];
const TEST_FILE = /(^|\/)(tests?|__tests__)\/|\.(test|spec)\.[^/]+$|_test\.\w+$|Tests?\.cs$/;

const basename = (path: string) => path.slice(path.lastIndexOf('/') + 1);
const short = (sha: string) => sha.slice(0, 9);

// gitignore-style matching for the patterns CODEOWNERS uses: a slash before the end anchors at the repo root,
// `**/` is any folder depth, `*` stays within one folder, and a pattern also covers everything under it.
export function protectedPattern(codeowners: string, path: string): string | undefined {
  return codeowners.split('\n').map((line) => line.trim().split(/\s+/)[0]).filter((p) => p && !p.startsWith('#')).find((pattern) => {
    const anchored = pattern.replace(/\/$/, '').includes('/');
    const body = pattern.replace(/^\/|\/$/g, '').replace(/[.+^${}()|[\]\\]/g, '\\$&')
      .replace(/\*\*\/|\*\*|\*/g, (glob) => ({ '**/': '(?:.*/)?', '**': '.*', '*': '[^/]*' })[glob]!);
    return new RegExp(`^${anchored ? '' : '(?:.*/)?'}${body}(?:/.*)?$`).test(path);
  });
}

const changedKeys = (base: string, head: string, keys: string[]) => {
  const [before, after] = [base, head].map((text) => (text ? JSON.parse(text) : {}));
  return keys.filter((key) => JSON.stringify(before[key]) !== JSON.stringify(after[key]));
};

const guardedToml = (text: string) => {
  let keep = false;
  return text.split('\n').filter((line) => {
    const header = line.match(/^\s*\[\[?([^\]]+)\]/);
    if (header) keep = /^(profile(\..+)?|features|test)$/.test(header[1].trim());
    return keep;
  }).map((line) => line.trim()).join('\n');
};

function taps(gate: Gate, budget?: number): string[] {
  const out: string[] = [];
  for (const file of gate.files) {
    for (const path of new Set([file.filename, file.previous_filename ?? file.filename])) {
      const pattern = protectedPattern(gate.codeowners, path);
      if (pattern) out.push(`protected path ${path} (${pattern})`);
    }
    const lines = (file.patch ?? '').split('\n');
    const added = lines.filter((line) => line.startsWith('+')).map((line) => line.slice(1));
    const name = basename(file.filename);
    const lockName = LOCKFILES[name];
    // GitHub leaves out the patch of a very large diff; a file the rules read then can't be cleared.
    if (file.patch === undefined && file.additions > 0 && (lockName || SKIPS.some(([type]) => type.test(file.filename)))) {
      out.push(`diff too large to read: ${file.filename}`);
    }
    if (lockName) {
      const names = (sign: string) => lines.filter((line) => line.startsWith(sign)).map((line) => line.slice(1).match(lockName)?.[1]).filter(Boolean);
      const removed = new Set(names('-'));
      const fresh = [...new Set(names('+'))].filter((dep) => !removed.has(dep));
      if (fresh.length > 0) out.push(`new dependencies in ${file.filename}: ${fresh.join(', ')}`);
    }
    const contents = gate.contents[file.filename];
    if (name === 'package.json' && contents) {
      for (const key of changedKeys(contents.base, contents.head, ['scripts', 'engines'])) out.push(`${file.filename} changes "${key}"`);
    }
    if (name === 'Cargo.toml' && contents && guardedToml(contents.base) !== guardedToml(contents.head)) {
      out.push(`${file.filename} changes [profile], [features] or [[test]]`);
    }
    const before = file.previous_filename ?? file.filename;
    if ((file.status === 'removed' || file.status === 'renamed') && TEST_FILE.test(before)) out.push(`test file ${file.status}: ${before}`);
    for (const [type, skip] of SKIPS) {
      if (!type.test(file.filename)) continue;
      for (const line of added) if (skip.test(line)) out.push(`test skip added in ${file.filename}: ${line.trim()}`);
    }
  }
  const changed = gate.files.filter((file) => !LOCKFILES[basename(file.filename)]).reduce((sum, file) => sum + file.additions + file.deletions, 0);
  if (budget !== undefined && changed > 1.5 * budget) out.push(`${changed} changed lines, over 1.5x the budget of ${budget}`);
  return out;
}

export function evaluate(gate: Gate, head: string, accepted: string[], budget?: number): { exitCode: number; lines: string[] } {
  const fail = (reason: string) => ({ exitCode: 1, lines: [`REASON: ${reason}`] });
  if (gate.pr.headRefOid !== head) return fail(`head moved: PR head ${short(gate.pr.headRefOid)} is not ${short(head)}`);
  if (gate.pr.statusCheckRollup.length === 0) return fail('no checks on head');
  const red = gate.pr.statusCheckRollup.find((check) => !GREEN.includes(check.conclusion || check.state || ''));
  if (red) return fail(`check ${red.name ?? red.context} is ${red.conclusion || red.state || red.status}`);
  if (gate.compare.behind_by > 0) return fail(`behind main ${short(gate.compare.main)}`);
  const review = reviewState(gate.botComments);
  if (review?.commit_sha !== head || review.status !== 'completed') return fail(`the review bot has not reviewed ${short(head)}`);
  const open = openThreads(gate.threads, accepted);
  if (open.length > 0) return fail(`open bot threads ${open.map((thread) => thread.id).sort().join(' ')}`);
  const needed = taps(gate, budget);
  if (needed.length > 0) return { exitCode: 7, lines: [...needed.map((tap) => `TAP: ${tap}`), `REASON: ${needed.length} taps before merge`] };
  return { exitCode: 0, lines: [`Gate passed at ${short(head)}: nothing needs the owner.`] };
}

export function collect(prArg: string, head: string): Gate {
  const repo = gh(['repo', 'view', '--json', 'nameWithOwner', '--jq', '.nameWithOwner']).trim();
  const pr = JSON.parse(gh(['pr', 'view', prArg, '--json', 'number,headRefOid,baseRefName,statusCheckRollup']));
  const compare = JSON.parse(gh(['api', `repos/${repo}/compare/${pr.baseRefName}...${head}`, '--jq', '{behind_by: .behind_by, main: .base_commit.sha}']));
  // A missing file reads as ''; any other failure stops the gate, so an unread CODEOWNERS never clears a change.
  const read = (path: string, ref: string) => {
    try {
      return execFileSync('gh', ['api', `repos/${repo}/contents/${path}?ref=${ref}`, '-H', 'Accept: application/vnd.github.raw'],
        { encoding: 'utf8', maxBuffer: 16 * 1024 * 1024, stdio: ['ignore', 'pipe', 'pipe'] });
    } catch (error) {
      if (String((error as { stderr?: string }).stderr).includes('HTTP 404')) return '';
      process.stderr.write(String((error as { stderr?: string }).stderr));
      throw error;
    }
  };
  const files = (JSON.parse(gh(['api', '--paginate', '--slurp', `repos/${repo}/pulls/${pr.number}/files`])) as PrFile[][]).flat();
  const contents: Gate['contents'] = {};
  for (const file of files) {
    if (['package.json', 'Cargo.toml'].includes(basename(file.filename))) {
      contents[file.filename] = { base: read(file.previous_filename ?? file.filename, compare.main), head: read(file.filename, head) };
    }
  }
  return {
    pr, compare, files, contents,
    botComments: botCommentBodies(repo, pr.number),
    threads: reviewThreads(repo, pr.number),
    codeowners: read('CODEOWNERS', compare.main),
  };
}

function main(): void {
  const [prArg, head] = process.argv.slice(2);
  const at = process.argv.indexOf('--budget');
  const result = evaluate(collect(prArg, head), head, acceptedIds(process.argv), at < 0 ? undefined : Number(process.argv[at + 1]));
  for (const line of result.lines) console.log(line);
  process.exitCode = result.exitCode;
}

if (process.argv[1]?.endsWith('mergeGate.ts')) main();
