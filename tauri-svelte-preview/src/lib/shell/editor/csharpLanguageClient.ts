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

function normalizedPath(path: string): string {
  return path.replaceAll('\\', '/').replace(/\/+$/, '');
}

function fileUri(path: string): string {
  return `file://${normalizedPath(path).split('/').map(encodeURIComponent).join('/')}`;
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
  let disposed = false;
  let socket: WebSocket | null = null;
  let client: LSPClient | null = null;
  let rejectReady: ((reason: Error) => void) | null = null;
  let session!: CodeMirrorCsharpSession;
  const ready = new Promise<LSPClient>(async (resolve, reject) => {
    rejectReady = reject;
    try {
      const endpoint = await ensureNativeCsharpLanguageClientFromTauri(requestedRoot);
      if (disposed) throw new Error('The C# client left the visible editor.');
      if (!endpoint || normalizedPath(endpoint.root) !== requestedRoot) {
        throw new Error('Rust did not provide a C# endpoint for the active project.');
      }

      const handlers = new Set<(message: string) => void>();
      socket = new WebSocket(endpoint.wsUrl);
      await new Promise<void>((opened, failed) => {
        socket!.addEventListener('open', () => opened(), { once: true });
        socket!.addEventListener('error', () => failed(new Error('Could not connect to Roslyn.')), {
          once: true
        });
      });
      if (disposed) throw new Error('The C# client left the visible editor.');

      socket.addEventListener('message', (event) => {
        const message = String(event.data);
        for (const handler of handlers) handler(message);
      });
      const transport: Transport = {
        send(message) {
          socket?.send(message);
        },
        subscribe(handler) {
          handlers.add(handler);
        },
        unsubscribe(handler) {
          handlers.delete(handler);
        }
      };
      client = new LSPClient({
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
      await client.initializing;
      if (disposed || activeRoot !== requestedRoot) {
        throw new Error('The C# client left the active Supercharged project.');
      }
      await markNativeCsharpLanguageClientReadyFromTauri(requestedRoot);
      resolve(client);
    } catch (error) {
      session.dispose();
      reject(error instanceof Error ? error : new Error(String(error)));
    }
  });

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
      if (disposed) return;
      disposed = true;
      rejectReady?.(new Error('The C# client left the visible editor.'));
      client?.disconnect();
      socket?.close();
      if (activeSession === session) activeSession = null;
    }
  };
  activeSession = session;
  return session;
}
