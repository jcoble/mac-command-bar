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
 * panels the user has actually opened — and of those, only the ones the change
 * tells something new. A panel is already showing the folder the session you
 * just picked is in when that session shares a project or a checkout with the
 * one you left, and reading it again would throw the panel's work away and
 * rebuild the same answer. So each panel remembers the folder it was last
 * loaded for, and a pick that does not change it loads nothing.
 *
 * Four panels are neither a tab nor always on screen: **source control**, the
 * **worktree manager**, the **stacks pane** and the **context cards**. All four
 * live in the tool column on the right, as four of the five views its icon strip
 * switches between, and the shell opens on none of them. So all four follow the
 * same principle by a third route — they load when you can actually see them.
 * Picking a session loads one only while it is in view, and opening one loads it
 * if a session is already picked. Out of view they cost nothing; in view they
 * are never stale.
 *
 * What counts as "in view" is not decided here — the tool column knows both
 * halves of it (which view is open, and whether the pane inside is folded) and
 * reports the answer as one yes or no, per view.
 *
 * The **Problems panel** in the bottom dock has the same route and a harder
 * problem: the dock is on screen from the moment the shell opens, so "in view"
 * cannot mean "mounted" there. `problemsVisible(true)` must only ever be
 * reported from something the user did. Nothing in the shell reports it today —
 * the panel's own Refresh button is the only way in — but the route is here and
 * tested, so the day the dock grows tabs or a fold, one call is all it takes.
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
  worktrees(selection: ProjectSelection): void;
  stacks(root: string | null): void;
  problems(root: string | null): void;
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
  /** The worktree manager came into view, or went out of it. Same contract. */
  worktreesVisible(visible: boolean): void;
  /** The stacks pane came into view, or went out of it. Same contract. */
  stacksVisible(visible: boolean): void;
  /** The Problems panel came into view, or went out of it. Same contract as
   * source control above — but see the note at the top of this file: the bottom
   * dock is on screen at launch, so this may only ever be called for something
   * the user did. */
  problemsVisible(visible: boolean): void;
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
  /** Can the user see the worktree manager right now? */
  let worktreesInView = false;
  /** Same bookkeeping as `gitLoadedFor`, for the worktree manager. */
  let worktreesLoadedFor: string | null = null;
  /** Can the user see the stacks pane right now? */
  let stacksInView = false;
  /** Same bookkeeping as `gitLoadedFor`, for the stacks pane. */
  let stacksLoadedFor: string | null = null;
  /** Can the user see the Problems panel right now? */
  let problemsInView = false;
  /** Same bookkeeping as `gitLoadedFor`, for the Problems panel. */
  let problemsLoadedFor: string | null = null;

  /** What a center tab is pointed at, as the one thing that can change for it.
   * The editor follows the project; the browser panel shows a web page rather
   * than a project, so nothing about a session is an input of its at all — and
   * the empty folder it always answers is what makes a pick a no-op for it. */
  const panelInput = (id: string, selection: ProjectSelection): string =>
    id === 'browser' ? '' : selection.root.trim();

  /** The folder each center tab was last loaded for. A tab that has never
   * loaded is absent, which is different from one loaded for no folder. */
  const panelLoadedFor = new Map<string, string>();

  const loadPanel = (id: string, selection: ProjectSelection): void => {
    const root = selection.root.trim();
    panelLoadedFor.set(id, panelInput(id, selection));
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

  /** Point the worktree manager at this selection. It needs the whole selection
   * rather than a bare folder: it lists the checkouts of the project the session
   * is in, and joins the shell's own sessions onto them. */
  const loadWorktrees = (selection: ProjectSelection): void => {
    worktreesLoadedFor = selection.root.trim();
    activators.worktrees({ root: selection.root, projects: selection.projects });
  };

  /** Point the stacks pane at this folder. The saved stacks are per project. */
  const loadStacks = (selection: ProjectSelection): void => {
    const root = selection.root.trim();
    stacksLoadedFor = root;
    activators.stacks(root || null);
  };

  /** Point the Problems panel at this folder. */
  const loadProblems = (selection: ProjectSelection): void => {
    const root = selection.root.trim();
    problemsLoadedFor = root;
    activators.problems(root || null);
  };

  /** The panels that come with the session the user just picked: the file tree,
   * plus every view-gated panel that is in view AND is not already showing this
   * folder — the same "coming back to it is not a reason to read it again" rule
   * the visibility reports below have always used.
   *
   * The file tree is the exception, and deliberately: its own service already
   * refuses to re-list a folder it is showing, and it is also the one that
   * retries after a scan that failed. Refusing the call here would take that
   * retry away and give nothing back. */
  const loadSessionPanels = (selection: ProjectSelection): void => {
    const root = selection.root.trim();
    if (root) activators.explorer(root);
    if (sourceControlInView && gitLoadedFor !== root) loadSourceControl(selection);
    if (contextInView && contextLoadedFor !== root) loadContext(selection);
    if (worktreesInView && worktreesLoadedFor !== root) loadWorktrees(selection);
    if (stacksInView && stacksLoadedFor !== root) loadStacks(selection);
    if (problemsInView && problemsLoadedFor !== root) loadProblems(selection);
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
      // never opened stays untouched, so switching session costs nothing for it
      // — and neither does a tab already pointed at this project, which is what
      // makes switching between two sessions in one repository cheap.
      for (const id of shownPanels) {
        if (panelLoadedFor.get(id) !== panelInput(id, selection)) loadPanel(id, selection);
      }
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

    worktreesVisible(visible: boolean): void {
      worktreesInView = visible;
      if (!visible || !sessionPanelsShown) return;
      const selection = readSelection();
      if (worktreesLoadedFor === selection.root.trim()) return;
      loadWorktrees(selection);
    },

    stacksVisible(visible: boolean): void {
      stacksInView = visible;
      if (!visible || !sessionPanelsShown) return;
      const selection = readSelection();
      if (stacksLoadedFor === selection.root.trim()) return;
      loadStacks(selection);
    },

    problemsVisible(visible: boolean): void {
      problemsInView = visible;
      if (!visible || !sessionPanelsShown) return;
      const selection = readSelection();
      if (problemsLoadedFor === selection.root.trim()) return;
      loadProblems(selection);
    },

    loadedPanels(): string[] {
      const loaded = [...shownPanels];
      if (sessionPanelsShown) loaded.push('explorer');
      if (gitLoadedFor !== null) loaded.push('git');
      if (contextLoadedFor !== null) loaded.push('context');
      if (worktreesLoadedFor !== null) loaded.push('worktrees');
      if (stacksLoadedFor !== null) loaded.push('stacks');
      if (problemsLoadedFor !== null) loaded.push('problems');
      return loaded;
    }
  };
}
