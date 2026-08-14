'use strict';

const vscode = require('vscode');

const ID = 'mcb.internal.extension-api-probe';
const VERSION = '0.1.0';
const PING_COMMAND = `${ID}.ping`;
const CONTEXT_COMMAND = `${ID}.currentContext`;
const ADAPTERS_COMMAND = `${ID}.runBoundedAdapters`;
const CONTEXT_CHANGED_COMMAND = `${ID}.contextChanged`;
const TERMINAL_BRIDGE_COMMAND = `${ID}.bridge.terminal`;
const SCM_BRIDGE_COMMAND = `${ID}.bridge.scm`;
const DISPOSE_BRIDGE_COMMAND = `${ID}.bridge.dispose`;
const REPORT_BRIDGE_COMMAND = `${ID}.bridge.report`;
const MARKER_FILE = 'README.md';
const MAX_ALLOW_LISTED_FILE_BYTES = 256 * 1024;

let activationCount = 0;
let staleEventCount = 0;
let lastContext = null;
const acceptedEvents = [];

function normalizePath(value) {
  const raw = String(value ?? '').trim().replaceAll('\\', '/').replace(/^file:\/\//, '');
  const absolute = raw.startsWith('/');
  const parts = [];
  for (const segment of raw.split('/')) {
    if (!segment || segment === '.') continue;
    if (segment === '..') {
      if (parts.length > 0) parts.pop();
      continue;
    }
    parts.push(segment);
  }
  const normalized = parts.join('/');
  return absolute ? `/${normalized}`.replace(/\/$/, '') || '/' : normalized || '.';
}

function isWithinRoot(root, candidate) {
  const normalizedRoot = normalizePath(root);
  const normalizedCandidate = normalizePath(candidate);
  return (
    normalizedCandidate === normalizedRoot ||
    normalizedCandidate.startsWith(`${normalizedRoot}/`)
  );
}

function markerUri(context) {
  return vscode.Uri.file(`${context.activeRoot}/${MARKER_FILE}`);
}

async function currentContext() {
  try {
    const context = await vscode.commands.executeCommand(CONTEXT_COMMAND);
    if (context && context.activeRoot && Number.isSafeInteger(context.generation)) {
      lastContext = {
        ownedId: String(context.ownedId),
        generation: context.generation,
        activeRoot: normalizePath(context.activeRoot),
        roots: Array.isArray(context.roots)
          ? context.roots.map((root) => ({
              path: normalizePath(root.path ?? root),
              uri: String(root.uri ?? vscode.Uri.file(normalizePath(root.path ?? root)).toString())
            }))
          : [{ path: normalizePath(context.activeRoot), uri: vscode.Uri.file(context.activeRoot).toString() }]
      };
      return lastContext;
    }
  } catch {
    // Fall through to the workspace-folder fallback for honest diagnostics.
  }

  const folder = vscode.workspace.workspaceFolders?.[0];
  if (!folder) {
    throw new Error('Assembly extension probe has no active canonical root.');
  }
  const root = normalizePath(folder.uri.fsPath);
  lastContext = {
    ownedId: 'vscode-workspace-fallback',
    generation: 1,
    activeRoot: root,
    roots: [{ path: root, uri: folder.uri.toString() }]
  };
  return lastContext;
}

function acceptEvent(context, uri) {
  if (!isWithinRoot(context.activeRoot, uri.fsPath)) {
    staleEventCount += 1;
    return;
  }
  acceptedEvents.push({
    kind: 'files',
    source: 'workspace.fs-watcher',
    generation: context.generation,
    activeRoot: context.activeRoot,
    path: normalizePath(uri.fsPath)
  });
}

function ownerRequest(context) {
  return {
    ownedId: context.ownedId,
    generation: context.generation,
    root: context.activeRoot
  };
}

async function readAllowListedMarker(context) {
  const uri = markerUri(context);
  if (!isWithinRoot(context.activeRoot, uri.fsPath)) {
    throw new Error('Extension probe marker read is outside the active root.');
  }
  const stat = await vscode.workspace.fs.stat(uri);
  if (stat.size > MAX_ALLOW_LISTED_FILE_BYTES) {
    throw new Error('Extension probe allow-listed file exceeds the read limit.');
  }
  const content = await vscode.workspace.fs.readFile(uri);
  let readDirectory;
  try {
    const entries = await vscode.workspace.fs.readDirectory(vscode.Uri.file(context.activeRoot));
    readDirectory = entries
      .filter(([name]) => name === MARKER_FILE)
      .map(([name, type]) => ({ name, type }));
  } catch (error) {
    readDirectory = {
      supported: false,
      message: error instanceof Error ? error.message : String(error)
    };
  }
  return {
    path: uri.fsPath,
    stat: { type: stat.type, size: stat.size },
    text: new TextDecoder().decode(content),
    readDirectory
  };
}

async function verifyBoundaryRejections(context) {
  let outsideReadRejected = false;
  try {
    await vscode.workspace.fs.readFile(
      vscode.Uri.file(`${context.activeRoot}-outside/${MARKER_FILE}`)
    );
  } catch {
    outsideReadRejected = true;
  }
  if (!outsideReadRejected) throw new Error('Extension probe outside-root read was accepted.');

  let staleGenerationRejected = false;
  try {
    await vscode.commands.executeCommand(TERMINAL_BRIDGE_COMMAND, {
      action: 'create',
      owner: { ...ownerRequest(context), generation: context.generation - 1 }
    });
  } catch {
    staleGenerationRejected = true;
  }
  if (!staleGenerationRejected) {
    throw new Error('Extension probe stale generation was accepted.');
  }
  return { outsideReadRejected, staleGenerationRejected };
}

async function ping(args = {}) {
  const context = await currentContext();
  if (args.generation !== undefined && args.generation !== context.generation) {
    throw new Error(`stale extension probe generation ${args.generation}; current is ${context.generation}.`);
  }
  const file = await readAllowListedMarker(context);
  const boundaries = await verifyBoundaryRejections(context);
  const progressResult = await vscode.window.withProgress(
    { location: vscode.ProgressLocation.Notification, title: 'Extension API probe', cancellable: false },
    async (progress) => {
      progress.report({ message: 'Read allow-listed marker', increment: 100 });
      return 'progress-complete';
    }
  );
  void vscode.window
    .showInformationMessage('Extension API probe activated')
    .then(undefined, () => {});
  const receipt = {
    extensionId: ID,
    version: VERSION,
    hostKind: 'LocalWebWorker',
    activationCount,
    ownedId: context.ownedId,
    generation: context.generation,
    workspaceFolderCount: context.roots.length,
    activeRoot: context.activeRoot,
    file,
    progressResult,
    notificationInvoked: true,
    acceptedEventCount: acceptedEvents.length,
    staleEventCount
  };
  receipt.boundaries = boundaries;
  await vscode.commands.executeCommand(REPORT_BRIDGE_COMMAND, {
    kind: 'activation',
    summary: 'Extension API probe activated',
    detail: `LocalWebWorker · generation ${context.generation} · README ${file.stat.size} bytes · outside and stale requests rejected`
  });
  return receipt;
}

async function runBoundedAdapters() {
  const context = await currentContext();
  const owner = ownerRequest(context);
  const terminal = {
    available: false,
    created: null,
    shown: null,
    written: null,
    resized: null,
    disposed: null
  };
  const created = await vscode.commands.executeCommand(TERMINAL_BRIDGE_COMMAND, {
    action: 'create',
    owner: { ...owner, cwd: context.activeRoot }
  });
  terminal.created = created;
  terminal.available = Boolean(created);
  if (!terminal.available) throw new Error('Extension probe terminal was not created.');
  try {
    terminal.shown = await vscode.commands.executeCommand(TERMINAL_BRIDGE_COMMAND, {
      action: 'show',
      owner
    });
    terminal.written = await vscode.commands.executeCommand(TERMINAL_BRIDGE_COMMAND, {
      action: 'write',
      owner,
      data: "printf 'mcb-extension-api-probe\\n'\r"
    });
    terminal.resized = await vscode.commands.executeCommand(TERMINAL_BRIDGE_COMMAND, {
      action: 'resize',
      owner,
      cols: 100,
      rows: 30
    });
  } finally {
    terminal.disposed = await vscode.commands.executeCommand(TERMINAL_BRIDGE_COMMAND, {
      action: 'dispose',
      owner
    });
  }

  const scm = {
    available: false,
    status: null,
    groups: [],
    selected: null,
    refreshed: null,
    disposed: null
  };
  const acquired = await vscode.commands.executeCommand(SCM_BRIDGE_COMMAND, {
    action: 'acquire',
    owner
  });
  scm.available = Boolean(acquired);
  if (!scm.available) throw new Error('Extension probe source control was not acquired.');
  try {
      scm.status = await vscode.commands.executeCommand(SCM_BRIDGE_COMMAND, {
        action: 'read',
        owner
      });
      scm.groups = await vscode.commands.executeCommand(SCM_BRIDGE_COMMAND, {
        action: 'groups',
        owner
      });
      const firstFile = scm.groups.flatMap((group) => group.files ?? [])[0];
      if (firstFile?.relativePath) {
        await vscode.commands.executeCommand(SCM_BRIDGE_COMMAND, {
          action: 'select',
          owner,
          relativePath: firstFile.relativePath
        });
        scm.selected = firstFile.relativePath;
      }
      scm.refreshed = await vscode.commands.executeCommand(SCM_BRIDGE_COMMAND, {
        action: 'refresh',
        owner
      });
  } finally {
    scm.disposed = await vscode.commands.executeCommand(SCM_BRIDGE_COMMAND, {
        action: 'dispose',
        owner
      });
  }

  const receipt = {
    extensionId: ID,
    hostKind: 'LocalWebWorker',
    ownedId: context.ownedId,
    generation: context.generation,
    activeRoot: context.activeRoot,
    terminal,
    scm
  };
  await vscode.commands.executeCommand(REPORT_BRIDGE_COMMAND, {
    kind: 'adapters',
    summary: 'Extension adapters completed',
    detail: `Terminal show/write/resize/dispose ${terminal.shown && terminal.written && terminal.resized && terminal.disposed ? 'passed' : 'failed'} · source control read/refresh/dispose ${scm.status && scm.refreshed && scm.disposed ? 'passed' : 'failed'} · ${scm.groups.length} groups`
  });
  return receipt;
}

async function contextChanged(event) {
  const current = await currentContext();
  const next = event?.context;
  if (
    !next ||
    next.ownedId !== current.ownedId ||
    next.generation !== current.generation ||
    normalizePath(next.activeRoot) !== current.activeRoot
  ) {
    staleEventCount += 1;
    return { accepted: false, staleEventCount };
  }
  acceptedEvents.push({
    kind: 'workspaceFolders',
    source: 'bounded-active-root-adapter',
    generation: current.generation,
    activeRoot: current.activeRoot,
    path: current.activeRoot
  });
  acceptedEvents.push({
    kind: 'files',
    source: 'bounded-active-root-adapter',
    generation: current.generation,
    activeRoot: current.activeRoot,
    path: markerUri(current).fsPath
  });
  return { accepted: true, generation: current.generation, activeRoot: current.activeRoot };
}

async function executeReportedCommand(kind, operation) {
  try {
    return await operation();
  } catch (error) {
    try {
      await vscode.commands.executeCommand(REPORT_BRIDGE_COMMAND, {
        kind,
        summary: kind === 'activation' ? 'Extension API probe failed' : 'Extension adapters failed',
        detail: error instanceof Error ? error.message : String(error)
      });
    } catch {
      // Preserve the original command failure when even the report bridge is unavailable.
    }
    throw error;
  }
}

function activate(extensionContext) {
  activationCount += 1;
  const disposables = [];
  const markerWatcher = vscode.workspace.createFileSystemWatcher(`**/${MARKER_FILE}`);

  disposables.push(
    vscode.commands.registerCommand(PING_COMMAND, (args) =>
      executeReportedCommand('activation', () => ping(args))
    )
  );
  disposables.push(
    vscode.commands.registerCommand(ADAPTERS_COMMAND, () =>
      executeReportedCommand('adapters', runBoundedAdapters)
    )
  );
  disposables.push(vscode.commands.registerCommand(CONTEXT_CHANGED_COMMAND, contextChanged));
  disposables.push(
    vscode.workspace.onDidChangeWorkspaceFolders(async () => {
      const context = await currentContext();
      acceptedEvents.push({
        kind: 'workspaceFolders',
        source: 'vscode.workspace.onDidChangeWorkspaceFolders',
        generation: context.generation,
        activeRoot: context.activeRoot,
        path: context.activeRoot
      });
    })
  );
  disposables.push(markerWatcher);
  disposables.push(
    markerWatcher.onDidChange(async (uri) => {
      acceptEvent(await currentContext(), uri);
    })
  );

  extensionContext.subscriptions.push(...disposables);
  return currentContext()
    .then((context) => ({
      extensionId: ID,
      version: VERSION,
      hostKind: 'LocalWebWorker',
      activationCount,
      ownedId: context.ownedId,
      generation: context.generation,
      activeRoot: context.activeRoot
    }))
    .catch((error) => ({
      extensionId: ID,
      version: VERSION,
      hostKind: 'LocalWebWorker',
      activationCount,
      pendingContext: true,
      message: error instanceof Error ? error.message : String(error)
    }));
}

async function deactivate() {
  try {
    await vscode.commands.executeCommand(DISPOSE_BRIDGE_COMMAND);
  } catch {
    // The main-thread bridge may already be gone during app teardown.
  }
  lastContext = null;
}

module.exports = { activate, deactivate };
