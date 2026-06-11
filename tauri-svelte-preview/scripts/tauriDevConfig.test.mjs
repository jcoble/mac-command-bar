import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const packageJson = JSON.parse(await readFile(new URL('../package.json', import.meta.url), 'utf8'));
const defaultConfig = JSON.parse(await readFile(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8'));
const attachConfig = JSON.parse(await readFile(new URL('../src-tauri/tauri.dev.attach.conf.json', import.meta.url), 'utf8'));

assert.equal(defaultConfig.build.beforeDevCommand, 'pnpm dev');
assert.equal(defaultConfig.build.devUrl, 'http://localhost:5177');
assert.equal(packageJson.scripts['tauri:dev'], 'tauri dev');
assert.equal(
  packageJson.scripts['test:tauri-app'],
  'pnpm build && pnpm test:native-lsp && cargo build --manifest-path src-tauri/Cargo.toml',
  'expected a repeatable real Tauri app validation script'
);

const attachScript = packageJson.scripts['tauri:dev:attach'];
assert.ok(attachScript, 'expected a Tauri attach-mode dev script');
assert.match(attachScript, /^tauri dev\b/);
assert.match(attachScript, /--config src-tauri\/tauri\.dev\.attach\.conf\.json\b/);

assert.equal(attachConfig.build.beforeDevCommand, '');
assert.equal(attachConfig.build.devUrl, 'http://127.0.0.1:5177');
assert.equal(attachConfig.build.frontendDist, '../build');

console.log('tauri dev config checks passed');
