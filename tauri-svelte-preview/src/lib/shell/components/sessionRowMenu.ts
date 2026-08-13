/**
 * The right-click roster for a session row.
 *
 * The row draws this list and runs the actions; this module decides what is on
 * it and what is switched off. An item is switched off when the app has no way
 * to carry it out yet — it still shows, with a plain sentence saying why, so
 * the menu does not change shape from row to row.
 */

import type { MyWorkStatus } from './myWorkViewOptions.ts';

export type SessionRowMenuAction =
  | 'rename'
  | 'move-to-project'
  | 'mark-done'
  | 'reopen'
  | 'archive'
  | 'unsettle'
  | 'copy-session-id'
  | 'copy-worktree-path'
  | 'continue-in-new-session'
  | 'open-in-editor'
  | 'reveal-in-finder'
  | 'delete';

export interface SessionRowMenuItem {
  id: SessionRowMenuAction;
  label: string;
  enabled: boolean;
  /** Why the item is off. Shown as its tooltip; absent when it is on. */
  disabledReason?: string;
  /** A destructive item, drawn in the warning tone. */
  destructive?: boolean;
  /** A rule is drawn above this item. */
  startsGroup?: boolean;
}

export interface SessionRowMenuInput {
  /** Where the session sits in the lifecycle right now. */
  status: MyWorkStatus;
  /** The id a person would paste elsewhere, when the session has one. */
  sessionId: string | null;
  /** The checkout the session runs in, when it has one. */
  worktreePath: string | null;
}

const NO_BACKEND = 'Not available yet';

export function sessionRowMenuItems(input: SessionRowMenuInput): SessionRowMenuItem[] {
  const items: SessionRowMenuItem[] = [
    // follow-up: no command writes a session title yet.
    { id: 'rename', label: 'Rename', enabled: false, disabledReason: NO_BACKEND },
    // follow-up: no command reassigns a session to another project yet.
    { id: 'move-to-project', label: 'Move to project', enabled: false, disabledReason: NO_BACKEND }
  ];

  // The lifecycle moves the rail already performs, in the order a session
  // travels: Working to Done to Settled, and back again.
  if (input.status === 'working') {
    items.push({ id: 'mark-done', label: 'Mark done', enabled: true });
  }
  if (input.status === 'done') {
    items.push({ id: 'reopen', label: 'Move back to Working', enabled: true });
  }
  if (input.status === 'settled') {
    items.push({ id: 'unsettle', label: 'Move back to Done', enabled: true });
  } else {
    items.push({ id: 'archive', label: 'Archive (Settle)', enabled: true });
  }

  items.push(
    {
      id: 'copy-session-id',
      label: 'Copy session id',
      enabled: Boolean(input.sessionId),
      disabledReason: input.sessionId ? undefined : 'This session has no id recorded',
      startsGroup: true
    },
    {
      id: 'copy-worktree-path',
      label: 'Copy worktree path',
      enabled: Boolean(input.worktreePath),
      disabledReason: input.worktreePath ? undefined : 'This session has no worktree recorded'
    },
    // follow-up: no command forks a session into a fresh one yet.
    {
      id: 'continue-in-new-session',
      label: 'Continue in new session',
      enabled: false,
      disabledReason: NO_BACKEND
    },
    { id: 'open-in-editor', label: 'Open in editor', enabled: true, startsGroup: true },
    // follow-up: no command opens a path in Finder yet.
    {
      id: 'reveal-in-finder',
      label: 'Reveal in Finder',
      enabled: false,
      disabledReason: NO_BACKEND
    },
    { id: 'delete', label: 'Delete', enabled: true, destructive: true, startsGroup: true }
  );

  return items;
}
