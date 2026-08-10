/** Pure action rosters for Source Control row context menus. */

export type SourceControlFileAction =
  | 'open-diff'
  | 'stage'
  | 'unstage'
  | 'open-file'
  | 'copy-path';

export type SourceControlFileHandler =
  | 'pick-file'
  | 'run-file-action'
  | 'open-in-editor'
  | 'copy-path';

export interface SourceControlContextMenuItem<Action extends string, Handler extends string> {
  id: Action;
  label: string;
  enabled: boolean;
  handler: Handler;
}

export interface SourceControlFileMenuOptions {
  groupAction: 'stage' | 'unstage';
  canWrite: boolean;
  busy: boolean;
  deleted: boolean;
  hasRoot: boolean;
}

/**
 * Keep the menu tied to the controls already on the changed-file row. The
 * handler names are deliberate testable receipts: the menu dispatcher calls
 * the same component functions as the left-click row and icon buttons.
 */
export function sourceControlFileContextMenuItems(
  options: SourceControlFileMenuOptions
): SourceControlContextMenuItem<SourceControlFileAction, SourceControlFileHandler>[] {
  const indexAction = options.groupAction;
  return [
    {
      id: 'open-diff',
      label: 'Open diff',
      enabled: !options.deleted,
      handler: 'pick-file'
    },
    {
      id: indexAction,
      label: indexAction === 'stage' ? 'Stage changes' : 'Unstage changes',
      enabled: options.canWrite && !options.busy,
      handler: 'run-file-action'
    },
    {
      id: 'open-file',
      label: 'Open file in editor',
      enabled: options.hasRoot && !options.deleted,
      handler: 'open-in-editor'
    },
    {
      id: 'copy-path',
      label: 'Copy path',
      enabled: true,
      handler: 'copy-path'
    }
  ];
}

export type SourceControlCommitAction = 'toggle-commit' | 'copy-hash';
export type SourceControlCommitHandler = 'toggle-commit' | 'copy-hash';

export function sourceControlCommitContextMenuItems(
  expanded: boolean
): SourceControlContextMenuItem<SourceControlCommitAction, SourceControlCommitHandler>[] {
  return [
    {
      id: 'toggle-commit',
      label: expanded ? 'Hide changed files' : 'View changed files',
      enabled: true,
      handler: 'toggle-commit'
    },
    {
      id: 'copy-hash',
      label: 'Copy commit hash',
      enabled: true,
      handler: 'copy-hash'
    }
  ];
}
