/**
 * workbenchNavigation.ts — the one way a panel asks for something outside
 * itself.
 *
 * Panels never import each other. Opening a file in the editor, showing a
 * file's changes, pointing the browser at a URL, handing an attachment to the
 * composer, starting a session: every one of those crosses a panel boundary,
 * and every one of them goes through this module. The page registers the real
 * work once, on mount, the same register-then-call shape `stackService.ts`
 * already uses; a panel only ever calls the plain functions below.
 *
 * PURE: no DOM, no Svelte, no backend call. Every caller is a no-op — never a
 * throw — while nothing is registered, so a panel that runs before the page is
 * ready simply does nothing.
 */
import type { ConversationAttachment } from './conversation/conversationTypes.ts';
import { requestOpenFile, type OpenFileRequest } from './openFileBus.ts';

/** The four surfaces the center pane's corner tabs switch between. */
export type CenterTabId = 'session' | 'editor' | 'diff' | 'git-history';

/** The nine panels the right column's icon strip switches between. */
export type RightTabId =
  | 'files'
  | 'source-control'
  | 'worktrees'
  | 'run'
  | 'context'
  | 'agents'
  | 'browser'
  | 'history'
  | 'tasks';

export const CENTER_TAB_IDS: readonly CenterTabId[] = [
  'session',
  'editor',
  'diff',
  'git-history'
];

export const RIGHT_TAB_IDS: readonly RightTabId[] = [
  'files',
  'source-control',
  'worktrees',
  'run',
  'context',
  'agents',
  'browser',
  'history',
  'tasks'
];

export interface OpenDiffRequest {
  /** Repository root the file belongs to. */
  projectRoot: string;
  /** Path relative to `projectRoot`. */
  relativePath: string;
}

export interface OpenUrlRequest {
  url: string;
}

export interface ComposerHandoff {
  ownedId: string;
  attachments?: ConversationAttachment[];
  /** Appended to the composer's current draft, on its own line. */
  appendText?: string;
}

export interface SendToSessionRequest {
  ownedId: string;
  /** The turn's words. May be empty when the picture says it all. */
  text: string;
  attachments?: ConversationAttachment[];
}

export interface StartSessionRequest {
  prompt: string;
  cwd: string;
  projectPath: string;
  title: string;
  provider?: 'codex' | 'claude' | 'antigravity';
}

export interface WorkbenchNavigationHandlers {
  showCenterTab(id: CenterTabId): void;
  showRightTab(id: RightTabId): void;
  openDiff(request: OpenDiffRequest): void | Promise<void>;
  openFileTimeline(request: OpenDiffRequest): void | boolean | Promise<void | boolean>;
  openUrl(request: OpenUrlRequest): void | Promise<void>;
  focusComposer(handoff: ComposerHandoff): void | Promise<void>;
  sendToSession(request: SendToSessionRequest): Promise<void>;
  startSession(request: StartSessionRequest): Promise<string | null>;
}

let handlers: Partial<WorkbenchNavigationHandlers> = {};

/** The page registers the real handlers once, on mount. Registering again
 * merges: a caller may hand over one field without clearing the rest. */
export function registerWorkbenchNavigation(next: Partial<WorkbenchNavigationHandlers>): void {
  handlers = { ...handlers, ...next };
}

export function clearWorkbenchNavigation(): void {
  handlers = {};
}

export function showCenterTab(id: CenterTabId): void {
  handlers.showCenterTab?.(id);
}

export function showRightTab(id: RightTabId): void {
  handlers.showRightTab?.(id);
}

/** Open a file and bring the editor forward. The request goes on the existing
 * file bus, which the editor subscribes to on its own. */
export function openFileInEditor(request: OpenFileRequest): void {
  requestOpenFile(request);
  showCenterTab('editor');
}

/** Show one file's changes. The Diff tab comes forward first, so its read belongs
 * to a visible surface. */
export async function openDiffForFile(request: OpenDiffRequest): Promise<void> {
  showCenterTab('diff');
  await handlers.openDiff?.(request);
}

/** Show the paged history for one repository-relative file. */
export async function openFileTimeline(request: OpenDiffRequest): Promise<void> {
  const opened = await handlers.openFileTimeline?.(request);
  if (opened === false) return;
  showCenterTab('git-history');
}

/** Point the browser panel at a URL. The panel is put on screen first: the
 * browser only loads while it is the tab in front. */
export async function openUrlInBrowser(request: OpenUrlRequest): Promise<void> {
  showRightTab('browser');
  await handlers.openUrl?.(request);
}

/** Hand the composer an attachment and some text, then bring the session
 * forward so the reader can see what they were given. */
export async function focusComposerWith(handoff: ComposerHandoff): Promise<void> {
  await handlers.focusComposer?.(handoff);
  showCenterTab('session');
}

/**
 * Send one turn to a session that already exists, and bring it forward so the
 * reader watches it go. Unlike `focusComposerWith` this does not stop at the
 * draft: the panel that calls it has already asked for the send, and a message
 * left staged in a box the reader is not looking at reads as a send that did
 * nothing. Throws whatever the send throws, so the panel can say so.
 */
export async function sendToSession(request: SendToSessionRequest): Promise<void> {
  const send = handlers.sendToSession;
  if (!send) return;
  await send(request);
  showCenterTab('session');
}

/** Start a session at `cwd` and send `prompt` as its first message. Answers
 * with the new session's id, or null when no session could be started. */
export async function startWorkbenchSession(
  request: StartSessionRequest
): Promise<string | null> {
  const start = handlers.startSession;
  if (!start) return null;
  return start(request);
}
