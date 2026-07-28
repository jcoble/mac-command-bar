/**
 * panelActivation.ts — when each /next panel is allowed to load its data.
 *
 * PURE: no backend call, no Svelte, no DOM. It only decides *whether* and
 * *with which project* a panel's loader may run; the loaders themselves are
 * handed in (see `shellPanels.ts` for the real ones).
 *
 * The rule the shell is built on is "nothing loads at launch", and there are
 * two different launch noises to keep out:
 *
 * 1. **The center dock announcing a tab.** Building or restoring the tab area
 *    reports an active tab, and that report is indistinguishable from a click.
 *    So tab loads stay switched off until `allowPanelLoads()` is called, which
 *    the page does one timer tick after the frame is ready — every one of
 *    dockview's own announcements is delivered by then (they travel on
 *    microtasks, which all drain before the first timer callback).
 * 2. **Start-up re-attaching a terminal.** Re-attaching a session that survived
 *    a reload selects it, which looks exactly like the user picking it. So
 *    session loads stay switched off until `allowSessionLoads()` is called,
 *    which the page does when start-up has finished.
 *
 * After that, loading is driven by two user actions only: bringing a tab to the
 * front, and picking a session. A panel that has never been shown never loads,
 * and changing session re-loads only the panels the user has actually opened.
 */

/** A project folder, in the shape the backend's project-scoped commands want. */
export interface PanelProject {
  id: string;
  name: string;
  path: string;
}

/** What the shell is pointed at right now. */
export interface ProjectSelection {
  /** Folder of the session the user is looking at; `''` when there is none. */
  root: string;
  /** Every project folder the shell knows about, for the machine-wide cards. */
  projects: PanelProject[];
}

/** The loaders, one per panel. Each is safe to call more than once. */
export interface PanelActivators {
  editor(root: string | null): void;
  git(root: string | null): void;
  browser(): void;
  explorer(root: string): void;
  context(selection: ProjectSelection): void;
}

export interface PanelActivation {
  /** Start honouring tab activations (one timer tick after the frame is ready). */
  allowPanelLoads(): void;
  /** Start honouring session picks (once start-up has finished). */
  allowSessionLoads(): void;
  /** A center tab came to the front. */
  panelShown(id: string): void;
  /** The user picked a session in the rail. */
  sessionPicked(): void;
  /** Which panels have loaded at least once — for tests and for the report. */
  loadedPanels(): string[];
}

/** Center tabs that have something to load. "session" is the terminal: it is
 * owned by the page's own start-up and must never be re-loaded from here. */
const LOADABLE_PANELS = new Set(['editor', 'git', 'browser']);

export function createPanelActivation(
  activators: PanelActivators,
  readSelection: () => ProjectSelection
): PanelActivation {
  const shownPanels = new Set<string>();
  let panelLoadsAllowed = false;
  let sessionLoadsAllowed = false;
  let sessionPanelsShown = false;

  const loadPanel = (id: string, selection: ProjectSelection): void => {
    const root = selection.root.trim();
    if (id === 'editor') activators.editor(root || null);
    else if (id === 'git') activators.git(root || null);
    else if (id === 'browser') activators.browser();
  };

  const loadSessionPanels = (selection: ProjectSelection): void => {
    if (selection.root.trim()) activators.explorer(selection.root.trim());
    activators.context({ root: selection.root, projects: selection.projects });
  };

  return {
    allowPanelLoads(): void {
      panelLoadsAllowed = true;
    },

    allowSessionLoads(): void {
      sessionLoadsAllowed = true;
    },

    panelShown(id: string): void {
      if (!panelLoadsAllowed || !LOADABLE_PANELS.has(id)) return;
      shownPanels.add(id);
      loadPanel(id, readSelection());
    },

    sessionPicked(): void {
      if (!sessionLoadsAllowed) return;
      const selection = readSelection();
      sessionPanelsShown = true;
      loadSessionPanels(selection);
      // Re-point the tabs the user has already opened at the new project. A tab
      // never opened stays untouched, so switching session costs nothing for it.
      for (const id of shownPanels) loadPanel(id, selection);
    },

    loadedPanels(): string[] {
      const loaded = [...shownPanels];
      if (sessionPanelsShown) loaded.push('explorer', 'context');
      return loaded;
    }
  };
}
