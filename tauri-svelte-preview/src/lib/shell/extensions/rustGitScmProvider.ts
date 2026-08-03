import 'vscode/localExtensionHost';
import * as vscode from 'vscode';

import type { ProjectGitFileStatus, ProjectGitStatus } from '$lib/tauriSource';

export type RustGitScmSnapshot = {
  root: string | null;
  status: ProjectGitStatus | null;
};

let apiReady = false;
let pending: RustGitScmSnapshot = { root: null, status: null };
let sourceControl: vscode.SourceControl | null = null;
let staged: vscode.SourceControlResourceGroup | null = null;
let changes: vscode.SourceControlResourceGroup | null = null;
let untracked: vscode.SourceControlResourceGroup | null = null;
let activeRoot: string | null = null;

function absolutePath(root: string, relativePath: string): string {
  return `${root.replace(/\/+$/, '')}/${relativePath.replace(/^\/+/, '')}`;
}

function resourceState(
  root: string,
  file: ProjectGitFileStatus,
  contextValue: string
): vscode.SourceControlResourceState {
  return {
    resourceUri: vscode.Uri.file(absolutePath(root, file.relativePath)),
    contextValue,
    decorations: {
      tooltip: `${file.status}: ${file.relativePath}`,
      strikeThrough: file.status === 'deleted'
    }
  };
}

function disposeSourceControl(): void {
  sourceControl?.dispose();
  sourceControl = null;
  staged = null;
  changes = null;
  untracked = null;
  activeRoot = null;
}

function applySnapshot(snapshot: RustGitScmSnapshot): void {
  if (!apiReady) return;
  const root = snapshot.root?.trim() || null;
  if (!root) {
    disposeSourceControl();
    return;
  }

  if (!sourceControl || activeRoot !== root) {
    disposeSourceControl();
    sourceControl = vscode.scm.createSourceControl('mcb-rust-git', 'Mac Command Bar Git', vscode.Uri.file(root));
    staged = sourceControl.createResourceGroup('index', 'Staged Changes');
    changes = sourceControl.createResourceGroup('workingTree', 'Changes');
    untracked = sourceControl.createResourceGroup('untracked', 'Untracked Files');
    activeRoot = root;
  }

  const files = snapshot.status?.files ?? [];
  staged!.resourceStates = files
    .filter((file) => file.indexStatus.trim() !== '' && file.indexStatus !== '?')
    .map((file) => resourceState(root, file, 'mcbGitStaged'));
  changes!.resourceStates = files
    .filter((file) => file.worktreeStatus.trim() !== '' && file.worktreeStatus !== '?')
    .map((file) => resourceState(root, file, 'mcbGitChange'));
  untracked!.resourceStates = files
    .filter((file) => file.indexStatus === '?' || file.worktreeStatus === '?')
    .map((file) => resourceState(root, file, 'mcbGitUntracked'));
  sourceControl!.count = files.length;
}

/** Called once, after the singleton Monaco/VS Code wrapper has started. */
export function markRustGitScmApiReady(): void {
  if (apiReady) return;
  apiReady = true;
  applySnapshot(pending);
}

/**
 * Project the app's already-loaded Git status into VS Code's SCM API.
 * No Git command is issued here; gitService and Rust remain the authority.
 */
export function syncRustGitSourceControl(snapshot: RustGitScmSnapshot): void {
  pending = snapshot;
  applySnapshot(snapshot);
}

export function rustGitScmStatus(): {
  apiReady: boolean;
  root: string | null;
  resourceCount: number;
} {
  return {
    apiReady,
    root: activeRoot,
    resourceCount:
      (staged?.resourceStates.length ?? 0) +
      (changes?.resourceStates.length ?? 0) +
      (untracked?.resourceStates.length ?? 0)
  };
}
