/**
 * Lazy directory loader for the /next Files panel.
 *
 * Activation lists only the project root. A folder click lists only that
 * folder, and collapsing it releases its descendants. There is no recursive
 * project scan or cross-project explorer cache in this path.
 */
import {
  isNativeTauriRuntime,
  listSourceDirectoryFromTauri
} from '../../tauriSource.ts';
import { countInvoke } from '../devInvokeCounter.svelte.ts';
import {
  forgetAllProjectSourceRecords,
  setProjectSourceRecords
} from '../projectSourceIndex.ts';
import { sourceRecordFromPath } from '../editor/sourceRecordFromPath.ts';
import {
  applyDirectoryResult,
  beginScan,
  discardDirectory,
  endScan,
  explorer,
  explorerNodes,
  failScan,
  loadedExplorerDirectories,
  loadedExplorerDirectoryDepth,
  markCheckoutDeleted,
  resetExplorer,
  setExplorerError
} from './explorerStore.svelte.ts';

export const SCANNER_UNAVAILABLE_MESSAGE =
  'The file browser is not available here. Open this window in the CommandBar app to browse project files.';

let nextRequestId = 0;
const directoryRequests = new Map<string, number>();

function describeError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function checkoutWasDeleted(message: string): boolean {
  return message.includes('Could not read source root metadata:') &&
    (message.includes('No such file or directory') || message.includes('os error 2'));
}

function publishLoadedFiles(root: string): void {
  forgetAllProjectSourceRecords();
  setProjectSourceRecords(
    root,
    explorerNodes()
      .filter((node) => !node.isDirectory)
      .map((node) => sourceRecordFromPath(root, node.path))
  );
}

export async function loadDirectory(directory: string, depth: number): Promise<boolean> {
  const root = explorer.root;
  if (!root) return false;
  const requestId = ++nextRequestId;
  directoryRequests.set(directory, requestId);
  setExplorerError(null);

  try {
    countInvoke(isNativeTauriRuntime() ? 'list_source_directory' : 'bridge:list-source-directory');
    const entries = await listSourceDirectoryFromTauri(root, directory, explorer.includeExcluded);
    if (
      directoryRequests.get(directory) !== requestId ||
      explorer.root !== root
    ) {
      return false;
    }
    if (!entries) {
      if (directory === root) failScan(SCANNER_UNAVAILABLE_MESSAGE);
      else setExplorerError(SCANNER_UNAVAILABLE_MESSAGE);
      return false;
    }
    applyDirectoryResult(directory, depth, entries);
    publishLoadedFiles(root);
    return true;
  } catch (error) {
    if (directoryRequests.get(directory) !== requestId || explorer.root !== root) return false;
    const detail = describeError(error);
    const message = `Could not list this folder: ${detail}`;
    if (directory === root && checkoutWasDeleted(detail)) {
      failScan('This session’s checkout/worktree no longer exists.', 'checkout-deleted');
    } else if (directory === root) failScan(message);
    else setExplorerError(message);
    return false;
  } finally {
    if (directoryRequests.get(directory) === requestId) directoryRequests.delete(directory);
  }
}

export function unloadDirectory(directory: string): void {
  directoryRequests.delete(directory);
  discardDirectory(directory);
  if (explorer.root) publishLoadedFiles(explorer.root);
}

export async function scanRoot(root: string): Promise<void> {
  const target = root.trim();
  if (!target) return;
  directoryRequests.clear();
  resetExplorer();
  forgetAllProjectSourceRecords();
  beginScan(target);
  try {
    await loadDirectory(target, 0);
  } finally {
    if (explorer.root === target) endScan();
  }
}

export function activate(root: string | null, checkoutDeleted = false): void {
  const target = (root ?? '').trim();
  if (!target) {
    directoryRequests.clear();
    forgetAllProjectSourceRecords();
    if (checkoutDeleted) markCheckoutDeleted();
    else resetExplorer();
    return;
  }
  if (explorer.activated && explorer.root === target && explorer.error === null) return;
  void scanRoot(target);
}

export function refresh(): void {
  const directories = loadedExplorerDirectories();
  if (directories.length === 0 && explorer.root) {
    void loadDirectory(explorer.root, 0);
    return;
  }
  for (const directory of directories) void loadDirectory(directory.path, directory.depth);
}

export function refreshChangedPath(path: string): void {
  const cut = path.lastIndexOf('/');
  const parent = cut < 0 ? '' : path.slice(0, cut);
  const depth = loadedExplorerDirectoryDepth(parent);
  if (depth !== null) void loadDirectory(parent, depth);
}

export function stopScan(): void {
  directoryRequests.clear();
  endScan();
}
