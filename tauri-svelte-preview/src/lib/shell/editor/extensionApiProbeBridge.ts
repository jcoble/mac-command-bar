import * as vscode from 'vscode';

import {
  acquireExtensionApiProbeScm,
  createExtensionApiProbeTerminal,
  currentExtensionApiProbeContext,
  disposeExtensionApiProbeResources,
  disposeExtensionApiProbeRuntime,
  disposeExtensionApiProbeScm,
  disposeExtensionApiProbeTerminal,
  groupsExtensionApiProbeScm,
  onExtensionApiProbeContextChanged,
  readExtensionApiProbeScm,
  recordExtensionApiProbeObservation,
  refreshExtensionApiProbeScm,
  resizeExtensionApiProbeTerminal,
  selectExtensionApiProbeScm,
  showExtensionApiProbeTerminal,
  writeExtensionApiProbeTerminal,
  type ExtensionApiProbeOwnerRequest,
  type ExtensionApiProbeTerminalRequest
} from './extensionApiProbeController.ts';
import {
  MCB_EXTENSION_API_PROBE_CONTEXT_CHANGED_COMMAND,
  MCB_EXTENSION_API_PROBE_CONTEXT_COMMAND,
  MCB_EXTENSION_API_PROBE_DISPOSE_BRIDGE_COMMAND,
  MCB_EXTENSION_API_PROBE_REPORT_BRIDGE_COMMAND,
  MCB_EXTENSION_API_PROBE_SCM_BRIDGE_COMMAND,
  MCB_EXTENSION_API_PROBE_TERMINAL_BRIDGE_COMMAND
} from './fixtures/mcbExtensionApiProbe.ts';

let registered = false;
let stopContextNotifications: (() => void) | null = null;

type TerminalBridgeRequest = {
  action: 'create' | 'show' | 'write' | 'resize' | 'dispose';
  owner: ExtensionApiProbeTerminalRequest;
  data?: string;
  cols?: number;
  rows?: number;
};

type ScmBridgeRequest = {
  action: 'acquire' | 'read' | 'groups' | 'select' | 'refresh' | 'dispose';
  owner: ExtensionApiProbeOwnerRequest;
  relativePath?: string;
};

async function terminalBridge(request: TerminalBridgeRequest) {
  switch (request.action) {
    case 'create':
      return createExtensionApiProbeTerminal(request.owner);
    case 'show':
      return showExtensionApiProbeTerminal(request.owner);
    case 'write':
      return writeExtensionApiProbeTerminal(request.owner, request.data ?? '');
    case 'resize':
      return resizeExtensionApiProbeTerminal(
        request.owner,
        request.cols ?? 0,
        request.rows ?? 0
      );
    case 'dispose':
      return disposeExtensionApiProbeTerminal(request.owner);
  }
}

async function scmBridge(request: ScmBridgeRequest) {
  switch (request.action) {
    case 'acquire':
      return acquireExtensionApiProbeScm(request.owner);
    case 'read':
      return readExtensionApiProbeScm(request.owner);
    case 'groups':
      return groupsExtensionApiProbeScm(request.owner);
    case 'select':
      return selectExtensionApiProbeScm(request.owner, request.relativePath ?? '');
    case 'refresh':
      return refreshExtensionApiProbeScm(request.owner);
    case 'dispose':
      return disposeExtensionApiProbeScm(request.owner);
  }
}

export function registerExtensionApiProbeBridgeCommands(): void {
  if (registered) return;
  registered = true;
  vscode.commands.registerCommand(MCB_EXTENSION_API_PROBE_CONTEXT_COMMAND, () => {
    const context = currentExtensionApiProbeContext();
    if (!context) throw new Error('The extension API probe has no active canonical root.');
    return context;
  });
  vscode.commands.registerCommand(MCB_EXTENSION_API_PROBE_TERMINAL_BRIDGE_COMMAND, terminalBridge);
  vscode.commands.registerCommand(MCB_EXTENSION_API_PROBE_SCM_BRIDGE_COMMAND, scmBridge);
  vscode.commands.registerCommand(MCB_EXTENSION_API_PROBE_DISPOSE_BRIDGE_COMMAND, async () => {
    await disposeExtensionApiProbeResources();
    return true;
  });
  vscode.commands.registerCommand(
    MCB_EXTENSION_API_PROBE_REPORT_BRIDGE_COMMAND,
    recordExtensionApiProbeObservation
  );
  stopContextNotifications = onExtensionApiProbeContextChanged((context, previous) => {
    void vscode.commands
      .executeCommand(MCB_EXTENSION_API_PROBE_CONTEXT_CHANGED_COMMAND, { context, previous })
      .then(undefined, () => {
        // The fixture is command-activated. A switch before activation has no
        // worker listener yet and is intentionally just the next context.
      });
  });
}

export async function disposeExtensionApiProbeBridge(): Promise<void> {
  stopContextNotifications?.();
  stopContextNotifications = null;
  await disposeExtensionApiProbeRuntime();
}
