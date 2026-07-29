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
 * After that, loading is driven by user actions only: bringing a tab to the
 * front, picking a session, and opening a view of the tool column. A panel that
 * has never been shown never loads, and changing session re-loads only the
 * panels the user has actually opened.
 *
 * Two panels are neither a tab nor always on screen: **source control** and the
 * **context cards**. Both live in the tool column on the right, as two of the
 * four views its icon strip switches between, and the shell opens on neither.
 * So both follow the same principle by a third route — they load when you can
 * actually see them. Picking a session loads one only while it is in view, and
 * opening one loads it if a session is already picked. Out of view they cost
 * nothing; in view they are never stale.
 *
 * What counts as "in view" is not decided here — the tool column knows both
 * halves of it (which view is open, and whether the pane inside is folded) and
 * reports the answer as one yes or no, per view.
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
  /** Whether a session pick would be honoured right now. The page asks before
   * re-opening a session's files, so start-up re-attaching a terminal cannot
   * make the editor read files while everything else is still switched off. */
  loadsAllowed(): boolean;
  /** A center tab came to the front. */
  panelShown(id: string): void;
  /** The user picked a session in the rail. */
  sessionPicked(): void;
  /** Source control came into view, or went out of it. Report where it stands
   * now; the tool column reports it once at start-up too, so this is never
   * guesswork. */
  sourceControlVisible(visible: boolean): void;
  /** The context cards came into view, or went out of it. Same contract as
   * source control above. */
  contextVisible(visible: boolean): void;
  /** Which panels have loaded at least once — for tests and for the report. */
  loadedPanels(): string[];
}

/** Center tabs that have something to load. "session" is the terminal: it is
 * owned by the page's own start-up and must never be re-loaded from here.
 *
 * Source control and the context cards are deliberately absent: both are views
 * of the tool column, not tabs, so nothing ever brings them to the front. Their
 * own rule is in `loadSourceControl` and `loadContext` below. */
const LOADABLE_PANELS = new Set(['editor', 'browser']);

export function createPanelActivation(
  activators: PanelActivators,
  readSelection: () => ProjectSelection
): PanelActivation {
  const shownPanels = new Set<string>();
  let panelLoadsAllowed = false;
  let sessionLoadsAllowed = false;
  let sessionPanelsShown = false;
  /** Can the user see source control right now? */
  let sourceControlInView = false;
  /** The folder source control was last loaded for, or `null` if it never has
   * been. `''` is a real value here — it means "loaded, for no folder". */
  let gitLoadedFor: string | null = null;
  /** Can the user see the context cards right now? */
  let contextInView = false;
  /** Same bookkeeping as `gitLoadedFor`, for the context cards. */
  let contextLoadedFor: string | null = null;

  const loadPanel = (id: string, selection: ProjectSelection): void => {
    const root = selection.root.trim();
    if (id === 'editor') activators.editor(root || null);
    else if (id === 'browser') activators.browser();
  };

  /** Point source control at this folder. A session with no folder still calls
   * it, which is what tells the panel there is no repository to show. */
  const loadSourceControl = (selection: ProjectSelection): void => {
    const root = selection.root.trim();
    gitLoadedFor = root;
    activators.git(root || null);
  };

  /** Point the context cards at this selection. They are machine-wide, so a
   * session with no folder still has something to show. */
  const loadContext = (selection: ProjectSelection): void => {
    contextLoadedFor = selection.root.trim();
    activators.context({ root: selection.root, projects: selection.projects });
  };

  /** The panels that come with the session the user just picked: the file tree,
   * plus source control and the context cards while they are in view. */
  const loadSessionPanels = (selection: ProjectSelection): void => {
    const root = selection.root.trim();
    if (root) activators.explorer(root);
    if (sourceControlInView) loadSourceControl(selection);
    if (contextInView) loadContext(selection);
  };

  return {
    allowPanelLoads(): void {
      panelLoadsAllowed = true;
    },

    allowSessionLoads(): void {
      sessionLoadsAllowed = true;
    },

    loadsAllowed(): boolean {
      return sessionLoadsAllowed;
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

    sourceControlVisible(visible: boolean): void {
      sourceControlInView = visible;
      // Looking away loads nothing, and neither does looking at it before a
      // session has been picked — there is no folder to read yet, and the pick
      // that follows will load it.
      if (!visible || !sessionPanelsShown) return;
      const selection = readSelection();
      // Already showing this folder: coming back to it is not a reason to read
      // the repository again.
      if (gitLoadedFor === selection.root.trim()) return;
      loadSourceControl(selection);
    },

    contextVisible(visible: boolean): void {
      contextInView = visible;
      // Same two reasons to do nothing as source control: looking away loads
      // nothing, and looking at it before a session has been picked has nothing
      // to point at — the pick that follows will load it.
      if (!visible || !sessionPanelsShown) return;
      const selection = readSelection();
      // Already showing this folder: coming back to it is not a reason to read
      // the machine again.
      if (contextLoadedFor === selection.root.trim()) return;
      loadContext(selection);
    },

    loadedPanels(): string[] {
      const loaded = [...shownPanels];
      if (sessionPanelsShown) loaded.push('explorer');
      if (gitLoadedFor !== null) loaded.push('git');
      if (contextLoadedFor !== null) loaded.push('context');
      return loaded;
    }
  };
}
