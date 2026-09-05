import { execFile } from 'node:child_process';
import { createHash } from 'node:crypto';
import { chmod, copyFile, mkdir, readFile, writeFile } from 'node:fs/promises';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);
const projectRoot = new URL('../', import.meta.url);
const adapterDir = new URL('src-tauri/adapters/', projectRoot);
const stopController = new AbortController();

for (const signalName of ['SIGINT', 'SIGTERM']) {
  process.once(signalName, () => stopController.abort(new Error(`Adapter build stopped by ${signalName}`)));
}

async function run(command, args) {
  const { stdout, stderr } = await execFileAsync(command, args, {
    cwd: projectRoot,
    signal: stopController.signal,
    maxBuffer: 8 * 1024 * 1024
  });
  if (stdout.trim()) process.stdout.write(stdout);
  if (stderr.trim()) process.stderr.write(stderr);
}

async function packageVersion(packageName) {
  const packageJson = JSON.parse(
    await readFile(new URL(`node_modules/${packageName}/package.json`, projectRoot), 'utf8')
  );
  return packageJson.version;
}

async function sha256(path) {
  return createHash('sha256').update(await readFile(path)).digest('hex');
}

function launcher({ cli, displayName, environment, runtime }) {
  const environmentLines = environment === 'PATH'
    ? 'PATH="$(dirname "$cli_path"):$PATH"\nexport PATH'
    : `${environment}="$cli_path"\nexport ${environment}`;
  return `#!/bin/sh
set -eu
adapter_dir=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
cache_dir="$HOME/.mac-command-bar/cli-paths"
cache_file="$cache_dir/${cli}"
cli_path=""
if [ -f "$cache_file" ]; then
  IFS= read -r cli_path < "$cache_file" || true
fi
if [ ! -x "$cli_path" ]; then
  cli_path=$(command -v ${cli} 2>/dev/null || true)
fi
if [ ! -x "$cli_path" ] && [ -x /bin/zsh ]; then
  cli_path=$(/bin/zsh -lic 'command -v ${cli}' 2>/dev/null | tail -n 1)
fi
if [ ! -x "$cli_path" ]; then
  echo "${displayName} CLI was not found. Install it, then restart Assembly." >&2
  exit 127
fi
mkdir -p "$cache_dir"
cache_tmp="$cache_file.$$"
printf '%s\\n' "$cli_path" > "$cache_tmp"
mv "$cache_tmp" "$cache_file"
${environmentLines}
exec "$adapter_dir/${runtime}" "$@"
`;
}

function bunTarget() {
  const key = `${process.platform}-${process.arch}`;
  const targets = {
    'darwin-arm64': 'bun-darwin-arm64',
    'darwin-x64': 'bun-darwin-x64-baseline',
    'linux-arm64': 'bun-linux-arm64',
    'linux-x64': 'bun-linux-x64-baseline',
    'win32-arm64': 'bun-windows-arm64',
    'win32-x64': 'bun-windows-x64-baseline'
  };
  const target = targets[key];
  if (!target) throw new Error(`Release adapters do not support ${key}`);
  return target;
}

async function main() {
  await mkdir(adapterDir, { recursive: true });
  const executableSuffix = process.platform === 'win32' ? '.exe' : '';
  if (process.platform === 'win32') {
    throw new Error('Release adapter launchers are not implemented for Windows');
  }
  const codexOutput = new URL('codex-acp-runtime', adapterDir);
  const claudeOutput = new URL('claude-agent-acp-runtime', adapterDir);
  const agyOutput = new URL('agy-acp-runtime', adapterDir);
  const codexLauncher = new URL('codex-acp', adapterDir);
  const claudeLauncher = new URL('claude-agent-acp', adapterDir);
  const agyLauncher = new URL('agy-acp', adapterDir);

  await run('bun', [
    'build',
    'node_modules/@agentclientprotocol/codex-acp/dist/index.js',
    '--minify',
    '--compile',
    `--target=${bunTarget()}`,
    `--outfile=${codexOutput.pathname}`
  ]);
  await run('bun', [
    'build',
    'node_modules/@agentclientprotocol/claude-agent-acp/dist/index.js',
    '--minify',
    '--compile',
    `--target=${bunTarget()}`,
    `--outfile=${claudeOutput.pathname}`
  ]);
  await run('cargo', [
    'build',
    '--locked',
    '--release',
    '--manifest-path',
    'tools/agy-acp/Cargo.toml'
  ]);
  const agyBuild = new URL(`tools/agy-acp/target/release/agy-acp${executableSuffix}`, projectRoot);
  await copyFile(agyBuild, agyOutput);
  await writeFile(codexLauncher, launcher({
    cli: 'codex',
    displayName: 'Codex',
    environment: 'CODEX_PATH',
    runtime: 'codex-acp-runtime'
  }));
  await writeFile(claudeLauncher, launcher({
    cli: 'claude',
    displayName: 'Claude Code',
    environment: 'CLAUDE_CODE_EXECUTABLE',
    runtime: 'claude-agent-acp-runtime'
  }));
  await writeFile(agyLauncher, launcher({
    cli: 'agy',
    displayName: 'Antigravity',
    environment: 'PATH',
    runtime: 'agy-acp-runtime'
  }));

  for (const path of [codexOutput, claudeOutput, agyOutput, codexLauncher, claudeLauncher, agyLauncher]) {
    await chmod(path, 0o755);
  }

  const adapters = [
    {
      provider: 'codex',
      id: 'codex-acp',
      version: await packageVersion('@agentclientprotocol/codex-acp'),
      executable: 'codex-acp',
      files: [
        { path: 'codex-acp', sha256: await sha256(codexLauncher) },
        { path: 'codex-acp-runtime', sha256: await sha256(codexOutput) }
      ]
    },
    {
      provider: 'claude',
      id: 'claude-agent-acp',
      version: await packageVersion('@agentclientprotocol/claude-agent-acp'),
      executable: 'claude-agent-acp',
      files: [
        { path: 'claude-agent-acp', sha256: await sha256(claudeLauncher) },
        { path: 'claude-agent-acp-runtime', sha256: await sha256(claudeOutput) }
      ]
    },
    {
      provider: 'antigravity',
      id: 'agy-acp',
      version: '0.1.0',
      executable: 'agy-acp',
      files: [
        { path: 'agy-acp', sha256: await sha256(agyLauncher) },
        { path: 'agy-acp-runtime', sha256: await sha256(agyOutput) }
      ]
    }
  ];
  await writeFile(
    new URL('manifest.json', adapterDir),
    `${JSON.stringify({ schemaVersion: 1, adapters }, null, 2)}\n`
  );
  console.log(`Prepared ${adapters.length} release adapters in ${adapterDir.pathname}`);
}

await main();
