/**
 * shellCommands.ts — the actions the /next command palette offers.
 *
 * Registration is pure bookkeeping: it stores functions in the palette's
 * registry and calls none of them, so it is safe at component init and must
 * never be put in an `$effect`. Each action is a plain sentence a user can read
 * without knowing how the shell is built.
 *
 * The palette seeds "Reset layout", "Look for agent sessions" and "Open
 * settings" itself from the callbacks the page passes it; everything here is
 * the panels' own actions.
 */
import { browser, reloadBrowserFrame } from './browser/browserStore.svelte.ts';
import { refreshAll as refreshContextCards } from './context/contextService.ts';
import { closeEditorFile, editorState } from './editor/editorStore.svelte.ts';
import { explorer } from './explorer/explorerStore.svelte.ts';
import { refresh as refreshFileList, stopScan } from './explorer/explorerService.ts';
import { gitPanel } from './git/gitPanelStore.svelte.ts';
import { gitService } from './git/gitService.ts';
import type { SidebarViewId } from './layout/sidebarViews.ts';
import { registerCommands } from './palette/commandRegistry.ts';
import { refresh as refreshPlaywright } from './processes/playwrightService.ts';
import { refreshProblemsForSelection } from './shellPanels.ts';
import { refreshStacks } from './stacks/stackService.ts';

export interface ShellCommandHooks {
  /** Bring a center tab to the front (its id in the tab roster). */
  showPanel(id: string): void;
  /** Unfold the Source control section of the tool column on the right. */
  expandSourceControl(): void;
  /** Open one view of the tool column on the right. Opening a view is what
   * lets that view read anything, so this is the entry point for somebody who
   * has not got the column open at all. */
  showView(id: SidebarViewId): void;
  /** Open the new-session dialog. */
  openNewSession(): void;
}

/** Register the panel actions. Calling it again replaces them, never doubles. */
export function registerShellCommands(hooks: ShellCommandHooks): () => void {
  const sourceControlUnavailable = () => !gitPanel.activated || gitPanel.desktopOnly;

  return registerCommands('panels', [
    {
      id: 'show-session',
      label: 'Show the terminal',
      detail: 'Bring the session you are running to the front',
      perform: () => hooks.showPanel('session')
    },
    {
      id: 'show-editor',
      label: 'Show the editor',
      detail: 'Bring the code you have open to the front',
      perform: () => hooks.showPanel('editor')
    },
    {
      id: 'show-git',
      label: 'Show source control',
      detail: 'Open the source control section of the tool column on the right',
      perform: () => hooks.expandSourceControl()
    },
    {
      id: 'show-browser',
      label: 'Show the browser',
      detail: 'Bring the web page panel to the front',
      perform: () => hooks.showPanel('browser')
    },
    {
      id: 'show-diff',
      label: 'Show the changes to a file',
      detail: 'Bring the diff panel to the front',
      perform: () => hooks.showPanel('diff')
    },
    {
      id: 'new-session',
      label: 'Start a new session',
      detail: 'Pick a project folder and start an agent in it',
      perform: () => hooks.openNewSession()
    },
    {
      id: 'git-refresh',
      label: 'Source control: refresh',
      detail: 'Read the changed files and recent commits again',
      disabled: sourceControlUnavailable,
      perform: () => gitService.refresh()
    },
    {
      id: 'git-fetch',
      label: 'Source control: fetch',
      detail: 'Fetch from the remote without changing your files',
      disabled: sourceControlUnavailable,
      perform: () => gitService.runRemoteAction('fetch')
    },
    {
      id: 'git-pull',
      label: 'Source control: pull',
      detail: 'Bring down commits from the remote branch',
      disabled: sourceControlUnavailable,
      perform: () => gitService.runRemoteAction('pull')
    },
    {
      id: 'git-push',
      label: 'Source control: push',
      detail: 'Send this branch to the remote',
      disabled: sourceControlUnavailable,
      perform: () => gitService.runRemoteAction('push')
    },
    {
      id: 'explorer-refresh',
      label: 'Refresh the file list',
      detail: 'List this project’s files again',
      disabled: () => !explorer.root,
      perform: () => refreshFileList()
    },
    {
      id: 'explorer-stop-scan',
      label: 'Stop listing files',
      detail: 'Give up on the file list that is still being read',
      disabled: () => !explorer.scanning,
      perform: () => stopScan()
    },
    {
      id: 'context-refresh',
      label: 'Refresh the context cards',
      detail: 'Read runs, processes, agent sessions, worktrees and repositories again',
      perform: () => {
        refreshContextCards();
        void refreshPlaywright();
      }
    },
    {
      id: 'playwright-show',
      label: 'Show leftover Playwright processes',
      detail: 'Open the Context view and look again for browsers Playwright left running',
      perform: () => {
        hooks.showView('context');
        void refreshPlaywright();
      }
    },
    {
      id: 'worktrees-clean-up',
      label: 'Clean up worktrees',
      detail: 'Open the worktree list and see which folders are safe to remove',
      perform: () => hooks.showView('worktrees')
    },
    {
      id: 'show-stacks',
      label: 'Show stacks',
      detail: 'Open the stacks section of the tool column on the right',
      perform: () => hooks.showView('stacks')
    },
    {
      id: 'stacks-refresh',
      label: 'Stacks: refresh',
      detail: 'Look again at which ports the stacks have open',
      perform: () => void refreshStacks()
    },
    {
      id: 'problems-refresh',
      label: 'Look for problems again',
      detail: 'Ask the language server what is wrong with this project',
      perform: () => refreshProblemsForSelection()
    },
    {
      id: 'browser-reload',
      label: 'Reload the web page',
      detail: 'Load the address in the browser panel again',
      disabled: () => !browser.url,
      perform: () => {
        reloadBrowserFrame();
        hooks.showPanel('browser');
      }
    },
    {
      id: 'editor-close-file',
      label: 'Close the file in the editor',
      detail: 'Stop showing the file you are reading',
      disabled: () => editorState.activePath === null,
      perform: () => {
        if (editorState.activePath) closeEditorFile(editorState.activePath);
      }
    }
  ]);
}
