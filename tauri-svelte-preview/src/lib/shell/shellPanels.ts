/**
 * shellPanels.ts — binds the /next panels' loaders to the session rail.
 *
 * The decision of *when* a panel may load lives in `panelActivation.ts` (pure,
 * tested), including the two panels that load only while the user can see them:
 * source control and the context cards. This file is the small amount of glue
 * that names the real loaders and reads the current project out of the rail. No
 * backend call is made here: every function below hands off to a lane's own
 * service, which is where the calls are counted.
 *
 * Nothing runs at import — the rail is only read when a loader is about to run.
 */
import { activate as activateContextCards } from './context/contextService.ts';
import { activate as activateEditor } from './editor/sourceIntelligence.ts';
import { activate as activateExplorer } from './explorer/explorerService.ts';
import { gitService } from './git/gitService.ts';
import { createPanelActivation, type PanelProject, type ProjectSelection } from './panelActivation.ts';
import { activate as activatePlaywright } from './processes/playwrightService.ts';
import { activate as activateProblems, refresh as refreshProblems } from './problems/problemsService.ts';
import { activateStacks } from './stacks/stackService.ts';
import { rail } from './stores/sessionRailStore.svelte.ts';
import { activate as activateWorktrees } from './worktrees/worktreeManagerService.ts';
import type { WorktreeSessionInput } from './worktrees/worktreeManagerRows.ts';
import { activateBrowser } from './browser/browserStore.svelte.ts';

/** Last folder of a path, for naming a project in the context cards. */
function folderName(path: string): string {
  const parts = path.split('/').filter(Boolean);
  return parts.length > 0 ? parts[parts.length - 1] : path;
}

/** A session's working folder: its cwd (the worktree checkout) first, else the
 * project it was opened for. The cwd is where the session's files and branch
 * actually live — a worktree session pointed at projectPath shows the WRONG tree. */
function folderFor(session: { projectPath: string | null; cwd: string }): string {
  return session.cwd.trim() || (session.projectPath ?? '').trim();
}

/**
 * What the shell is pointed at: the active session's folder, plus one entry per
 * distinct folder across every owned session (the context cards ask the backend
 * about a list of projects, not just the current one).
 */
export function readSelection(): ProjectSelection {
  const active = rail.owned.find((entry) => entry.ownedId === rail.activeOwnedId) ?? null;
  const projects = new Map<string, PanelProject>();
  for (const session of rail.owned) {
    const path = folderFor(session);
    if (!path || projects.has(path)) continue;
    projects.set(path, { id: path, name: folderName(path), path });
  }
  return { root: active ? folderFor(active) : '', projects: [...projects.values()] };
}

/** The rail's sessions in the shape the worktree manager joins on. */
function worktreeSessions(): WorktreeSessionInput[] {
  return rail.owned.map((session) => ({
    ownedId: session.ownedId,
    title: session.title,
    cwd: session.cwd,
    projectPath: session.projectPath,
    state: session.state
  }));
}

/**
 * Point the Problems panel at whatever session is active right now, then load.
 *
 * This is the panel's Refresh button and the palette command. The bottom dock
 * is on screen from launch, so the panel deliberately loads nothing on its own
 * — this explicit gesture is how a root ever reaches it. Without this, Refresh
 * read a root that nothing had set and told the user to pick a session they
 * had already picked.
 */
export function refreshProblemsForSelection(): void {
  activateProblems(readSelection().root.trim() || null);
  void refreshProblems();
}

export const shellPanels = createPanelActivation(
  {
    editor: (root) => activateEditor(root),
    git: (root) => gitService.activate(root),
    browser: () => activateBrowser(),
    explorer: (root) => activateExplorer(root),
    context: (selection) => {
      activateContextCards({
        projects: selection.projects,
        activeRoot: selection.root.trim() || null
      });
      // The Playwright card sits with the context cards and is read at the same
      // moment. It takes no project — leftover browsers are machine-wide.
      activatePlaywright();
    },
    worktrees: (selection) =>
      activateWorktrees({
        root: selection.root.trim() || null,
        projectName: folderName(selection.root.trim()),
        sessions: worktreeSessions()
      }),
    stacks: (root) => activateStacks({ activeRoot: root }),
    problems: (root) => activateProblems(root)
  },
  readSelection
);
