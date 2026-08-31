import {
  findReferencesKeymap,
  formatKeymap,
  LSPClient,
  hoverTooltips,
  jumpToDefinitionKeymap,
  renameKeymap,
  serverCompletion,
  serverDiagnostics,
  signatureHelp,
  type Transport
} from '@codemirror/lsp-client';
import type { Extension } from '@codemirror/state';
import { keymap } from '@codemirror/view';

import {
  ensureNativeCsharpLanguageClientFromTauri,
  markNativeCsharpLanguageClientReadyFromTauri
} from '$lib/tauriSource';

type CodeMirrorCsharpSession = {
  root: string;
  extension(path: string): Promise<Extension>;
  dispose(): void;
};

let activeRoot: string | null = null;
let activeSession: CodeMirrorCsharpSession | null = null;

type CsharpClientState = {
  disposed: boolean;
  socket: WebSocket | null;
  client: LSPClient | null;
  detachSocketListeners: (() => void) | null;
};

function normalizedPath(path: string): string {
  return path.replaceAll('\\', '/').replace(/\/+$/, '');
}

function fileUri(path: string): string {
  return `file://${normalizedPath(path).split('/').map(encodeURIComponent).join('/')}`;
}

async function startCsharpLanguageClient(
  requestedRoot: string,
  state: CsharpClientState,
  disposeOnFailure: () => void
): Promise<LSPClient> {
  try {
    const endpoint = await ensureNativeCsharpLanguageClientFromTauri(requestedRoot);
    if (state.disposed) throw new Error('The C# client left the visible editor.');
    if (!endpoint || normalizedPath(endpoint.root) !== requestedRoot) {
      throw new Error('Rust did not provide a C# endpoint for the active project.');
    }

    const handlers = new Set<(message: string) => void>();
    let socketError: Error | null = null;
    const socket = new WebSocket(endpoint.wsUrl);
    state.socket = socket;
    function handleSocketError(): void {
      socketError = new Error('Could not connect to Roslyn.');
    }
    function handleSocketClose(): void {
      if (!state.disposed && !state.client) socketError = new Error('Could not connect to Roslyn.');
    }
    function handleSocketMessage(event: MessageEvent): void {
      const message = String(event.data);
      for (const handler of handlers) handler(message);
    }
    socket.addEventListener('error', handleSocketError, { once: true });
    socket.addEventListener('close', handleSocketClose, { once: true });
    socket.addEventListener('message', handleSocketMessage);
    state.detachSocketListeners = () => {
      socket.removeEventListener('error', handleSocketError);
      socket.removeEventListener('close', handleSocketClose);
      socket.removeEventListener('message', handleSocketMessage);
      handlers.clear();
      state.detachSocketListeners = null;
    };

    const transport: Transport = {
      send(message) {
        socket.send(message);
      },
      subscribe(handler) {
        handlers.add(handler);
      },
      unsubscribe(handler) {
        handlers.delete(handler);
      }
    };
    const client = new LSPClient({
      rootUri: fileUri(requestedRoot),
      timeout: 30_000,
      extensions: [
        serverCompletion({ override: true }),
        hoverTooltips(),
        keymap.of([...formatKeymap, ...renameKeymap, ...jumpToDefinitionKeymap, ...findReferencesKeymap]),
        signatureHelp(),
        serverDiagnostics()
      ]
    }).connect(transport);
    state.client = client;
    await client.initializing;
    if (socketError) throw socketError;
    if (state.disposed || activeRoot !== requestedRoot) {
      throw new Error('The C# client left the active Supercharged project.');
    }
    await markNativeCsharpLanguageClientReadyFromTauri(requestedRoot);
    return client;
  } catch (error) {
    disposeOnFailure();
    throw error instanceof Error ? error : new Error(String(error));
  }
}

/** Keep the browser-side client aligned with Rust's one active Supercharged root. */
export function setNativeCsharpActiveRoot(root: string | null): void {
  const nextRoot = root ? normalizedPath(root) : null;
  if (activeRoot === nextRoot) return;
  activeRoot = nextRoot;
  if (activeSession?.root !== nextRoot) {
    activeSession?.dispose();
    activeSession = null;
  }
}

/** Connect the visible CodeMirror document to Rust's authenticated Roslyn WebSocket. */
export function connectCodeMirrorCsharpClient(root: string): CodeMirrorCsharpSession {
  const requestedRoot = normalizedPath(root);
  if (activeRoot !== requestedRoot) {
    throw new Error('This C# document belongs to an inactive Supercharged project.');
  }
  if (activeSession?.root === requestedRoot) return activeSession;

  activeSession?.dispose();
  const state: CsharpClientState = {
    disposed: false,
    socket: null,
    client: null,
    detachSocketListeners: null
  };
  let session: CodeMirrorCsharpSession | null = null;
  const ready = startCsharpLanguageClient(requestedRoot, state, () => session?.dispose());

  session = {
    root: requestedRoot,
    async extension(path: string) {
      const documentPath = normalizedPath(path);
      if (documentPath !== requestedRoot && !documentPath.startsWith(`${requestedRoot}/`)) {
        throw new Error('The active C# document is outside its Roslyn workspace root.');
      }
      return (await ready).plugin(fileUri(documentPath), 'csharp');
    },
    dispose() {
      if (state.disposed) return;
      state.disposed = true;
      state.detachSocketListeners?.();
      state.client?.disconnect();
      state.socket?.close();
      if (activeSession === session) activeSession = null;
    }
  };
  activeSession = session;
  return session;
}
