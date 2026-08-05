import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import {
  MCB_EXTENSION_API_PROBE_BROWSER_ENTRY,
  MCB_EXTENSION_API_PROBE_ADAPTERS_COMMAND,
  MCB_EXTENSION_API_PROBE_COMMAND,
  MCB_EXTENSION_API_PROBE_CONTEXT_COMMAND,
  MCB_EXTENSION_API_PROBE_ID,
  MCB_EXTENSION_API_PROBE_MANIFEST,
  createMcbExtensionApiProbe,
  createProbeWorkspaceContext,
  isProbePathWithinRoot,
  normalizeProbePath,
  relativeProbePath
} from '../src/lib/shell/editor/fixtures/mcbExtensionApiProbe.ts';

const fixtureSource = readFileSync(
  new URL('../src/lib/shell/editor/fixtures/mcbExtensionApiProbe.ts', import.meta.url),
  'utf8'
);
const browserEntrySource = readFileSync(
  new URL('../src/lib/shell/editor/fixtures/mcbExtensionApiProbe.browser.cjs', import.meta.url),
  'utf8'
);
const runtimeSource = readFileSync(
  new URL('../src/lib/shell/extensions/extensionRuntime.ts', import.meta.url),
  'utf8'
);
const catalogSource = readFileSync(
  new URL('../src/lib/shell/extensions/extensionCatalog.ts', import.meta.url),
  'utf8'
);
const csharpClientSource = readFileSync(
  new URL('../src/lib/shell/editor/csharpLanguageClient.ts', import.meta.url),
  'utf8'
);
const bridgeSource = readFileSync(
  new URL('../src/lib/shell/editor/extensionApiProbeBridge.ts', import.meta.url),
  'utf8'
);
const controllerSource = readFileSync(
  new URL('../src/lib/shell/editor/extensionApiProbeController.ts', import.meta.url),
  'utf8'
);
const monacoWorkersSource = readFileSync(
  new URL('../src/lib/shell/editor/monacoWorkers.ts', import.meta.url),
  'utf8'
);
const extensionHostIframeSource = readFileSync(
  new URL('../static/vscode/webWorkerExtensionHostIframe.html', import.meta.url),
  'utf8'
);
const nextPageSource = readFileSync(new URL('../src/routes/next/+page.svelte', import.meta.url), 'utf8');

// The contract must remain host-neutral. The controller supplies the existing
// singleton/PTY/Rust adapters; this Node test must not load CSS, Tauri, VS Code,
// or a browser-only service container.
for (const forbidden of [
  "@tauri-apps",
  "from 'vscode'",
  'child_process',
  'Command::new',
  'invoke(',
  'registerExtension('
]) {
  assert.doesNotMatch(fixtureSource, new RegExp(forbidden.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')));
}

assert.equal(MCB_EXTENSION_API_PROBE_MANIFEST.browser, MCB_EXTENSION_API_PROBE_BROWSER_ENTRY);
assert.equal(MCB_EXTENSION_API_PROBE_BROWSER_ENTRY.endsWith('.cjs'), true);
assert.match(browserEntrySource, /module\.exports = \{ activate, deactivate \}/);
assert.match(browserEntrySource, /const vscode = require\('vscode'\)/);
assert.doesNotMatch(browserEntrySource, /^\s*import\s/m, 'browser entry must remain CommonJS, not ESM');
assert.match(browserEntrySource, /const ID = 'mcb\.internal\.extension-api-probe'/);
assert.match(browserEntrySource, /const PING_COMMAND = `\$\{ID\}\.ping`/);
assert.match(browserEntrySource, /const CONTEXT_COMMAND = `\$\{ID\}\.currentContext`/);
assert.match(browserEntrySource, /const ADAPTERS_COMMAND = `\$\{ID\}\.runBoundedAdapters`/);
assert.match(browserEntrySource, /async function runBoundedAdapters\(\)/);
assert.match(browserEntrySource, /bounded-active-root-adapter/);
assert.doesNotMatch(browserEntrySource, /vscode\.window\.createTreeView/);
assert.doesNotMatch(browserEntrySource, /registerTreeDataProvider|TREE_VIEW|TREE_ROW_COMMAND/);
assert.equal('views' in MCB_EXTENSION_API_PROBE_MANIFEST.contributes, false);
assert.match(browserEntrySource, /vscode\.workspace\.fs\.readFile/);
assert.match(browserEntrySource, /vscode\.workspace\.fs\.stat/);
assert.match(browserEntrySource, /vscode\.workspace\.fs\.readDirectory/);
assert.match(browserEntrySource, /vscode\.workspace\.onDidChangeWorkspaceFolders/);
assert.match(browserEntrySource, /vscode\.workspace\.createFileSystemWatcher/);
assert.match(browserEntrySource, /vscode\.window\s*\.\s*showInformationMessage/);
assert.match(browserEntrySource, /void vscode\.window\s*\.\s*showInformationMessage/);
assert.match(browserEntrySource, /\.then\(undefined, \(\) => \{\}\)/);
assert.doesNotMatch(browserEntrySource, /await vscode\.window\.showInformationMessage/);
assert.match(browserEntrySource, /vscode\.window\.withProgress/);
assert.match(browserEntrySource, /const MARKER_FILE = 'README\.md'/);
assert.match(browserEntrySource, /MAX_ALLOW_LISTED_FILE_BYTES/);
assert.match(browserEntrySource, /outsideReadRejected/);
assert.match(browserEntrySource, /staleGenerationRejected/);
assert.match(browserEntrySource, /REPORT_BRIDGE_COMMAND/);
assert.doesNotMatch(browserEntrySource, /executeOptionalBridgeCommand/);
assert.doesNotMatch(browserEntrySource, /unsupportedBridgeReceipt/);

assert.match(catalogSource, /mac-command-bar\.extension-api-probe/);
assert.match(runtimeSource, /mcbExtensionApiProbeBrowserUrl/);
assert.match(runtimeSource, /ExtensionHostKind\.LocalWebWorker/);
assert.match(runtimeSource, /registerFileUrl\(\s*MCB_EXTENSION_API_PROBE_BROWSER_ENTRY/);
assert.match(
  runtimeSource,
  /new URL\(mcbExtensionApiProbeBrowserUrl, globalThis\.location\.href\)\.href/,
  'the LocalWebWorker browser entry must resolve on the app origin, never as file:///src'
);
assert.equal(
  (runtimeSource.match(/mac-command-bar\.extension-api-probe/g) ?? []).length,
  1,
  'probe extension must be registered through one curated runtime branch'
);
assert.match(runtimeSource, /__mcbCuratedExtensionRegistrations/);
assert.doesNotMatch(csharpClientSource, /updateWorkspaceFolders/);
assert.match(csharpClientSource, /enableExtHostWorker:\s*true/);
assert.match(csharpClientSource, /registerExtensionApiProbeBridgeCommands/);
assert.match(bridgeSource, /MCB_EXTENSION_API_PROBE_CONTEXT_COMMAND/);
assert.match(bridgeSource, /MCB_EXTENSION_API_PROBE_TERMINAL_BRIDGE_COMMAND/);
assert.match(bridgeSource, /MCB_EXTENSION_API_PROBE_SCM_BRIDGE_COMMAND/);
assert.match(bridgeSource, /MCB_EXTENSION_API_PROBE_REPORT_BRIDGE_COMMAND/);
assert.match(controllerSource, /terminalService\.createProbe/);
assert.match(controllerSource, /acquireExtensionApiProbeScmLease/);
assert.match(controllerSource, /request\.ownedId !== current\.ownedId/);
assert.match(controllerSource, /request\.generation !== current\.generation/);
assert.match(controllerSource, /recordExtensionApiProbeObservation/);
assert.match(nextPageSource, /role="status" aria-live="polite"/);
assert.match(
  monacoWorkersSource,
  /@codingame\/monaco-vscode-api\/workers\/extensionHost\.worker\?url/
);
const extensionHostInlineScript = extensionHostIframeSource.match(
  /<script>([\s\S]*?)<\/script>/
)?.[1];
assert.ok(extensionHostInlineScript, 'extension-host iframe must contain its bootstrap script');
const extensionHostInlineScriptHash = createHash('sha256')
  .update(extensionHostInlineScript)
  .digest('base64');
assert.match(
  extensionHostIframeSource,
  new RegExp(`script-src[^;]*'sha256-${extensionHostInlineScriptHash.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}'`),
  'extension-host iframe CSP must allow exactly its current inline bootstrap script'
);
assert.doesNotMatch(
  monacoWorkersSource,
  /@codingame\/monaco-vscode-api\/workers\/extensionHost\.worker\?worker&url/,
  'extensionHostWorkerMain must be a plain module URL because the VS Code worker service wraps and imports it'
);
assert.match(monacoWorkersSource, /case "extensionHostWorkerMain":\s*return extensionHostWorkerUrl;/);
assert.match(
  monacoWorkersSource,
  /case "webWorkerExtensionHostIframe":[\s\S]*?return "\/vscode\/webWorkerExtensionHostIframe\.html";/,
  'the extension-host HTML bootstrap must use the app-owned static asset instead of the Vite 8 pnpm HTML path'
);
assert.match(nextPageSource, /setExtensionApiProbeWorkspace/);
assert.match(nextPageSource, /disposeExtensionApiProbeRuntime\(\)\.finally/);
assert.equal(
  MCB_EXTENSION_API_PROBE_MANIFEST.activationEvents.includes(
    `onCommand:${MCB_EXTENSION_API_PROBE_ADAPTERS_COMMAND}`
  ),
  true
);

assert.equal(normalizeProbePath('/workspace/src/../README.md'), '/workspace/README.md');
assert.equal(normalizeProbePath('workspace\\src\\file.ts'), 'workspace/src/file.ts');
assert.equal(isProbePathWithinRoot('/workspace', '/workspace/src/file.ts'), true);
assert.equal(isProbePathWithinRoot('/workspace', '/workspace-other/file.ts'), false);
assert.equal(relativeProbePath('/workspace', '/workspace/src/file.ts'), 'src/file.ts');
assert.equal(relativeProbePath('/workspace', '/outside/file.ts'), null);

const initial = createProbeWorkspaceContext({
  ownedId: 'owned-probe',
  generation: 1,
  roots: ['/workspace', '/workspace-two'],
  activeRoot: '/workspace'
});
const next = createProbeWorkspaceContext({
  ownedId: 'owned-probe',
  generation: 2,
  roots: ['/workspace-two'],
  activeRoot: '/workspace-two'
});

function makeDisposable(log, label) {
  let disposed = false;
  return {
    dispose() {
      if (disposed) return;
      disposed = true;
      log.push(['dispose', label]);
    }
  };
}

function makeHost() {
  const log = [];
  const commands = new Map();
  const workspaceListeners = [];
  const fileListeners = [];
  const notifications = [];
  const progressRuns = [];
  let context = initial;
  let terminalNumber = 0;
  const scmLog = [];

  const host = {
    commands: {
      registerCommand(command, handler) {
        assert.equal(commands.has(command), false, `duplicate command registration: ${command}`);
        commands.set(command, handler);
        log.push(['register-command', command]);
        return makeDisposable(log, `command:${command}`);
      },
      async executeCommand(command, ...arguments_) {
        const handler = commands.get(command);
        assert.ok(handler, `missing command: ${command}`);
        return handler(arguments_[0]);
      }
    },
    workspace: {
      currentContext: () => context,
      fs: {
        async readFile(root, relativePath) {
          log.push(['readFile', root, relativePath]);
          return `fixture:${root}:${relativePath}`;
        },
        async stat(root, relativePath) {
          log.push(['stat', root, relativePath]);
          return { type: 'file', size: 7 };
        },
        async readDirectory(root, relativePath) {
          log.push(['readDirectory', root, relativePath]);
          return [{ name: 'fixture.ts', type: 'file' }];
        }
      },
      onDidChangeWorkspaceFolders(listener) {
        workspaceListeners.push(listener);
        return makeDisposable(log, 'workspace-listener');
      },
      onDidChangeFiles(listener) {
        fileListeners.push(listener);
        return makeDisposable(log, 'file-listener');
      }
    },
    window: {
      async showInformationMessage(message, ...actions) {
        notifications.push({ message, actions });
        return actions[0];
      },
      async withProgress(options, task) {
        const reports = [];
        progressRuns.push({ options, reports });
        return task({ report: (value) => reports.push(value) }, { isCancellationRequested: false });
      }
    },
    terminal: {
      async request(request) {
        terminalNumber += 1;
        log.push(['terminal-request', request]);
        return {
          sessionId: `pty-probe-${terminalNumber}`,
          ownedId: request.ownedId,
          generation: request.generation,
          root: request.root
        };
      },
      show(lease) {
        log.push(['terminal-show', lease.sessionId]);
      },
      async write(lease, data) {
        log.push(['terminal-write', lease.sessionId, data]);
      },
      async resize(lease, cols, rows) {
        log.push(['terminal-resize', lease.sessionId, cols, rows]);
      },
      async dispose(lease) {
        log.push(['terminal-dispose', lease.sessionId]);
      }
    },
    scm: {
      async readStatus(request) {
        scmLog.push(['read', request]);
        return {
          root: request.root,
          generation: request.generation,
          groups: [
            {
              id: 'changes',
              label: 'Changes',
              resources: [{ relativePath: 'src/fixture.ts', status: 'modified', selected: true }]
            }
          ],
          selectedRelativePath: 'src/fixture.ts'
        };
      },
      async selectResource(request, relativePath) {
        scmLog.push(['select', request, relativePath]);
      },
      async refresh(request) {
        scmLog.push(['refresh', request]);
        return {
          root: request.root,
          generation: request.generation,
          groups: [],
          selectedRelativePath: null
        };
      },
      async dispose(request) {
        scmLog.push(['dispose', request]);
      }
    }
  };

  return {
    host,
    log,
    commands,
    workspaceListeners,
    fileListeners,
    notifications,
    progressRuns,
    scmLog,
    setContext(value) {
      context = value;
    }
  };
}

const fake = makeHost();
const probe = createMcbExtensionApiProbe({
  allowedPaths: [
    { root: '/workspace', relativePath: 'src/fixture.ts' },
    { root: '/workspace', relativePath: 'src' }
  ]
});

const firstActivation = await probe.activate(fake.host, initial);
assert.deepEqual(firstActivation.registrations, {
  commands: 1,
  workspaceListeners: 1,
  fileListeners: 1
});
const secondActivation = await probe.activate(fake.host, initial);
assert.deepEqual(secondActivation.registrations, firstActivation.registrations);
assert.equal(fake.commands.size, 1, 'activation is idempotent and does not duplicate commands');

const ping = await fake.host.commands.executeCommand(MCB_EXTENSION_API_PROBE_COMMAND, {
  generation: 1
});
assert.deepEqual(ping, {
  extensionId: MCB_EXTENSION_API_PROBE_ID,
  version: '0.1.0',
  hostKind: 'LocalWebWorker',
  ownedId: 'owned-probe',
  generation: 1,
  workspaceFolderCount: 2,
  activeRoot: '/workspace'
});

assert.equal(
  await probe.workspaceFs.readFile({
    generation: 1,
    root: '/workspace',
    path: '/workspace/src/fixture.ts'
  }),
  'fixture:/workspace:src/fixture.ts'
);
assert.deepEqual(
  await probe.workspaceFs.stat({ generation: 1, root: '/workspace', path: '/workspace/src/fixture.ts' }),
  { type: 'file', size: 7 }
);
assert.deepEqual(
  await probe.workspaceFs.readDirectory({ generation: 1, root: '/workspace', path: '/workspace/src' }),
  [{ name: 'fixture.ts', type: 'file' }]
);
await assert.rejects(
  probe.workspaceFs.readFile({ generation: 1, root: '/workspace', path: '/outside/file.ts' }),
  /outside the active workspace roots/
);
await assert.rejects(
  probe.workspaceFs.readFile({ generation: 1, root: '/workspace', path: '/workspace/src/other.ts' }),
  /not allow-listed/
);
await assert.rejects(
  probe.workspaceFs.readFile({ generation: 0, root: '/workspace', path: '/workspace/src/fixture.ts' }),
  /stale extension probe generation/
);

await probe.showInformationMessage('Probe notification', 'Dismiss');
const progressResult = await probe.withProgress('Probe progress', async (progress) => {
  progress.report({ message: 'Reading fixture', increment: 100 });
  return 'done';
});
assert.equal(progressResult, 'done');
assert.equal(fake.notifications.length, 1);
assert.equal(fake.progressRuns[0].options.cancellable, false);
assert.deepEqual(fake.progressRuns[0].reports, [{ message: 'Reading fixture', increment: 100 }]);

const lease = await probe.terminal.request({
  ownedId: 'owned-probe',
  generation: 1,
  root: '/workspace',
  cwd: '/workspace/src',
  cols: 80,
  rows: 24
});
probe.terminal.show(lease);
await probe.terminal.write(lease, 'echo probe');
await probe.terminal.resize(lease, 100, 30);
await probe.terminal.dispose(lease);
await assert.rejects(
  probe.terminal.request({
    ownedId: 'other-owner',
    generation: 1,
    root: '/workspace',
    cwd: '/workspace'
  }),
  /not owned by the active session/
);
await assert.rejects(
  probe.terminal.request({
    ownedId: 'owned-probe',
    generation: 1,
    root: '/workspace',
    cwd: '/workspace/../outside'
  }),
  /outside the active workspace root/
);

const scm = await probe.scm.readStatus({ ownedId: 'owned-probe', generation: 1, root: '/workspace' });
assert.equal(scm.groups[0].resources[0].selected, true);
await probe.scm.selectResource(
  { ownedId: 'owned-probe', generation: 1, root: '/workspace' },
  'src/fixture.ts'
);
await probe.scm.refresh({ ownedId: 'owned-probe', generation: 1, root: '/workspace' });
await probe.scm.dispose({ ownedId: 'owned-probe', generation: 1, root: '/workspace' });
assert.deepEqual(
  Object.keys(fake.host.scm).sort(),
  ['dispose', 'readStatus', 'refresh', 'selectResource'],
  'SCM adapter exposes read/refresh/dispose only; no repository mutations'
);
await assert.rejects(
  probe.scm.readStatus({ ownedId: 'owned-probe', generation: 1, root: '/workspace-two' }),
  /active workspace root/
);

// One accepted file event and one stale event; stale data never reaches the
// extension-visible event ledger.
fake.fileListeners[0]({
  kind: 'files',
  generation: 1,
  ownedId: 'owned-probe',
  root: '/workspace',
  paths: ['/workspace/src/fixture.ts']
});
fake.fileListeners[0]({
  kind: 'files',
  generation: 0,
  ownedId: 'owned-probe',
  root: '/workspace',
  paths: ['/workspace/src/fixture.ts']
});
assert.equal(probe.snapshot().events.filter((event) => event.kind === 'files').length, 1);
assert.equal(probe.snapshot().staleEventCount, 1);

fake.setContext(next);
fake.workspaceListeners[0](next);
assert.equal(probe.snapshot().context.generation, 2);
assert.equal(probe.snapshot().events.filter((event) => event.kind === 'workspaceFolders').length, 1);
await assert.rejects(
  probe.workspaceFs.readFile({ generation: 1, root: '/workspace', path: '/workspace/src/fixture.ts' }),
  /stale extension probe generation/
);

probe.deactivate();
probe.deactivate();
assert.deepEqual(probe.snapshot().registrations, {
  commands: 0,
  workspaceListeners: 0,
  fileListeners: 0
});
assert.equal(probe.snapshot().active, false);
assert.equal(fake.log.filter(([kind]) => kind === 'dispose').length, 3);

console.log('extensionApiCompatibility.test.mjs: all checks passed');
