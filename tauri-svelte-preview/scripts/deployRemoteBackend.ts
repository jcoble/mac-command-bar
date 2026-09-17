import { execFile } from 'node:child_process';
import { createHash, createPublicKey, randomUUID, verify } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { lstat, mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

import {
  REMOTE_BACKEND_PAYLOAD_FILES,
  REMOTE_BACKEND_TARGET,
  type RemoteBackendManifest
} from './remoteBackendPackage.ts';

const execFileAsync = promisify(execFile);
const archiveEntries = new Set([
  'manifest.json',
  'SHA256SUMS',
  ...REMOTE_BACKEND_PAYLOAD_FILES
]);
const archiveDirectories = new Set(['payload', 'payload/adapters']);

export type RemoteBackendPackageInspection = {
  manifest: RemoteBackendManifest;
  archivePath: string;
};

const ed25519SpkiPrefix = Buffer.from('302a300506032b6570032100', 'hex');

function normalizedArchiveEntry(entry: string): string {
  return entry.replace(/^\.\//, '').replace(/\/$/, '');
}

export function validateSshTarget(target: string): string {
  const value = target.trim();
  if (!value || value.startsWith('-') || !/^[0-9A-Za-z._@-]+$/.test(value)) {
    throw new Error("SSH destination may contain only letters, numbers, '.', '_', '@', and '-'");
  }
  return value;
}

async function sha256(filePath: string): Promise<string> {
  return createHash('sha256').update(await readFile(filePath)).digest('hex');
}

async function blake2b512(filePath: string): Promise<Buffer> {
  const hash = createHash('blake2b512');
  for await (const chunk of createReadStream(filePath)) hash.update(chunk as Buffer);
  return hash.digest();
}

function configuredPublicKeyPacket(encodedPublicKey: string): Buffer {
  const decoded = Buffer.from(encodedPublicKey.trim(), 'base64').toString('utf8').trim().split('\n');
  if (decoded.length !== 2 || !decoded[0].startsWith('untrusted comment:')) {
    throw new Error('Remote backend signing public key is invalid');
  }
  const packet = Buffer.from(decoded[1], 'base64');
  if (packet.length !== 42 || !['Ed', 'ED'].includes(packet.subarray(0, 2).toString('ascii'))) {
    throw new Error('Remote backend signing public key is invalid');
  }
  return packet;
}

export async function verifyRemoteBackendSignature(
  archivePath: string,
  signaturePath: string,
  encodedPublicKey: string
): Promise<void> {
  const publicPacket = configuredPublicKeyPacket(encodedPublicKey);
  const encodedSignature = (await readFile(signaturePath, 'utf8')).trim();
  const lines = Buffer.from(encodedSignature, 'base64').toString('utf8').trim().split('\n');
  if (lines.length !== 4 || !lines[0].startsWith('untrusted comment:') || !lines[2].startsWith('trusted comment: ')) {
    throw new Error('Remote backend package signature is invalid');
  }
  const signaturePacket = Buffer.from(lines[1], 'base64');
  const globalSignature = Buffer.from(lines[3], 'base64');
  if (signaturePacket.length !== 74 || signaturePacket.subarray(0, 2).toString('ascii') !== 'ED' || globalSignature.length !== 64) {
    throw new Error('Remote backend package signature is invalid');
  }
  if (!publicPacket.subarray(2, 10).equals(signaturePacket.subarray(2, 10))) {
    throw new Error('Remote backend package signature uses an unexpected key');
  }
  const publicKey = createPublicKey({
    key: Buffer.concat([ed25519SpkiPrefix, publicPacket.subarray(10)]),
    format: 'der',
    type: 'spki'
  });
  const signature = signaturePacket.subarray(10);
  if (!verify(null, await blake2b512(archivePath), publicKey, signature)) {
    throw new Error('Remote backend package signature verification failed');
  }
  const trustedComment = lines[2].slice('trusted comment: '.length);
  if (!verify(null, Buffer.concat([signature, Buffer.from(trustedComment)]), publicKey, globalSignature)) {
    throw new Error('Remote backend package trusted comment verification failed');
  }
}

function validateManifest(value: unknown): RemoteBackendManifest {
  if (!value || typeof value !== 'object') throw new Error('Remote backend manifest is missing');
  const manifest = value as Partial<RemoteBackendManifest>;
  if (manifest.schemaVersion !== 1 || manifest.packageType !== 'assembly-remote-backend') {
    throw new Error('Remote backend manifest is not supported');
  }
  if (manifest.target !== REMOTE_BACKEND_TARGET) {
    throw new Error(`Unsupported remote backend target: ${String(manifest.target)}`);
  }
  if (!/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(manifest.version ?? '')) {
    throw new Error('Remote backend manifest has an invalid version');
  }
  if (!/^[0-9a-f]{7,40}$/.test(manifest.commit ?? '')) {
    throw new Error('Remote backend manifest has an invalid commit');
  }
  if (!Array.isArray(manifest.files) || manifest.files.length !== REMOTE_BACKEND_PAYLOAD_FILES.length) {
    throw new Error('Remote backend manifest has an invalid file list');
  }
  const paths = new Set(manifest.files.map((file) => file.path));
  for (const expected of REMOTE_BACKEND_PAYLOAD_FILES) {
    if (!paths.has(expected)) throw new Error(`Remote backend manifest is missing ${expected}`);
  }
  for (const file of manifest.files) {
    if (!archiveEntries.has(file.path) || !/^[0-9a-f]{64}$/.test(file.sha256)) {
      throw new Error(`Remote backend manifest has an invalid file entry: ${file.path}`);
    }
  }
  return manifest as RemoteBackendManifest;
}

export async function inspectRemoteBackendPackage(
  archivePath: string,
  signaturePath: string,
  encodedPublicKey: string
): Promise<RemoteBackendPackageInspection> {
  const resolved = path.resolve(archivePath);
  if (!resolved.endsWith('.tar.gz')) throw new Error('Remote backend package must be a .tar.gz archive');
  await verifyRemoteBackendSignature(resolved, path.resolve(signaturePath), encodedPublicKey);
  const { stdout: listing } = await execFileAsync('tar', ['-tzf', resolved]);
  const entries = listing.split('\n').map(normalizedArchiveEntry).filter(Boolean);
  const files = entries.filter((entry) => !archiveDirectories.has(entry));
  if (files.length !== archiveEntries.size || files.some((entry) => !archiveEntries.has(entry))) {
    throw new Error('Remote backend package contains unexpected paths');
  }
  const { stdout: verboseListing } = await execFileAsync('tar', ['-tvzf', resolved]);
  if (verboseListing.split('\n').filter(Boolean).some((entry) => !['-', 'd'].includes(entry[0]))) {
    throw new Error('Remote backend package contains a non-file archive entry');
  }
  const extracted = await mkdtemp(path.join(tmpdir(), 'assembly-remote-inspect-'));
  try {
    await execFileAsync('tar', ['-xzf', resolved, '-C', extracted]);
    const manifest = validateManifest(JSON.parse(await readFile(path.join(extracted, 'manifest.json'), 'utf8')));
    for (const expected of archiveEntries) {
      if (!(await lstat(path.join(extracted, expected))).isFile()) {
        throw new Error(`Remote backend package entry is not a regular file: ${expected}`);
      }
    }
    for (const file of manifest.files) {
      if (await sha256(path.join(extracted, file.path)) !== file.sha256) {
        throw new Error(`Remote backend package checksum failed: ${file.path}`);
      }
    }
    return { manifest, archivePath: resolved };
  } finally {
    await rm(extracted, { recursive: true, force: true });
  }
}

async function run(command: string, args: string[], timeout: number): Promise<string> {
  const { stdout } = await execFileAsync(command, args, { timeout, maxBuffer: 4 * 1024 * 1024 });
  return stdout.trim();
}

function sshArgs(target: string, command: string): string[] {
  return [
    '-o', 'BatchMode=yes',
    '-o', 'ConnectTimeout=10',
    '-o', 'ControlMaster=no',
    '-o', 'ControlPath=none',
    '--', target, command
  ];
}

export async function deployRemoteBackend(
  sshTarget: string,
  archivePath: string,
  signaturePath: string,
  encodedPublicKey: string,
  onPhase: (message: string) => void = console.log
): Promise<RemoteBackendPackageInspection> {
  const target = validateSshTarget(sshTarget);
  onPhase('1/5 Verifying the local backend package');
  const inspected = await inspectRemoteBackendPackage(archivePath, signaturePath, encodedPublicKey);

  onPhase('2/5 Checking the remote operating system');
  const platform = await run('ssh', sshArgs(target, 'printf "%s %s" "$(uname -s)" "$(uname -m)"'), 15_000);
  if (platform !== 'Linux x86_64') throw new Error(`Unsupported remote platform: ${platform}`);

  const stagingName = `.assembly-install-${inspected.manifest.commit.slice(0, 12)}-${randomUUID()}`;
  const stagingPath = `/tmp/${stagingName}`;
  const remoteArchive = `${stagingPath}/package.tar.gz`;
  onPhase('3/5 Uploading the prebuilt backend package');
  await run('ssh', sshArgs(target, `install -d -m 700 '${stagingPath}'`), 15_000);
  try {
    await run(
      'scp',
      ['-q', '-o', 'BatchMode=yes', '-o', 'ConnectTimeout=10', '-o', 'ControlMaster=no', '-o', 'ControlPath=none', '--', inspected.archivePath, `${target}:${remoteArchive}`],
      120_000
    );
    onPhase('4/5 Installing and starting the backend');
    await run(
      'ssh',
      sshArgs(target, `set -eu; cd '${stagingPath}'; tar -xzf package.tar.gz; sh install.sh`),
      60_000
    );
    onPhase('5/5 Verifying the installed service and database');
    const receipt = await run(
      'ssh',
      sshArgs(target, `set -eu; systemctl --user is-active --quiet assembly-remote.service; test -f "$HOME/.local/share/assembly/sessions.db"; listeners=$(ss -ltnH 'sport = :7777'); test -n "$listeners"; printf '%s\\n' "$listeners" | awk '$4 != "127.0.0.1:7777" { exit 1 } END { if (NR == 0) exit 1 }'; sha256sum "$HOME/.local/bin/assembly-remote-server" | awk '{print $1}'`),
      15_000
    );
    const binary = inspected.manifest.files.find((file) => file.path === 'payload/assembly-remote-server');
    if (!binary || receipt !== binary.sha256) throw new Error('Installed backend checksum does not match the package');
    return inspected;
  } finally {
    await run('ssh', sshArgs(target, `find '${stagingPath}' -depth -delete 2>/dev/null || true`), 15_000).catch(() => undefined);
  }
}

async function main(): Promise<void> {
  const [sshTarget, archivePath, signaturePath] = process.argv.slice(2);
  if (!sshTarget || !archivePath || !signaturePath) {
    throw new Error('Usage: deployRemoteBackend.ts <ssh-target> <backend-package.tar.gz> <backend-package.tar.gz.sig>');
  }
  const projectRoot = fileURLToPath(new URL('../', import.meta.url));
  const config = JSON.parse(await readFile(path.join(projectRoot, 'src-tauri/tauri.conf.json'), 'utf8')) as {
    plugins?: { updater?: { pubkey?: unknown } };
  };
  const publicKey = config.plugins?.updater?.pubkey;
  if (typeof publicKey !== 'string') throw new Error('Assembly updater public key is unavailable');
  const result = await deployRemoteBackend(sshTarget, archivePath, signaturePath, publicKey);
  console.log(`Installed Assembly remote backend ${result.manifest.version} from ${result.manifest.commit}`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main();
}
