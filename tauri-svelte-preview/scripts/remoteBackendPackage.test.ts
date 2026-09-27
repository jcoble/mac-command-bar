import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { promisify } from 'node:util';

import {
  REMOTE_BACKEND_ADAPTER_FILES,
  REMOTE_BACKEND_INSTALL_SCRIPT,
  writeRemoteBackendPackage
} from './remoteBackendPackage.ts';

const execFileAsync = promisify(execFile);

async function writeAdapterFixture(directory: string): Promise<void> {
  await mkdir(directory);
  for (const file of REMOTE_BACKEND_ADAPTER_FILES) {
    await writeFile(path.join(directory, file), `${file}-bytes`);
  }
}

test('builds a deterministic remote backend package contract', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'assembly-remote-package-test-'));
  try {
    const input = path.join(root, 'input');
    const output = path.join(root, 'output');
    const extracted = path.join(root, 'extracted');
    await mkdir(input);
    await mkdir(extracted);
    await writeFile(path.join(input, 'server'), 'server-bytes');
    await writeAdapterFixture(path.join(input, 'adapters'));
    const archive = await writeRemoteBackendPackage({
      binaryPath: path.join(input, 'server'),
      adapterDirectory: path.join(input, 'adapters'),
      outputDirectory: output,
      version: '1.2.3',
      commit: '0123456789abcdef0123456789abcdef01234567'
    });
    const archiveBytes = await readFile(archive);
    const record = JSON.parse(await readFile(`${archive}.download.json`, 'utf8')) as {
      schemaVersion: number; version: string; protocolVersion: number; target: string;
      archiveFile: string; bytes: number; sha256: string;
    };
    assert.deepEqual(record, {
      schemaVersion: 1, version: '1.2.3', protocolVersion: 3,
      target: 'x86_64-unknown-linux-gnu', archiveFile: path.basename(archive),
      bytes: archiveBytes.length, sha256: createHash('sha256').update(archiveBytes).digest('hex')
    });
    await execFileAsync('tar', ['-xzf', archive, '-C', extracted]);
    const manifest = JSON.parse(await readFile(path.join(extracted, 'manifest.json'), 'utf8')) as {
      schemaVersion: number;
      packageType: string;
      target: string;
      commit: string;
      files: Array<{ path: string; sha256: string }>;
    };
    assert.equal(manifest.schemaVersion, 1);
    assert.equal(manifest.packageType, 'assembly-remote-backend');
    assert.equal(manifest.target, 'x86_64-unknown-linux-gnu');
    assert.equal(manifest.commit, '0123456789abcdef0123456789abcdef01234567');
    assert.equal(manifest.files.length, 10);
    for (const file of manifest.files) {
      const digest = createHash('sha256').update(await readFile(path.join(extracted, file.path))).digest('hex');
      assert.equal(digest, file.sha256);
    }
    assert.equal(await readFile(path.join(extracted, 'install.sh'), 'utf8'), REMOTE_BACKEND_INSTALL_SCRIPT);
    assert.match(REMOTE_BACKEND_INSTALL_SCRIPT, /sha256sum -c SHA256SUMS/);
    assert.match(REMOTE_BACKEND_INSTALL_SCRIPT, /ASSEMBLY_SERVER_BIND=127\.0\.0\.1:7777/);
    assert.match(REMOTE_BACKEND_INSTALL_SCRIPT, /systemctl --user restart assembly-remote\.service/);
    assert.match(REMOTE_BACKEND_INSTALL_SCRIPT, /systemctl --user stop assembly-remote\.service/);
    assert.match(REMOTE_BACKEND_INSTALL_SCRIPT, /backend-manifest\.json/);
    assert.match(REMOTE_BACKEND_INSTALL_SCRIPT, /\.assembly-remote-server\.new/);
    assert.match(REMOTE_BACKEND_INSTALL_SCRIPT, /\.local\/bin\/assembly-adapters/);
    assert.doesNotMatch(REMOTE_BACKEND_INSTALL_SCRIPT, /\.local\/bin\/adapters/);
    assert.match(REMOTE_BACKEND_INSTALL_SCRIPT, /claude-agent-acp claude-agent-acp-runtime/);
    assert.match(REMOTE_BACKEND_INSTALL_SCRIPT, /payload\/adapters\/\$adapter/);
    assert.doesNotMatch(REMOTE_BACKEND_INSTALL_SCRIPT, /command -v claude-agent-acp/);
    assert.doesNotMatch(REMOTE_BACKEND_INSTALL_SCRIPT, /command in node codex/);
    assert.match(REMOTE_BACKEND_INSTALL_SCRIPT, /\$4 != "127\.0\.0\.1:7777"/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test('rejects invalid release identity before writing', async () => {
  await assert.rejects(
    writeRemoteBackendPackage({
      binaryPath: '/missing',
      adapterDirectory: '/missing',
      outputDirectory: '/missing',
      version: '../bad',
      commit: 'not-a-commit'
    }),
    /Invalid backend version/
  );
});

test('remote download verifier rejects changed, truncated, and unexpected archives', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'assembly-remote-download-test-'));
  try {
    const input = path.join(root, 'input');
    const output = path.join(root, 'output');
    await mkdir(input);
    await writeFile(path.join(input, 'server'), 'server-bytes');
    await writeAdapterFixture(path.join(input, 'adapters'));
    const archive = await writeRemoteBackendPackage({
      binaryPath: path.join(input, 'server'), adapterDirectory: path.join(input, 'adapters'),
      outputDirectory: output, version: '1.2.3', commit: '0123456789abcdef0123456789abcdef01234567'
    });
    const rust = await readFile(new URL('../src-tauri/src/agent_conversation/remote_install.rs', import.meta.url), 'utf8');
    const script = rust.match(/const REMOTE_DOWNLOAD: &str = r#"([\s\S]*?)"#;/)?.[1];
    assert.ok(script, 'remote verifier script exists');
    const run = async (source: string, bytes: number, digest: string) => {
      const destination = path.join(root, `stage-${crypto.randomUUID()}`);
      await mkdir(destination);
      const wrapper = `import os, urllib.request\nclass Source:\n def __init__(self, path):\n  self.file=open(path,'rb'); self.headers={'Content-Length':str(os.path.getsize(path))}\n def __enter__(self): return self\n def __exit__(self,*args): self.file.close()\n def geturl(self): return 'https://github.com/jcoble/mac-command-bar/releases/download/test/package.tar.gz'\n def read(self,n): return self.file.read(n)\nurllib.request.urlopen=lambda request,timeout:Source(${JSON.stringify(source)})\n`;
      try {
        const { stdout } = await execFileAsync('python3', ['-c', wrapper + script, 'https://github.com/jcoble/mac-command-bar/releases/download/test/package.tar.gz', path.join(destination, 'package.tar.gz'), String(bytes), digest, '1.2.3']);
        return stdout;
      } finally {
        await rm(destination, { recursive: true, force: true });
      }
    };
    const archiveBytes = await readFile(archive);
    const digest = createHash('sha256').update(archiveBytes).digest('hex');
    assert.match(await run(archive, archiveBytes.length, digest), /V\t1\.2\.3\t/);
    await assert.rejects(run(archive, archiveBytes.length, '0'.repeat(64)), /signed digest/);
    await assert.rejects(run(archive, archiveBytes.length + 1, digest), /signed record/);
    const unpacked = path.join(root, 'unpacked');
    await mkdir(unpacked);
    await execFileAsync('tar', ['-xzf', archive, '-C', unpacked]);
    await writeFile(path.join(unpacked, 'unexpected'), 'extra');
    const unexpected = path.join(root, 'unexpected.tar.gz');
    await execFileAsync('tar', ['-czf', unexpected, '-C', unpacked, '.']);
    const unexpectedBytes = await readFile(unexpected);
    await assert.rejects(run(unexpected, unexpectedBytes.length, createHash('sha256').update(unexpectedBytes).digest('hex')), /Unexpected backend archive entry/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
