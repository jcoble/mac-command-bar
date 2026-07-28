/**
 * shellPanels.ts — binds the /next panels' loaders to the session rail.
 *
 * The decision of *when* a panel may load lives in `panelActivation.ts` (pure,
 * tested). This file is the small amount of glue that names the real loaders
 * and reads the current project out of the rail. No backend call is made here:
 * every function below hands off to a lane's own service, which is where the
 * calls are counted.
 *
 * Nothing runs at import — the rail is only read when a loader is about to run.
 */
import { activate as activateContextCards } from './context/contextService.ts';
import { activate as activateEditor } from './editor/sourceIntelligence.ts';
import { activate as activateExplorer } from './explorer/explorerService.ts';
import { gitService } from './git/gitService.ts';
import { createPanelActivation, type PanelProject, type ProjectSelection } from './panelActivation.ts';
import { rail } from './stores/sessionRailStore.svelte.ts';
import { activateBrowser } from './browser/browserStore.svelte.ts';

/** Last folder of a path, for naming a project in the context cards. */
function folderName(path: string): string {
  const parts = path.split('/').filter(Boolean);
  return parts.length > 0 ? parts[parts.length - 1] : path;
}

/** A session's project folder: the project it was opened for, else its cwd. */
function folderFor(session: { projectPath: string | null; cwd: string }): string {
  return (session.projectPath ?? session.cwd ?? '').trim();
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

export const shellPanels = createPanelActivation(
  {
    editor: (root) => activateEditor(root),
    git: (root) => gitService.activate(root),
    browser: () => activateBrowser(),
    explorer: (root) => activateExplorer(root),
    context: (selection) =>
      activateContextCards({
        projects: selection.projects,
        activeRoot: selection.root.trim() || null
      })
  },
  readSelection
);
