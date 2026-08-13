/**
 * sessionWorkspaces.ts — what each session had open, kept per session.
 *
 * PURE: no store, no DOM, no backend call. It only turns "this is what the
 * editor and the file tree look like right now" into a small record, and reads
 * those records back out of a storage the caller hands in.
 *
 * The problem it exists for: the editor and the file tree are one of each for
 * the whole shell, so two sessions working in the same repository were sharing
 * one set of tabs. Switching between them lost your place both ways. Giving
 * every session its own record means switching can put back what that session
 * had, and the two stop overwriting each other.
 *
 * The second half of the file is about the same switch costing less. A record
 * is only a list of paths, so putting one back used to mean reading every one
 * of those files off disk again — even the file you were reading thirty seconds
 * ago. So the tabs of the last few sessions are held in memory as well, and a
 * switch back to one of them puts the files themselves back rather than their
 * names. See {@link retainTabs} for what "the last few" means and why.
 *
 * What is NOT here: any decision about when to save or restore. The page owns
 * that (see `+page.svelte`), so nothing in this file can fire on its own.
 */
import { loadLayout, saveLayout, type LayoutStorage } from './layout/layoutStorage.ts';
import type { AgentExecutionOwner } from './ownedSessions.ts';
import type {
  AgentConfigValue,
  AgentWriterLease,
  AgentWriterLeaseTransition
} from './conversation/conversationTypes.ts';

export const SESSION_CONVERSATION_WORKSPACE_VERSION = 1;

export interface SessionConversationWorkspace {
  mode: 'structured' | 'raw';
  version?: number;
  generation?: number;
  owner?: AgentExecutionOwner;
  attachmentIds?: string[];
  config?: Record<string, AgentConfigValue>;
  parentScrollTop?: number;
  childScrollTopById?: Record<string, number>;
  sequence?: number;
  telemetry?: Record<string, AgentConfigValue>;
  writerLease?: AgentWriterLease;
  writerLeaseTransition?: AgentWriterLeaseTransition | null;
  selectedChildId?: string | null;
  scrollTop?: number;
  providerGeneration?: number;
  lastSequence?: number;
  [key: string]: unknown;
}

export interface SessionBrowserWorkspace {
  url: string;
  inputUrl: string;
  activated: boolean;
}

export interface SessionCenterWorkspace {
  activePanelId: string | null;
  layout: object;
}

/** What one session had open, small enough to store for every session at once. */
export interface SessionWorkspaceSnapshot {
  /** Editor tabs in strip order, newest at the end. */
  openPaths: string[];
  /** The tab that was showing. */
  activePath: string | null;
  /** The highlighted file in the tree. */
  selectedPath: string | null;
  /** Scroll offset of the tree, in pixels. */
  scrollTop: number;
  /**
   * The file the Diff tab was showing, as a path relative to the repository, or
   * null when it was showing nothing.
   */
  diffPath: string | null;
  /**
   * The project folder `diffPath` is inside, or null when there is no diff.
   *
   * Stored next to the path because a path on its own cannot be checked: the
   * Diff tab is one tab for the whole shell, and "src/lib/index.ts" names a real
   * file in most projects. Keeping the folder it came from is what lets
   * {@link diffPathFor} tell "this is your file" from "this is the last
   * project's file that happens to have the same name".
   */
  diffRoot: string | null;
  /** Small conversation UI state only. Provider history is never stored here. */
  conversation?: SessionConversationWorkspace;
  /** The embedded Browser state owned by this session. */
  browser?: SessionBrowserWorkspace;
  /** The center Dockview arrangement and active tab owned by this session. */
  center?: SessionCenterWorkspace;
}

export const SESSION_WORKSPACES_STORAGE_KEY = 'mac-command-bar.next.session-workspaces';

/**
 * How many tabs a session's record keeps. Twelve is more than anyone has open
 * at once and far less than the hundreds a long session can accumulate — the
 * point of the limit is that every session on the rail is stored together, so
 * one busy session must not be able to fill the browser's storage on its own.
 */
export const OPEN_PATHS_CAP = 12;

/** The strip, trimmed to the cap: the most recent tabs, which are the ones at
 * the end. The tab that was showing is always kept, even when it is older than
 * all of them — coming back to a session and not finding the file you left on
 * screen is the one thing nobody would forgive. It keeps its place in the
 * strip; the oldest of the recent tabs makes room for it. */
function cappedPaths(paths: string[], activePath: string | null): string[] {
  const unique: string[] = [];
  for (const path of paths) {
    if (typeof path === 'string' && path && !unique.includes(path)) unique.push(path);
  }
  if (unique.length <= OPEN_PATHS_CAP) return unique;

  const recent = unique.slice(unique.length - OPEN_PATHS_CAP);
  if (!activePath || recent.includes(activePath) || !unique.includes(activePath)) return recent;
  return [activePath, ...recent.slice(1)];
}

/** Turn what is on screen into a record. Nothing is read from anywhere: the
 * caller passes the editor's and the tree's own state straight in. */
export function captureWorkspace(input: {
  openFiles: { path: string }[];
  activePath: string | null;
  selectedPath: string | null;
  scrollTop: number;
  /** What the Diff tab is showing, if the caller passes it. Optional so the
   * page can start recording it separately from the rest of this record; a
   * caller that says nothing gets "no diff", which is what an older stored
   * record means too. */
  diffPath?: string | null;
  /** The project folder that diff came from. See `diffRoot` on the record. */
  diffRoot?: string | null;
  conversation?: SessionConversationWorkspace;
  browser?: SessionBrowserWorkspace;
  center?: SessionCenterWorkspace | null;
}): SessionWorkspaceSnapshot {
  const activePath = input.activePath ?? null;
  // A path with no folder cannot be checked against the session being restored,
  // and the safe reading of an unknown owner is "not this session's" — so half
  // an answer is stored as no answer rather than as a diff we would then show
  // to the wrong project.
  const diffPath = pathOf(input.diffPath);
  const diffRoot = pathOf(input.diffRoot);
  const bothKnown = diffPath !== null && diffRoot !== null;
  const snapshot: SessionWorkspaceSnapshot = {
    openPaths: cappedPaths(
      input.openFiles.map((file) => file.path),
      activePath
    ),
    activePath,
    selectedPath: input.selectedPath ?? null,
    scrollTop: Math.max(0, input.scrollTop),
    diffPath: bothKnown ? diffPath : null,
    diffRoot: bothKnown ? diffRoot : null
  };
  if (input.conversation) snapshot.conversation = normalizeConversation(input.conversation);
  if (input.browser) snapshot.browser = normalizeBrowser(input.browser);
  if (input.center) snapshot.center = normalizeCenter(input.center) ?? undefined;
  return snapshot;
}

function stringsOf(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return value.filter((entry): entry is string => typeof entry === 'string' && entry.length > 0);
}

function pathOf(value: unknown): string | null {
  return typeof value === 'string' && value.length > 0 ? value : null;
}

function nonNegativeInteger(value: unknown): number | undefined {
  return typeof value === 'number' && Number.isInteger(value) && value >= 0 ? value : undefined;
}

function normalizeConversation(value: unknown): SessionConversationWorkspace {
  const entry = value && typeof value === 'object' && !Array.isArray(value)
    ? value as Record<string, unknown>
    : {};
  const preserved = { ...entry };
  for (const key of [
    'mode', 'draft', 'version', 'generation', 'owner', 'attachmentIds', 'config',
    'parentScrollTop', 'childScrollTopById', 'sequence', 'telemetry', 'writerLease',
    'writerLeaseTransition', 'selectedChildId', 'scrollTop', 'providerGeneration', 'lastSequence'
  ]) delete preserved[key];
  return {
    ...preserved,
    mode: entry.mode === 'raw' ? 'raw' : 'structured',
    ...(nonNegativeInteger(entry.version) !== undefined
      ? { version: nonNegativeInteger(entry.version) }
      : {}),
    ...(nonNegativeInteger(entry.generation) !== undefined
      ? { generation: nonNegativeInteger(entry.generation) }
      : {}),
    ...(isExecutionOwner(entry.owner) ? { owner: entry.owner } : {}),
    ...(Array.isArray(entry.attachmentIds)
      ? { attachmentIds: stringsOf(entry.attachmentIds) }
      : {}),
    ...(isRecord(entry.config) ? { config: entry.config as Record<string, AgentConfigValue> } : {}),
    ...(typeof entry.parentScrollTop === 'number' && entry.parentScrollTop >= 0
      ? { parentScrollTop: entry.parentScrollTop }
      : {}),
    ...(isScrollMap(entry.childScrollTopById)
      ? { childScrollTopById: entry.childScrollTopById }
      : {}),
    ...(nonNegativeInteger(entry.sequence) !== undefined
      ? { sequence: nonNegativeInteger(entry.sequence) }
      : {}),
    ...(isRecord(entry.telemetry)
      ? { telemetry: entry.telemetry as Record<string, AgentConfigValue> }
      : {}),
    ...(isWriterLease(entry.writerLease) ? { writerLease: entry.writerLease } : {}),
    ...(entry.writerLeaseTransition === null || isWriterLeaseTransition(entry.writerLeaseTransition)
      ? { writerLeaseTransition: entry.writerLeaseTransition }
      : {}),
    ...(typeof entry.selectedChildId === 'string' || entry.selectedChildId === null
      ? { selectedChildId: entry.selectedChildId as string | null }
      : {}),
    ...(typeof entry.scrollTop === 'number' && entry.scrollTop >= 0
      ? { scrollTop: entry.scrollTop }
      : {}),
    ...(nonNegativeInteger(entry.providerGeneration) !== undefined
      ? { providerGeneration: nonNegativeInteger(entry.providerGeneration) }
      : {}),
    ...(nonNegativeInteger(entry.lastSequence) !== undefined
      ? { lastSequence: nonNegativeInteger(entry.lastSequence) }
      : {})
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value);
}

function isExecutionOwner(value: unknown): value is AgentExecutionOwner {
  return [
    'structured',
    'terminal',
    'transitioning-to-structured',
    'transitioning-to-terminal',
    'stopped'
  ].includes(value as string);
}

function isScrollMap(value: unknown): value is Record<string, number> {
  return isRecord(value) && Object.values(value).every(
    (entry) => typeof entry === 'number' && entry >= 0
  );
}

function isWriterLease(value: unknown): value is AgentWriterLease {
  if (!isRecord(value)) return false;
  return typeof value.ownedId === 'string'
    && nonNegativeInteger(value.generation) !== undefined
    && ['structured', 'terminal', 'none'].includes(value.owner as string);
}

function isWriterLeaseTransition(value: unknown): value is AgentWriterLeaseTransition {
  if (!isRecord(value)) return false;
  return typeof value.ownedId === 'string'
    && nonNegativeInteger(value.generation) !== undefined
    && ['structured', 'terminal', 'none'].includes(value.from as string)
    && ['structured', 'terminal', 'none'].includes(value.to as string)
    && ['requested', 'committed', 'failed'].includes(value.state as string)
    && (value.error === undefined || typeof value.error === 'string');
}

function normalizeBrowser(value: unknown): SessionBrowserWorkspace {
  const entry = value && typeof value === 'object' && !Array.isArray(value)
    ? value as Record<string, unknown>
    : {};
  return {
    url: typeof entry.url === 'string' ? entry.url : '',
    inputUrl: typeof entry.inputUrl === 'string' ? entry.inputUrl : '',
    activated: entry.activated === true
  };
}

function normalizeCenter(value: unknown): SessionCenterWorkspace | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null;
  const entry = value as Record<string, unknown>;
  if (!entry.layout || typeof entry.layout !== 'object' || Array.isArray(entry.layout)) return null;
  return {
    activePanelId: typeof entry.activePanelId === 'string' ? entry.activePanelId : null,
    layout: entry.layout as object
  };
}

/** One stored entry, or null if it is not a record at all. Fields of the wrong
 * type become their empty version rather than sinking the whole entry: a
 * half-corrupt record still restores most of a session. */
function snapshotOf(value: unknown): SessionWorkspaceSnapshot | null {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null;

  const entry = value as Record<string, unknown>;
  const activePath = pathOf(entry.activePath);
  const scrollTop = typeof entry.scrollTop === 'number' && entry.scrollTop > 0 ? entry.scrollTop : 0;
  // Records written before the Diff tab was remembered have neither field, and
  // they read back as "this session was not looking at a diff" — which is the
  // right answer for them and the safe one in general.
  const diffPath = pathOf(entry.diffPath);
  const diffRoot = pathOf(entry.diffRoot);
  const bothKnown = diffPath !== null && diffRoot !== null;
  const snapshot: SessionWorkspaceSnapshot = {
    openPaths: cappedPaths(stringsOf(entry.openPaths), activePath),
    activePath,
    selectedPath: pathOf(entry.selectedPath),
    scrollTop,
    diffPath: bothKnown ? diffPath : null,
    diffRoot: bothKnown ? diffRoot : null
  };
  if ('conversation' in entry) snapshot.conversation = normalizeConversation(entry.conversation);
  if ('browser' in entry) snapshot.browser = normalizeBrowser(entry.browser);
  const center = normalizeCenter(entry.center);
  if (center) snapshot.center = center;
  return snapshot;
}

/** Trailing slashes make two spellings of one folder look different. */
function sameFolder(left: string, right: string): boolean {
  return left.replace(/\/+$/, '') === right.replace(/\/+$/, '');
}

/**
 * The file the Diff tab should show for a session whose project folder is
 * `root`, or null when it should show nothing.
 *
 * This is the answer to the bug where switching session left the Diff tab
 * showing a file from the project you had just left. There are three ways to get
 * null, and all of them mean "clear the tab": the session has no stored record
 * at all, it was not looking at a diff, or the diff it was looking at belongs to
 * a different project. Only a diff from this session's own project comes back.
 *
 * A session with no folder (`root` empty) can own no diff, so it always clears.
 */
export function diffPathFor(
  snapshot: SessionWorkspaceSnapshot | null | undefined,
  root: string | null | undefined
): string | null {
  const folder = (root ?? '').trim();
  if (!snapshot || !folder || !snapshot.diffPath || !snapshot.diffRoot) return null;
  return sameFolder(snapshot.diffRoot, folder) ? snapshot.diffPath : null;
}

/** Every session's record, by owned id. Anything unreadable — no value, broken
 * JSON, a value that is not a map — comes back as "no session has one yet". */
export function readWorkspaces(storage: LayoutStorage): Record<string, SessionWorkspaceSnapshot> {
  const stored = loadLayout<unknown>(storage, SESSION_WORKSPACES_STORAGE_KEY);
  if (!stored || typeof stored !== 'object' || Array.isArray(stored)) return {};

  const all: Record<string, SessionWorkspaceSnapshot> = {};
  for (const [ownedId, value] of Object.entries(stored)) {
    const snapshot = snapshotOf(value);
    if (snapshot) all[ownedId] = snapshot;
  }
  return all;
}

/** False means the write was refused (a full storage). Nothing is retried: the
 * cost is one session coming back to an empty editor after a reload. */
export function writeWorkspaces(
  storage: LayoutStorage,
  all: Record<string, SessionWorkspaceSnapshot>
): boolean {
  return saveLayout(storage, SESSION_WORKSPACES_STORAGE_KEY, all);
}

/** Drop the records of sessions the rail no longer has, so a removed session
 * takes its workspace with it and the store cannot grow forever. */
export function pruneWorkspaces(
  all: Record<string, SessionWorkspaceSnapshot>,
  keepOwnedIds: string[]
): Record<string, SessionWorkspaceSnapshot> {
  const keep = new Set(keepOwnedIds);
  const kept: Record<string, SessionWorkspaceSnapshot> = {};
  for (const [ownedId, snapshot] of Object.entries(all)) {
    if (keep.has(ownedId)) kept[ownedId] = snapshot;
  }
  return kept;
}

/* ---------------------------------------------------------------------------
 * Tabs held in memory, so switching back to a session does not read its files
 * off disk all over again.
 * ------------------------------------------------------------------------- */

/** What this file needs of an editor tab to decide about it. The editor's own
 * tab is wider than this, and everything else on it — the language, where the
 * reader had scrolled to — is carried along untouched. */
export interface RetainedTab {
  /** Which file the tab is for. */
  path: string;
  /** The file's contents once read, and null until then. */
  preview: unknown;
  /** A read is in flight. */
  loading: boolean;
  /** Why the last read failed, in plain words, or null. */
  error: string | null;
}

/** The tabs of the sessions still held in memory. */
export interface RetainedWorkspaces<Tab extends RetainedTab> {
  /** The sessions held, the one used longest ago first — so the one to let go
   * of is always the one at the front. */
  leastRecentFirst: string[];
  /** The tabs each held session left behind. */
  tabsByOwnedId: Record<string, Tab[]>;
}

/**
 * How many sessions keep their tabs in memory at once.
 *
 * Three covers the switching people actually do — the session you are on and
 * the two you keep going back to — and it is what stops this from growing
 * without limit: a held session holds the full text of every file it had open.
 * The fourth session to be left behind is the one let go of, and coming back to
 * that one reads its files from disk exactly as it did before any of this.
 */
export const RETAINED_WORKSPACES_CAP = 3;

/** Nothing held yet. */
export function emptyRetainedWorkspaces<Tab extends RetainedTab>(): RetainedWorkspaces<Tab> {
  return { leastRecentFirst: [], tabsByOwnedId: {} };
}

/** The queue and the tabs, with everything past the cap let go of. */
function withinCap<Tab extends RetainedTab>(
  leastRecentFirst: string[],
  tabsByOwnedId: Record<string, Tab[]>,
  cap: number
): RetainedWorkspaces<Tab> {
  const queue = [...leastRecentFirst];
  const tabs = { ...tabsByOwnedId };
  while (queue.length > Math.max(0, cap)) {
    const letGo = queue.shift();
    if (letGo !== undefined) delete tabs[letGo];
  }
  return { leastRecentFirst: queue, tabsByOwnedId: tabs };
}

/**
 * A tab held with its read unfinished comes back as a tab nobody has read yet.
 *
 * Leaving a session abandons the reads it had in flight — the editor throws
 * away an answer for a file it can no longer see — so a tab held as "still
 * loading" would come back waiting for something that will never arrive, and
 * nothing would ever start the read again. A tab whose read failed is cleared
 * too: coming back to a session used to read every one of its files, so a
 * failure got a second chance on every switch, and it still should.
 */
function readyToRead<Tab extends RetainedTab>(tab: Tab): Tab {
  const contentsInMemory = tab.preview !== null && tab.preview !== undefined;
  if (contentsInMemory || (!tab.loading && tab.error === null)) return tab;
  return { ...tab, loading: false, error: null };
}

/**
 * Hold the tabs a session is leaving behind, and count that as using it.
 *
 * A session leaving with an empty editor is not held at all: there is nothing
 * to put back, and letting its place go means the three places belong to
 * sessions that do have files open.
 *
 * The store handed in is never changed — a new one comes back — so the page can
 * keep using the old one right up until it assigns the new.
 */
export function retainTabs<Tab extends RetainedTab>(
  retained: RetainedWorkspaces<Tab>,
  ownedId: string,
  tabs: readonly Tab[],
  cap: number = RETAINED_WORKSPACES_CAP
): RetainedWorkspaces<Tab> {
  const others = retained.leastRecentFirst.filter((id) => id !== ownedId);
  const held = { ...retained.tabsByOwnedId };
  delete held[ownedId];
  if (tabs.length === 0) return withinCap(others, held, cap);
  held[ownedId] = tabs.map(readyToRead);
  return withinCap([...others, ownedId], held, cap);
}

/**
 * The tabs held for a session, or null when none are — which is the answer for
 * a session opened for the first time this run, and for one whose place was
 * given up to a newer session.
 *
 * Taking them counts as using the session, so the one you keep coming back to
 * is never the one let go of.
 */
export function takeRetainedTabs<Tab extends RetainedTab>(
  retained: RetainedWorkspaces<Tab>,
  ownedId: string
): { retained: RetainedWorkspaces<Tab>; tabs: Tab[] | null } {
  const tabs = retained.tabsByOwnedId[ownedId];
  if (!tabs) return { retained, tabs: null };
  return {
    retained: {
      leastRecentFirst: [...retained.leastRecentFirst.filter((id) => id !== ownedId), ownedId],
      tabsByOwnedId: retained.tabsByOwnedId
    },
    tabs
  };
}

/** What the page has to do to put a session's editor back the way it was. */
export interface WorkspaceRestorePlan<Tab extends RetainedTab> {
  /** Tabs to put straight back on screen, contents and all, or null when this
   * session's files have to be read from disk. */
  restoredTabs: Tab[] | null;
  /** Files still to be asked for through the open-file bus, in strip order. */
  pathsToOpen: string[];
  /** The file to leave in front, or null when there is nothing to show. */
  activePath: string | null;
}

/**
 * Work out how to put a session's editor back, from its stored record and
 * whatever tabs are still held for it.
 *
 * With tabs held, they go back as they are and only a file the record names
 * that they do not have is read — which happens when the strip was longer than
 * the twelve a record keeps, or when the record outlived the tabs.
 *
 * With nothing held, this is exactly what the page did before: every stored path
 * is asked for, in order.
 *
 * A held tab carries the file as it was when the reader left it, not as it is
 * on disk now. That is what an editor does with a file you have open, and it is
 * the whole saving here; a file that has changed underneath is picked up the
 * next time that tab is actually read.
 */
export function planWorkspaceRestore<Tab extends RetainedTab>(
  snapshot: SessionWorkspaceSnapshot | null | undefined,
  retainedTabs: readonly Tab[] | null | undefined
): WorkspaceRestorePlan<Tab> {
  const restoredTabs = retainedTabs && retainedTabs.length > 0 ? [...retainedTabs] : null;
  const heldPaths = restoredTabs ? restoredTabs.map((tab) => tab.path) : [];
  const held = new Set(heldPaths);
  const pathsToOpen = (snapshot?.openPaths ?? []).filter((path) => !held.has(path));
  // No record of which file was showing: the last tab in the strip is the one
  // in front, which is where opening them one after another used to leave it.
  const strip = [...heldPaths, ...pathsToOpen];
  return {
    restoredTabs,
    pathsToOpen,
    activePath: snapshot?.activePath ?? strip[strip.length - 1] ?? null
  };
}
