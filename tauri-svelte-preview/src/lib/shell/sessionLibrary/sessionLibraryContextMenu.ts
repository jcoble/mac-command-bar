/**
 * Session History context-menu roster.
 *
 * This is deliberately pure: the component owns positioning and click handling,
 * while this file keeps the action names and capability truth testable without a
 * browser. Unsupported actions stay visible and explain their FUTURE-NATIVE
 * status instead of pretending that a no-op succeeded.
 */
import type { SessionLibraryRecord } from './sessionLibraryModel.ts';

export type SessionContextMenuAction =
  | 'resume-worktree'
  | 'continue-new-session'
  | 'view-log'
  | 'copy-id'
  | 'archive'
  | 'delete';

export interface SessionContextMenuItem {
  id: SessionContextMenuAction;
  label: string;
  enabled: boolean;
  disabledReason: string | null;
}

export interface SessionContextMenuOptions {
  target: 'session' | 'center-tab';
  record?: SessionLibraryRecord | null;
  canResume?: boolean;
  canArchive?: boolean;
  canDelete?: boolean;
}

const FUTURE_NEW_SESSION = 'Future-native: continue in a new session is not wired yet.';
const FUTURE_LOG = 'Future-native: transcript log viewing is not wired yet.';
const FUTURE_TAB_ACTION = 'Future-native: this action needs a session row target.';
const FUTURE_PROVIDER_MUTATION = 'Future-native: provider session mutation is not available.';

export function sessionContextMenuRoster(options: SessionContextMenuOptions): SessionContextMenuItem[] {
  const isSession = options.target === 'session' && Boolean(options.record);
  const canResume = isSession && options.canResume !== false;
  const canArchive = isSession && options.canArchive !== false && Boolean(options.record?.ownedId);
  const canDelete = isSession && options.canDelete !== false && Boolean(options.record?.ownedId);

  return [
    {
      id: 'resume-worktree',
      label: 'Resume in Worktree',
      enabled: canResume,
      disabledReason: canResume ? null : FUTURE_TAB_ACTION
    },
    {
      id: 'continue-new-session',
      label: 'Continue in New Session',
      enabled: false,
      disabledReason: FUTURE_NEW_SESSION
    },
    {
      id: 'view-log',
      label: 'View Log',
      enabled: false,
      disabledReason: FUTURE_LOG
    },
    {
      id: 'copy-id',
      label: 'Copy ID',
      enabled: isSession || options.target === 'center-tab',
      disabledReason: isSession || options.target === 'center-tab' ? null : FUTURE_TAB_ACTION
    },
    {
      id: 'archive',
      label: 'Archive',
      enabled: canArchive,
      disabledReason: canArchive
        ? null
        : isSession
          ? options.record?.source === 'provider'
            ? FUTURE_PROVIDER_MUTATION
            : 'Archive is not available for this session.'
          : FUTURE_TAB_ACTION
    },
    {
      id: 'delete',
      label: 'Delete',
      enabled: canDelete,
      disabledReason: canDelete
        ? null
        : isSession
          ? options.record?.source === 'provider'
            ? FUTURE_PROVIDER_MUTATION
            : 'Delete is not available for this session.'
          : FUTURE_TAB_ACTION
    }
  ];
}

export const SESSION_CONTEXT_MENU_ACTIONS: readonly SessionContextMenuAction[] = [
  'resume-worktree',
  'continue-new-session',
  'view-log',
  'copy-id',
  'archive',
  'delete'
];
