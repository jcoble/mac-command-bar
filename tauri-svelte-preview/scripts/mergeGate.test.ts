import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { chmodSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { collect, evaluate, type Gate } from './mergeGate.ts';

// gh output recorded from PR #473 (one Svelte file, green checks, reviewed by the bot), served by a fake gh
// that prints the first recorded answer whose key appears in its arguments. CODEOWNERS is the repo's own.
const recorded = JSON.parse(readFileSync(join(import.meta.dirname, 'fixtures/mergeGate/pr473.json'), 'utf8'));
const [outdated, agentResolved] = JSON.parse(readFileSync(join(import.meta.dirname, 'fixtures/mergeGate/threads.json'), 'utf8')).threads;
const HEAD = recorded['pr view'].headRefOid as string;
const MAIN = recorded['/compare/'].main as string;

function fakeGh(answers: Record<string, unknown>): Record<string, string> {
  const dir = mkdtempSync(join(tmpdir(), 'merge-gate-'));
  writeFileSync(join(dir, 'answers.json'), JSON.stringify(answers));
  writeFileSync(join(dir, 'gh'), `#!${process.execPath}
const answers = require(${JSON.stringify(join(dir, 'answers.json'))});
const args = process.argv.slice(2).join(' ');
const key = Object.keys(answers).find((k) => args.includes(k));
if (key === undefined) { console.error('gh: Not Found (HTTP 404)'); process.exit(1); }
process.stdout.write(typeof answers[key] === 'string' ? answers[key] : JSON.stringify(answers[key]));
`);
  chmodSync(join(dir, 'gh'), 0o755);
  return { ...process.env, PATH: `${dir}:${process.env.PATH}` } as Record<string, string>;
}

const answers = { 'repo view': 'jcoble/mac-command-bar', 'contents/CODEOWNERS': readFileSync(join(import.meta.dirname, '../../CODEOWNERS'), 'utf8'), ...recorded };

function cleanGate(): Gate {
  const path = process.env.PATH;
  process.env.PATH = fakeGh(answers).PATH;
  try { return collect('473', HEAD); } finally { process.env.PATH = path; }
}

const reason = (result: { lines: string[] }) => result.lines.at(-1);
const file = (filename: string, patch: string, status = 'modified') =>
  ({ filename, status, additions: patch.split('\n').filter((l) => l.startsWith('+')).length, deletions: 0, patch });

function withFile(entry: ReturnType<typeof file> & { previous_filename?: string }, contents?: { base: string; head: string }): Gate {
  const gate = cleanGate();
  gate.files.push(entry);
  if (contents) gate.contents[entry.filename] = contents;
  return gate;
}

function assertTap(gate: Gate, tap: RegExp): void {
  const result = evaluate(gate, HEAD, []);
  assert.equal(result.exitCode, 7, result.lines.join('\n'));
  assert.match(result.lines.join('\n'), tap);
  assert.equal(reason(result), `REASON: ${result.lines.filter((l) => l.startsWith('TAP: ')).length} taps before merge`);
}

function run(args: string[], extra: Record<string, unknown> = {}) {
  return spawnSync(process.execPath, ['--experimental-strip-types', join(import.meta.dirname, 'mergeGate.ts'), ...args],
    { env: fakeGh({ ...answers, ...extra }), encoding: 'utf8' });
}

test('a clean PR passes with exit 0', () => {
  const result = run(['473', HEAD]);
  assert.equal(result.status, 0, result.stdout + result.stderr);
  assert.deepEqual(evaluate(cleanGate(), HEAD, []).exitCode, 0);
});

test('head moved: the PR head is not the build head', () => {
  const result = run(['473', 'f00dfeed']);
  assert.equal(result.status, 1);
  assert.equal(result.stdout.trim().split('\n').at(-1), `REASON: head moved: PR head ${HEAD.slice(0, 9)} is not f00dfeed`);
});

test('a red check stops the gate and names the check', () => {
  const gate = cleanGate();
  gate.pr.statusCheckRollup[0] = { ...gate.pr.statusCheckRollup[0], conclusion: 'FAILURE' };
  assert.deepEqual(evaluate(gate, HEAD, []), { exitCode: 1, lines: ['REASON: check lifecycle-checks is FAILURE'] });
  gate.pr.statusCheckRollup = [];
  assert.equal(reason(evaluate(gate, HEAD, [])), 'REASON: no checks on head');
});

test('behind main', () => {
  const gate = cleanGate();
  gate.compare.behind_by = 3;
  assert.deepEqual(evaluate(gate, HEAD, []), { exitCode: 1, lines: [`REASON: behind main ${MAIN.slice(0, 9)}`] });
});

test('the bot has not reviewed the head', () => {
  const gate = cleanGate();
  gate.botComments = [];
  assert.equal(reason(evaluate(gate, HEAD, [])), `REASON: the review bot has not reviewed ${HEAD.slice(0, 9)}`);
});

test('an agent-resolved thread still counts as open; a thread in accepted is closed', () => {
  const gate = cleanGate();
  gate.threads = [agentResolved];
  assert.deepEqual(evaluate(gate, HEAD, []), { exitCode: 1, lines: [`REASON: open bot threads ${agentResolved.id}`] });
  assert.equal(evaluate(gate, HEAD, [agentResolved.id]).exitCode, 0);
});

test('an outdated thread is closed', () => {
  const gate = cleanGate();
  gate.threads = [outdated];
  assert.equal(evaluate(gate, HEAD, []).exitCode, 0);
});

test('workflow/mod.rs change hits CODEOWNERS', () => {
  assertTap(withFile(file('tauri-svelte-preview/src-tauri/src/workflow/mod.rs', '+let x = 1;')),
    /^TAP: protected path tauri-svelte-preview\/src-tauri\/src\/workflow\/mod\.rs \(\/tauri-svelte-preview\/src-tauri\/src\/workflow\/\)$/m);
});

test('a new .cargo/config.toml hits CODEOWNERS', () => {
  assertTap(withFile(file('core/.cargo/config.toml', '+[build]', 'added')), /^TAP: protected path core\/\.cargo\/config\.toml \(\*\*\/\.cargo\/\)$/m);
});

test('a package.json scripts change is a tap; a dependency version change is not', () => {
  const base = JSON.stringify({ scripts: { test: 'node a.test.ts' }, dependencies: { a: '1.0.0' } });
  assertTap(withFile(file('package.json', '+x'), { base, head: JSON.stringify({ scripts: { test: 'true' }, dependencies: { a: '1.0.0' } }) }),
    /^TAP: package\.json changes "scripts"$/m);
  assert.equal(evaluate(withFile(file('package.json', '+x'), { base, head: base.replace('1.0.0', '1.1.0') }), HEAD, []).exitCode, 0);
});

test('a Cargo.toml [features] change is a tap', () => {
  assertTap(withFile(file('core/Cargo.toml', '+x'), { base: '[package]\nname = "a"\n', head: '[package]\nname = "a"\n[features]\ndefault = []\n' }),
    /^TAP: core\/Cargo\.toml changes \[profile\], \[features\] or \[\[test\]\]$/m);
});

test('an added .skip( in a test file is a tap; an iterator .skip( in Rust is not', () => {
  assertTap(withFile(file('src/lib/a.test.ts', "+test.skip('x', () => {});")), /^TAP: test skip added in src\/lib\/a\.test\.ts: test\.skip\('x', \(\) => \{\}\);$/m);
  assert.equal(evaluate(withFile(file('core/src/a.rs', '+let rest = parts.iter().skip(1);')), HEAD, []).exitCode, 0);
});

test('[Fact(Skip is a tap', () => {
  assertTap(withFile(file('Tests/AppTests.cs', '+    [Fact(Skip = "flaky")]')), /^TAP: test skip added in Tests\/AppTests\.cs/m);
});

test('Dart skip: is a tap', () => {
  assertTap(withFile(file('mobile/test/runs_test.dart', "+  testWidgets('x', (t) async {}, skip: true);")), /^TAP: test skip added in mobile\/test\/runs_test\.dart/m);
});

test('<IsTestProject>false is a tap', () => {
  assertTap(withFile(file('Tests/App.Tests.csproj', '+    <IsTestProject>false</IsTestProject>')), /^TAP: test skip added in Tests\/App\.Tests\.csproj/m);
});

test('a deleted or renamed test file is a tap', () => {
  assertTap(withFile(file('src/lib/a.test.ts', '', 'removed')), /^TAP: test file removed: src\/lib\/a\.test\.ts$/m);
  assertTap(withFile({ ...file('src/lib/b.ts', '', 'renamed'), previous_filename: 'src/lib/b.test.ts' }), /^TAP: test file renamed: src\/lib\/b\.test\.ts$/m);
});

test('a new dependency in a lockfile is a tap; a version bump is not', () => {
  assertTap(withFile(file('tauri-svelte-preview/pnpm-lock.yaml', "+  'left-pad@1.3.0':\n+    resolution: {integrity: sha512-x}")),
    /^TAP: new dependencies in tauri-svelte-preview\/pnpm-lock\.yaml: left-pad$/m);
  assertTap(withFile(file('core/Cargo.lock', '+[[package]]\n+name = "evil"\n+version = "0.1.0"\n+[[package]]\n+name = "worse"')), /^TAP: new dependencies in core\/Cargo\.lock: evil, worse$/m);
  const bump = file('tauri-svelte-preview/pnpm-lock.yaml', '-  bits-ui@2.18.1:\n+  bits-ui@2.18.2:');
  assert.equal(evaluate(withFile(bump), HEAD, []).exitCode, 0);
});

test('over 1.5x the line budget is a tap', () => {
  const gate = withFile({ ...file('src/lib/big.ts', '+x'), additions: 200, deletions: 30 });
  assert.equal(evaluate(gate, HEAD, [], 160).exitCode, 0);
  const result = evaluate(gate, HEAD, [], 150);
  assert.equal(result.exitCode, 7);
  assert.match(result.lines.join('\n'), /^TAP: 232 changed lines, over 1\.5x the budget of 150$/m);
});

test('several taps print one line each, then the count', () => {
  const result = run(['473', HEAD, '--budget', '1'], {
    '/pulls/': [[{ filename: 'tauri-svelte-preview/src-tauri/src/workflow/mod.rs', status: 'modified', additions: 5, deletions: 0, patch: '+x' }]],
  });
  assert.equal(result.status, 7, result.stdout + result.stderr);
  assert.deepEqual(result.stdout.trim().split('\n').slice(-3), [
    'TAP: protected path tauri-svelte-preview/src-tauri/src/workflow/mod.rs (/tauri-svelte-preview/src-tauri/src/workflow/)',
    'TAP: 5 changed lines, over 1.5x the budget of 1',
    'REASON: 2 taps before merge',
  ]);
});
