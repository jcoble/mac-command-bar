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
  canonicalPath,
  discardDirectory,
  endScan,
  explorer,
  explorerScanGeneration,
  explorerNodes,
  failScan,
  loadedExplorerDirectories,
  loadedExplorerDirectoryDepth,
  isExplorerDirectoryPresent,
  isExplorerPathAtOrBelow,
  markCheckoutDeleted,
  resetExplorer,
  setExplorerError
} from './explorerStore.svelte.ts';

export const SCANNER_UNAVAILABLE_MESSAGE =
  'The file browser is not available here. Open this window in the CommandBar app to browse project files.';

let nextRequestId = 0;
const directoryRequests = new Map<string, number>();

function isCurrentDirectoryRequest(
  directory: string,
  root: string,
  generation: number,
  requestId: number
): boolean {
  return directoryRequests.get(directory) === requestId &&
    explorerScanGeneration() === generation &&
    canonicalPath(explorer.root ?? '') === root &&
    isExplorerDirectoryPresent(directory);
}

function describeError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function checkoutWasDeleted(message: string): boolean {
  const normalized = message.toLowerCase();
  return (normalized.includes('could not read source root metadata:') ||
    normalized.includes('could not read source directory metadata:')) &&
    (normalized.includes('no such file or directory') || normalized.includes('os error 2'));
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
  const root = canonicalPath(explorer.root ?? '');
  const target = canonicalPath(directory);
  if (!root || !target) return false;
  const generation = explorerScanGeneration();
  const requestId = ++nextRequestId;
  directoryRequests.set(target, requestId);
  setExplorerError(null);

  try {
    countInvoke(isNativeTauriRuntime() ? 'list_source_directory' : 'bridge:list-source-directory');
    const entries = await listSourceDirectoryFromTauri(root, target, explorer.includeExcluded);
    if (!isCurrentDirectoryRequest(target, root, generation, requestId)) return false;
    if (!entries) {
      if (target === root) failScan(SCANNER_UNAVAILABLE_MESSAGE);
      else setExplorerError(SCANNER_UNAVAILABLE_MESSAGE);
      return false;
    }
    applyDirectoryResult(target, depth, entries);
    publishLoadedFiles(root);
    return true;
  } catch (error) {
    if (!isCurrentDirectoryRequest(target, root, generation, requestId)) return false;
    const detail = describeError(error);
    const message = `Could not list this folder: ${detail}`;
    if (target === root && checkoutWasDeleted(detail)) {
      failScan('This session’s checkout/worktree no longer exists.', 'checkout-deleted');
    } else if (target === root) failScan(message);
    else setExplorerError(message);
    return false;
  } finally {
    if (directoryRequests.get(target) === requestId) directoryRequests.delete(target);
  }
}

/** Load the root and each missing directory on the path to a file, in order. */
export async function revealExplorerPath(path: string): Promise<string[]> {
  const root = canonicalPath(explorer.root ?? '');
  const target = canonicalPath(path);
  if (!root || !target || !isExplorerPathAtOrBelow(target, root) || target === root) return [];

  const relative = target.slice(root.length).replace(/^\/+/, '');
  const components = relative.split('/').filter(Boolean).slice(0, -1);
  const directories: string[] = [];
  let directory = root;
  for (const component of components) {
    directory = `${directory}/${component}`;
    directories.push(directory);
  }

  const loaded: string[] = [];
  for (const [index, candidate] of [root, ...directories].entries()) {
    const depth = index;
    if (loadedExplorerDirectoryDepth(candidate) === null && !(await loadDirectory(candidate, depth))) {
      break;
    }
    if (candidate !== root) loaded.push(candidate);
  }
  return loaded;
}

export function unloadDirectory(directory: string): void {
  const target = canonicalPath(directory);
  for (const pendingPath of directoryRequests.keys()) {
    if (isExplorerPathAtOrBelow(pendingPath, target)) directoryRequests.delete(pendingPath);
  }
  discardDirectory(target);
  if (explorer.root) publishLoadedFiles(explorer.root);
}

export async function scanRoot(root: string): Promise<void> {
  const target = canonicalPath(root);
  if (!target) return;
  directoryRequests.clear();
  resetExplorer();
  forgetAllProjectSourceRecords();
  beginScan(target);
  const generation = explorerScanGeneration();
  try {
    await loadDirectory(target, 0);
  } finally {
    if (explorer.root === target && explorerScanGeneration() === generation) endScan();
  }
}

export function activate(root: string | null, checkoutDeleted = false): void {
  const target = canonicalPath(root ?? '');
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
  const root = canonicalPath(explorer.root ?? '');
  if (!root) return;
  const target = canonicalPath(path);
  const cut = target.lastIndexOf('/');
  const parent = cut <= 0 ? root : target.slice(0, cut);
  const depth = loadedExplorerDirectoryDepth(parent);
  if (depth !== null) void loadDirectory(parent, depth);
}

export function stopScan(): void {
  directoryRequests.clear();
  endScan();
}
