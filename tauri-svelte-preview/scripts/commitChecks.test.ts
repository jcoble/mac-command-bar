import assert from 'node:assert/strict';
import test from 'node:test';
import { checkFiles, checkFixCommit } from './commitChecks.ts';

const empty = { fileSize: {}, providerBranches: {} };
const lines = (count: number) => 'const value = 1;\n'.repeat(count);
const file = (source: string, path = 'src/example.ts', addedLines: number[] = []) => ({ path, source, addedLines });

test('new source files have a 600-line limit', () => {
  assert.deepEqual(checkFiles([file(lines(600))], empty), []);
  assert.match(checkFiles([file(lines(601))], empty)[0], /src\/example.ts.*600/);
});

test('baselined source files cannot grow and shrinking asks for a lower baseline', () => {
  const baseline = { ...empty, fileSize: { 'src/example.ts': 800 } };
  assert.deepEqual(checkFiles([file(lines(800))], baseline), []);
  assert.match(checkFiles([file(lines(801))], baseline)[0], /800/);
  assert.match(checkFiles([file(lines(799))], baseline)[0], /lower.*799/);
});

test('generated files and build outputs are excluded from size checks', () => {
  for (const path of ['node_modules/a.ts', 'target/a.rs', 'build/a.svelte', 'generated/a.ts', 'src/a.generated.ts']) {
    assert.deepEqual(checkFiles([file(lines(601), path)], empty), []);
  }
  assert.deepEqual(checkFiles([file('// @generated\n' + lines(601))], empty), []);
});

test('provider comparisons are confined to the adapter folder', () => {
  const source = "if (provider === 'codex') run();";
  assert.match(checkFiles([file(source)], empty)[0], /provider.*0/);
  assert.deepEqual(checkFiles([file(source, 'tauri-svelte-preview/src-tauri/src/agent_conversation/providers/acp.rs')], empty), []);
});

test('existing provider branch counts can stay the same or shrink', () => {
  const baseline = { ...empty, providerBranches: { 'src/example.ts': 1 } };
  const branch = "if (provider !== 'claude') run();";
  assert.deepEqual(checkFiles([file(branch)], baseline), []);
  assert.deepEqual(checkFiles([file('run();')], baseline), []);
  assert.match(checkFiles([file(branch + '\n' + branch)], baseline)[0], /provider.*1/);
});

test('provider strings, switch cases and Rust variants are recognized', () => {
  for (const source of [
    'if (provider == "claude") {}',
    "if ('codex' === provider) {}",
    "case 'antigravity': run();",
    'AgentConversationProvider::Codex => run(),',
    'AgentConversationProvider::Claude | AgentConversationProvider::Antigravity => run(),',
    'if provider != AgentConversationProvider::Antigravity {}',
  ]) assert.equal(checkFiles([file(source)], empty).length, 1, source);
  assert.deepEqual(checkFiles([file("const provider = 'codex';")], empty), []);
});

test('provider checks ignore test files and Rust test modules', () => {
  const branch = "if (provider === 'codex') run();";
  assert.deepEqual(checkFiles([file(branch, 'src/a.test.ts'), file(branch, 'tests/a.ts')], empty), []);
  assert.deepEqual(checkFiles([file('#[cfg(test)]\nmod tests {\n' + branch + '\n}', 'src/a.rs')], empty), []);
});

test('production code after a Rust test-only item is still checked', () => {
  const later = "if provider == \"codex\" { run(); }";
  const helper = 'fn root() {\n  #[cfg(test)]\n  let path = test_root();\n}\n' + later;
  assert.equal(checkFiles([file(helper, 'src/a.rs')], empty).length, 1);
  const module = '#[cfg(test)]\nmod tests {\n  fn t() { let x = "{"; }\n}\n' + later;
  assert.equal(checkFiles([file(module, 'src/a.rs')], empty).length, 1);
});

test('fix subjects require a changed test or an explicit reason', () => {
  for (const subject of ['fix: repair', 'FIX(core): repair', 'Fix the crash']) {
    assert.match(checkFixCommit(subject, [file('const x = 1;')])[0], /test/);
    assert.deepEqual(checkFixCommit(subject, [file('test();', 'src/a.test.ts')]), []);
    assert.deepEqual(checkFixCommit(subject, [file('test();', 'tests/a.rs')]), []);
    assert.deepEqual(checkFixCommit(subject, [file('test();', 'mobile/test/chat_test.dart')]), []);
  }
  assert.deepEqual(checkFixCommit('feat: add feature', []), []);
  assert.deepEqual(checkFixCommit('fix: docs\n\nNo-test: docs', []), ['No-test: docs']);
  assert.match(checkFixCommit('fix: repair\n\nNo-test: ', [])[0], /test/);
});

test('Rust fixes accept added test attributes and changed code inside test modules', () => {
  for (const attribute of ['#[test]', '#[tokio::test]']) {
    assert.deepEqual(checkFixCommit('fix: repair', [file(attribute + '\nfn works() {}', 'src/a.rs', [1, 2])]), []);
  }
  const source = 'fn production() {}\n#[cfg(test)]\nmod tests {\n  fn works() {\n    assert!(true);\n  }\n}\nfn other() {}';
  assert.deepEqual(checkFixCommit('fix: repair', [file(source, 'src/a.rs', [5])]), []);
  assert.match(checkFixCommit('fix: repair', [file(source, 'src/a.rs', [1])])[0], /test/);
  assert.match(checkFixCommit('fix: repair', [file(source, 'src/a.rs', [8])])[0], /test/);
});

test('existing JavaScript suites and any test-gated Rust module count as tests', () => {
  for (const path of ['scripts/parseUnifiedDiff.test.mjs', 'scripts/a.test.js', 'scripts/a.test.mts']) {
    assert.deepEqual(checkFixCommit('fix: repair', [file('test();', path)]), []);
  }
  const source = 'fn production() {}\n#[cfg(test)]\nmod contract_tests {\n  fn works() {\n    assert!(true);\n  }\n}';
  assert.deepEqual(checkFixCommit('fix: repair', [file(source, 'src/a.rs', [5])]), []);
});
