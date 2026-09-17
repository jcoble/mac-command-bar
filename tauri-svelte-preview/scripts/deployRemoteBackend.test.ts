import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { createHash, generateKeyPairSync, sign } from 'node:crypto';
import { mkdir, mkdtemp, readFile, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { promisify } from 'node:util';

import { inspectRemoteBackendPackage, validateSshTarget, verifyRemoteBackendSignature } from './deployRemoteBackend.ts';
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

async function signFixture(filePath: string, signaturePath: string): Promise<string> {
  const { publicKey, privateKey } = generateKeyPairSync('ed25519');
  const jwk = publicKey.export({ format: 'jwk' });
  if (!jwk.x) throw new Error('Generated Ed25519 key has no public value');
  const keyId = Buffer.from('0102030405060708', 'hex');
  const publicPacket = Buffer.concat([Buffer.from('Ed'), keyId, Buffer.from(jwk.x, 'base64url')]);
  const digest = createHash('blake2b512').update(await readFile(filePath)).digest();
  const signature = sign(null, digest, privateKey);
  const trustedComment = `timestamp:1\tfile:${path.basename(filePath)}\tprehashed`;
  const globalSignature = sign(null, Buffer.concat([signature, Buffer.from(trustedComment)]), privateKey);
  const signaturePacket = Buffer.concat([Buffer.from('ED'), keyId, signature]);
  const signatureText = [
    'untrusted comment: test signature',
    signaturePacket.toString('base64'),
    `trusted comment: ${trustedComment}`,
    globalSignature.toString('base64')
  ].join('\n');
  await writeFile(signaturePath, Buffer.from(`${signatureText}\n`).toString('base64'));
  return Buffer.from(`untrusted comment: test public key\n${publicPacket.toString('base64')}\n`).toString('base64');
}

test('accepts SSH aliases without allowing option or shell injection', () => {
  assert.equal(validateSshTarget('agent-workbox'), 'agent-workbox');
  assert.equal(validateSshTarget('person@server.example'), 'person@server.example');
  for (const target of ['', '-oProxyCommand=false', 'host name', 'host;false', '$(false)']) {
    assert.throws(() => validateSshTarget(target), /SSH destination/);
  }
});

test('the installer contract never accepts a non-loopback backend listener', () => {
  assert.match(REMOTE_BACKEND_INSTALL_SCRIPT, /ASSEMBLY_SERVER_BIND=127\.0\.0\.1:7777/);
  assert.match(REMOTE_BACKEND_INSTALL_SCRIPT, /\$4 != "127\.0\.0\.1:7777"/);
  assert.doesNotMatch(REMOTE_BACKEND_INSTALL_SCRIPT, /0\.0\.0\.0:7777|\[::\]:7777/);
});

test('inspects the exact package layout and verifies every payload hash', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'assembly-remote-deploy-test-'));
  try {
    const input = path.join(root, 'input');
    await mkdir(input);
    await writeFile(path.join(input, 'server'), 'server-bytes');
    await writeAdapterFixture(path.join(input, 'adapters'));
    const archive = await writeRemoteBackendPackage({
      binaryPath: path.join(input, 'server'),
      adapterDirectory: path.join(input, 'adapters'),
      outputDirectory: path.join(root, 'output'),
      version: '1.2.3',
      commit: '0123456789abcdef0123456789abcdef01234567'
    });
    const signature = `${archive}.sig`;
    const publicKey = await signFixture(archive, signature);
    await verifyRemoteBackendSignature(archive, signature, publicKey);
    const inspected = await inspectRemoteBackendPackage(archive, signature, publicKey);
    assert.equal(inspected.manifest.version, '1.2.3');
    assert.equal(inspected.manifest.files.length, 9);

    await writeFile(archive, Buffer.concat([await readFile(archive), Buffer.from('tampered')]));
    await assert.rejects(
      verifyRemoteBackendSignature(archive, signature, publicKey),
      /signature verification failed/
    );

    const badRoot = path.join(root, 'unexpected');
    await mkdir(badRoot);
    await writeFile(path.join(badRoot, 'unexpected.txt'), 'unexpected');
    const badArchive = path.join(root, 'unexpected.tar.gz');
    await execFileAsync('tar', ['-czf', badArchive, '-C', badRoot, '.']);
    const badSignature = `${badArchive}.sig`;
    const badPublicKey = await signFixture(badArchive, badSignature);
    await assert.rejects(inspectRemoteBackendPackage(badArchive, badSignature, badPublicKey), /unexpected paths/);

    const linkedRoot = path.join(root, 'linked');
    await mkdir(path.join(linkedRoot, 'payload'), { recursive: true });
    await writeFile(path.join(linkedRoot, 'manifest.json'), '{}');
    await writeFile(path.join(linkedRoot, 'SHA256SUMS'), '');
    await writeFile(path.join(linkedRoot, 'payload/assembly-remote-server'), 'server');
    await mkdir(path.join(linkedRoot, 'payload/adapters'));
    for (const file of REMOTE_BACKEND_ADAPTER_FILES) {
      await writeFile(path.join(linkedRoot, 'payload/adapters', file), file);
    }
    await symlink('manifest.json', path.join(linkedRoot, 'install.sh'));
    const linkedArchive = path.join(root, 'linked.tar.gz');
    await execFileAsync('tar', ['-czf', linkedArchive, '-C', linkedRoot, '.']);
    const linkedSignature = `${linkedArchive}.sig`;
    const linkedPublicKey = await signFixture(linkedArchive, linkedSignature);
    await assert.rejects(
      inspectRemoteBackendPackage(linkedArchive, linkedSignature, linkedPublicKey),
      /non-file archive entry/
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
