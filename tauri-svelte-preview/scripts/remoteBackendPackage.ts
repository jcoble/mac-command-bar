import { execFile } from 'node:child_process';
import { createHash } from 'node:crypto';
import { copyFile, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);

export const REMOTE_BACKEND_TARGET = 'x86_64-unknown-linux-gnu';

export type RemoteBackendManifest = {
  schemaVersion: 1;
  packageType: 'assembly-remote-backend';
  version: string;
  commit: string;
  target: typeof REMOTE_BACKEND_TARGET;
  files: Array<{ path: string; sha256: string; mode: '0755' | '0644' }>;
};

export type RemoteBackendPackageInput = {
  binaryPath: string;
  bridgePath: string;
  outputDirectory: string;
  version: string;
  commit: string;
};

export const REMOTE_BACKEND_INSTALL_SCRIPT = `#!/bin/sh
set -eu

[ "$(uname -s)" = "Linux" ] || { echo "Assembly remote backend requires Linux" >&2; exit 1; }
[ "$(uname -m)" = "x86_64" ] || { echo "Unsupported Linux architecture: $(uname -m)" >&2; exit 1; }
for command in node codex sha256sum systemctl openssl install ss awk; do
  command -v "$command" >/dev/null 2>&1 || { echo "Required command is missing: $command" >&2; exit 1; }
done
claude_path=$(command -v claude-agent-acp 2>/dev/null || true)
[ -n "$claude_path" ] || { echo "Required command is missing: claude-agent-acp" >&2; exit 1; }

sha256sum -c SHA256SUMS
install -d "$HOME/.local/bin" "$HOME/.local/share/assembly" "$HOME/.config/assembly" "$HOME/.config/systemd/user"
install -m 755 payload/assembly-remote-server "$HOME/.local/bin/.assembly-remote-server.new"
mv "$HOME/.local/bin/.assembly-remote-server.new" "$HOME/.local/bin/assembly-remote-server"
install -m 644 payload/codex-acp-bridge.mjs "$HOME/.local/share/assembly/.codex-acp-bridge.mjs.new"
mv "$HOME/.local/share/assembly/.codex-acp-bridge.mjs.new" "$HOME/.local/share/assembly/codex-acp-bridge.mjs"

codex_path=$(command -v codex)
node_path=$(command -v node)
cat >"$HOME/.local/bin/.assembly-codex-acp.new" <<EOF
#!/bin/sh
export CODEX_BIN="$codex_path"
exec "$node_path" "$HOME/.local/share/assembly/codex-acp-bridge.mjs" "\$@"
EOF
cat >"$HOME/.local/bin/.assembly-claude-acp.new" <<EOF
#!/bin/sh
exec "$claude_path" "\$@"
EOF
chmod 755 "$HOME/.local/bin/.assembly-codex-acp.new" "$HOME/.local/bin/.assembly-claude-acp.new"
mv "$HOME/.local/bin/.assembly-codex-acp.new" "$HOME/.local/bin/assembly-codex-acp"
mv "$HOME/.local/bin/.assembly-claude-acp.new" "$HOME/.local/bin/assembly-claude-acp"

token=$(sed -n 's/^ASSEMBLY_SERVER_TOKEN=//p' "$HOME/.config/assembly/server.env" 2>/dev/null || true)
[ "\${#token}" -ge 32 ] || token=$(openssl rand -hex 32)
codex_hash=$(sha256sum "$HOME/.local/bin/assembly-codex-acp" | awk '{print $1}')
claude_hash=$(sha256sum "$HOME/.local/bin/assembly-claude-acp" | awk '{print $1}')
cat >"$HOME/.config/assembly/.server.env.new" <<EOF
ASSEMBLY_SERVER_BIND=127.0.0.1:7777
ASSEMBLY_SERVER_TOKEN=$token
ASSEMBLY_SERVER_DATA_DIR=$HOME/.local/share/assembly
MCB_CODEX_ACP_PATH=$HOME/.local/bin/assembly-codex-acp
MCB_CODEX_ACP_SHA256=$codex_hash
MCB_CLAUDE_AGENT_ACP_PATH=$HOME/.local/bin/assembly-claude-acp
MCB_CLAUDE_AGENT_ACP_SHA256=$claude_hash
PATH=$(dirname "$node_path"):$(dirname "$codex_path"):$HOME/.local/bin:/usr/local/bin:/usr/bin:/bin
EOF
chmod 600 "$HOME/.config/assembly/.server.env.new"
mv "$HOME/.config/assembly/.server.env.new" "$HOME/.config/assembly/server.env"

cat >"$HOME/.config/systemd/user/.assembly-remote.service.new" <<EOF
[Unit]
Description=Assembly Remote Service
After=network.target

[Service]
Type=simple
EnvironmentFile=$HOME/.config/assembly/server.env
ExecStart=$HOME/.local/bin/assembly-remote-server --assembly-server
Restart=on-failure
RestartSec=2

[Install]
WantedBy=default.target
EOF
mv "$HOME/.config/systemd/user/.assembly-remote.service.new" "$HOME/.config/systemd/user/assembly-remote.service"
systemctl --user daemon-reload
systemctl --user enable assembly-remote.service
systemctl --user restart assembly-remote.service

attempt=0
while [ "$attempt" -lt 15 ]; do
  listeners=$(ss -ltnH 'sport = :7777')
  if systemctl --user is-active --quiet assembly-remote.service \
    && [ -n "$listeners" ] \
    && printf '%s\n' "$listeners" | awk '$4 != "127.0.0.1:7777" { exit 1 } END { if (NR == 0) exit 1 }'; then
    exit 0
  fi
  attempt=$((attempt + 1))
  sleep 1
done
systemctl --user status assembly-remote.service --no-pager --lines=20 >&2 || true
echo "Assembly remote backend did not become active within 15 seconds" >&2
exit 1
`;

async function sha256(filePath: string): Promise<string> {
  return createHash('sha256').update(await readFile(filePath)).digest('hex');
}

function validatePackageInput(input: RemoteBackendPackageInput): void {
  if (!/^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/.test(input.version)) {
    throw new Error(`Invalid backend version: ${input.version}`);
  }
  if (!/^[0-9a-f]{7,40}$/.test(input.commit)) {
    throw new Error(`Invalid Git commit: ${input.commit}`);
  }
}

export async function writeRemoteBackendPackage(input: RemoteBackendPackageInput): Promise<string> {
  validatePackageInput(input);
  const staging = await mkdtemp(path.join(tmpdir(), 'assembly-remote-package-'));
  const payload = path.join(staging, 'payload');
  const archiveName = `assembly-remote-backend-${input.version}-${REMOTE_BACKEND_TARGET}.tar.gz`;
  const archivePath = path.join(input.outputDirectory, archiveName);
  try {
    await mkdir(payload, { recursive: true });
    await mkdir(input.outputDirectory, { recursive: true });
    const binary = path.join(payload, 'assembly-remote-server');
    const bridge = path.join(payload, 'codex-acp-bridge.mjs');
    await copyFile(input.binaryPath, binary);
    await copyFile(input.bridgePath, bridge);
    const installScript = path.join(staging, 'install.sh');
    await writeFile(installScript, REMOTE_BACKEND_INSTALL_SCRIPT, { mode: 0o755 });
    const files: RemoteBackendManifest['files'] = [
      { path: 'payload/assembly-remote-server', sha256: await sha256(binary), mode: '0755' },
      { path: 'payload/codex-acp-bridge.mjs', sha256: await sha256(bridge), mode: '0644' },
      { path: 'install.sh', sha256: await sha256(installScript), mode: '0755' }
    ];
    const manifest: RemoteBackendManifest = {
      schemaVersion: 1,
      packageType: 'assembly-remote-backend',
      version: input.version,
      commit: input.commit,
      target: REMOTE_BACKEND_TARGET,
      files
    };
    await writeFile(path.join(staging, 'manifest.json'), `${JSON.stringify(manifest, null, 2)}\n`);
    await writeFile(
      path.join(staging, 'SHA256SUMS'),
      `${files.map((file) => `${file.sha256}  ${file.path}`).join('\n')}\n`
    );
    await execFileAsync('tar', ['-czf', archivePath, '-C', staging, '.']);
    return archivePath;
  } finally {
    await rm(staging, { recursive: true, force: true });
  }
}

async function main(): Promise<void> {
  if (process.platform !== 'linux' || process.arch !== 'x64') {
    throw new Error('Remote backend release packages must be built on Linux x86_64');
  }
  const version = process.argv[2];
  if (!version) throw new Error('Remote backend version argument is required');
  const projectRoot = fileURLToPath(new URL('../', import.meta.url));
  const { stdout: commitOutput } = await execFileAsync('git', ['rev-parse', 'HEAD'], { cwd: projectRoot });
  const archivePath = await writeRemoteBackendPackage({
    binaryPath: path.join(projectRoot, 'src-tauri/target/release/mac-command-bar-webview-preview'),
    bridgePath: path.join(projectRoot, 'tools/codex-acp-bridge/bridge.mjs'),
    outputDirectory: path.join(projectRoot, 'remote-backend-release'),
    version,
    commit: commitOutput.trim()
  });
  console.log(`Prepared ${archivePath}`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await main();
}
