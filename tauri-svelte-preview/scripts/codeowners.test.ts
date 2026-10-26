import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { test } from 'node:test';
import { protectedPattern } from './mergeGate.ts';

const root = join(import.meta.dirname, '../..');
const codeowners = readFileSync(join(root, 'CODEOWNERS'), 'utf8');
const descriptors = JSON.parse(readFileSync(join(root, 'tauri-svelte-preview/src-tauri/src/agent_conversation/providers/descriptors.json'), 'utf8')).providers as Array<{ id: string }>;

// The repo paths each provider reads its settings and instructions from. The descriptors don't carry
// these yet, so a provider added to descriptors.json fails here until its paths are listed and protected.
const SETTINGS_PATHS: Record<string, string[]> = {
  codex: ['.codex/config.toml', 'AGENTS.md', 'tauri-svelte-preview/AGENTS.md'],
  claude: ['.claude/settings.json', '.mcp.json', 'CLAUDE.md', 'tauri-svelte-preview/CLAUDE.md'],
  antigravity: ['GEMINI.md', '.agents/rules.md', '.agent/workflows/a.md'],
};

test('every provider in descriptors.json has its settings and instruction files in CODEOWNERS', () => {
  for (const { id } of descriptors) {
    const paths = SETTINGS_PATHS[id];
    assert.ok(paths, `${id} is in descriptors.json; list its settings paths here and in CODEOWNERS`);
    for (const path of paths) assert.ok(protectedPattern(codeowners, path), `${path} (${id}) is not in CODEOWNERS`);
  }
});

test('the guard code, the release build and the checks are protected; ordinary app code is not', () => {
  for (const path of ['.assembly/workflows/task.yml', '.github/workflows/lifecycle-checks.yml', 'CODEOWNERS',
    'tauri-svelte-preview/src-tauri/src/workflow/mod.rs', 'tauri-svelte-preview/scripts/mergeGate.ts',
    'tauri-svelte-preview/src-tauri/tauri.conf.json', 'scripts/ci-checks.sh', 'core/.cargo/config.toml', 'rust-toolchain.toml']) {
    assert.ok(protectedPattern(codeowners, path), `${path} should be protected`);
  }
  for (const path of ['tauri-svelte-preview/src/lib/shell/board/BoardCards.svelte', 'core/src/lib.rs', 'docs/AGENTS-notes.md']) {
    assert.equal(protectedPattern(codeowners, path), undefined, `${path} should not be protected`);
  }
});
