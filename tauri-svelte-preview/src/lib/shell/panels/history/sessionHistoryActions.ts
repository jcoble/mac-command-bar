/**
 * What a session card in the History panel offers, and what it cannot.
 *
 * This file is pure on purpose: it decides which actions a given session
 * supports and, for the ones it does not, writes the sentence the card shows
 * instead. Keeping that here means the answer can be tested without a browser,
 * and means the expanded action row and the more-actions menu cannot drift
 * apart — both read this one list.
 *
 * The rule the whole panel follows: an action is either backed by something
 * real or it is off with a plain-English reason. Nothing pretends.
 */
import type { SessionLibraryRecord } from '../../sessionLibrary/sessionLibraryModel.ts';
import type { StartSessionRequest } from '../../workbenchNavigation.ts';

export type SessionHistoryActionId =
  | 'resume-worktree'
  | 'continue-new-session'
  | 'view-log'
  | 'copy-resume-command'
  | 'open-log'
  | 'reveal-log'
  | 'open-working-directory'
  | 'copy-session-id'
  | 'copy-log-path'
  | 'delete';

export interface SessionHistoryAction {
  id: SessionHistoryActionId;
  label: string;
  enabled: boolean;
  /** Plain English, shown as the item's tooltip when it is off. Null when enabled. */
  disabledReason: string | null;
  destructive: boolean;
}

/** The order a person sees, in the menu and in the expanded action row. */
export const SESSION_HISTORY_ACTION_IDS: readonly SessionHistoryActionId[] = [
  'resume-worktree',
  'continue-new-session',
  'view-log',
  'copy-resume-command',
  'open-log',
  'reveal-log',
  'open-working-directory',
  'copy-session-id',
  'copy-log-path',
  'delete'
];

/**
 * The more-actions menu shows everything except View Log, which already has its
 * own button in the expanded card's action row.
 */
export const SESSION_HISTORY_MENU_ACTION_IDS: readonly SessionHistoryActionId[] =
  SESSION_HISTORY_ACTION_IDS.filter((id) => id !== 'view-log');

const NO_TRANSCRIPT = 'No transcript file was found for this session.';
const NO_FOLDER = 'This session has no folder recorded.';
const NO_RESUME_COMMAND = 'This session has no resume command to copy.';
const NOT_RESUMABLE = 'There is nothing left to resume for this session.';
const NOT_OURS = 'Only sessions started in this app can be deleted.';

/** The one resume command a card offers to copy, when the scan found one. */
export function sessionResumeCommand(record: SessionLibraryRecord): string | null {
  return record.available?.resumeCommands?.[0] ?? null;
}

/** What Copy Session ID puts on the clipboard: the provider's id when there is
 * one, and the row's own identity when there is not. */
export function sessionHistoryIdentity(record: SessionLibraryRecord): string {
  return record.nativeSessionId ?? record.key;
}

/**
 * What Continue in New Session asks for: this card's folder, this card's
 * agent. Both used to be dropped on the way — the folder because the request
 * was built from fields the card had already blanked, and the agent because
 * the request never carried one, so every continued session started as codex.
 */
export function sessionHistoryStartRequest(record: SessionLibraryRecord): StartSessionRequest {
  const folder = record.canonicalCwd?.trim() || record.projectPath?.trim() || '';
  const provider = record.provider === 'claude' || record.provider === 'codex' || record.provider === 'antigravity'
    ? record.provider
    : undefined;
  return {
    prompt: record.firstPrompt ?? '',
    cwd: folder,
    projectPath: record.projectPath?.trim() || folder,
    title: record.title,
    ...(provider ? { provider } : {})
  };
}

function action(
  id: SessionHistoryActionId,
  label: string,
  available: boolean,
  reason: string,
  destructive = false
): SessionHistoryAction {
  return {
    id,
    label,
    enabled: available,
    disabledReason: available ? null : reason,
    destructive
  };
}

/** The ten actions a session card offers, in the order a person sees them. */
export function sessionHistoryActions(record: SessionLibraryRecord): SessionHistoryAction[] {
  const hasLog = Boolean(record.logPath?.trim());
  const hasFolder = Boolean(record.canonicalCwd?.trim());
  // A scanned session is resumed by starting an agent in its folder, so a row
  // whose scan never found one has nothing to resume into. Sessions this app
  // already owns keep their folder in the database and do not need the scan.
  const canResume = Boolean(record.ownedId) || Boolean(record.available && hasFolder);
  const resumeReason = record.available && !hasFolder ? NO_FOLDER : NOT_RESUMABLE;

  return [
    action('resume-worktree', 'Resume in Worktree', canResume, resumeReason),
    action('continue-new-session', 'Continue in New Session', hasFolder, NO_FOLDER),
    action('view-log', 'View Log', hasLog, NO_TRANSCRIPT),
    action(
      'copy-resume-command',
      'Copy Resume Command',
      Boolean(sessionResumeCommand(record)),
      NO_RESUME_COMMAND
    ),
    action('open-log', 'Open Log', hasLog, NO_TRANSCRIPT),
    action('reveal-log', 'Reveal Log', hasLog, NO_TRANSCRIPT),
    action('open-working-directory', 'Open Working Directory', hasFolder, NO_FOLDER),
    // Every row has an identity of some kind, so this one is never off.
    action('copy-session-id', 'Copy Session ID', true, ''),
    action('copy-log-path', 'Copy Log Path', hasLog, NO_TRANSCRIPT),
    action('delete', 'Delete', Boolean(record.ownedId), NOT_OURS, true)
  ];
}
