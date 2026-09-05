import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { createInterface } from 'node:readline';
import { fileURLToPath } from 'node:url';

const adapterDir = new URL('../src-tauri/adapters/', import.meta.url);
const manifest = JSON.parse(await readFile(new URL('manifest.json', adapterDir), 'utf8'));
assert.equal(manifest.schemaVersion, 1);

async function sha256(path) {
  return createHash('sha256').update(await readFile(path)).digest('hex');
}

async function initialize(adapter) {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 15_000);
  const child = spawn(fileURLToPath(new URL(adapter.executable, adapterDir)), [], {
    signal: controller.signal,
    stdio: ['pipe', 'pipe', 'pipe']
  });
  child.on('error', () => {});
  child.stdin.write(`${JSON.stringify({
    jsonrpc: '2.0',
    id: 1,
    method: 'initialize',
    params: {
      protocolVersion: 1,
      clientCapabilities: {},
      clientInfo: { name: 'assembly-release-test', version: '0.1.0' }
    }
  })}\n`);

  let response = '';
  for await (const line of createInterface({ input: child.stdout })) {
    response = line;
    break;
  }
  clearTimeout(timeout);
  child.stdin.end();
  if (!child.killed) child.kill('SIGTERM');
  assert.ok(response, `${adapter.id} did not answer initialize`);
  return JSON.parse(response);
}

for (const adapter of manifest.adapters) {
  for (const file of adapter.files) {
    assert.equal(await sha256(new URL(file.path, adapterDir)), file.sha256);
  }
  const initialized = await initialize(adapter);
  assert.equal(initialized.result.agentInfo.version, adapter.version);
}

console.log('release adapter checks passed');
