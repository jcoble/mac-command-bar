/**
 * sourceControlFileMenu.ts — the right-click menu on a changed file.
 *
 * Five read-side actions and nothing that changes the repository: staging is a
 * single "Stage All" press in this version, and discarding is not offered from
 * a menu at all. Each action says whether it can run and, when it cannot, why —
 * an item that is off without a reason teaches nothing.
 *
 * Pure: no DOM, no backend, no clipboard. The panel maps an id onto the real
 * work, so this list can be checked in a plain Node test.
 */

import type { ProjectGitFileStatus } from '../../../tauriSource.ts';
import { isGitFileDeleted } from '../../git/gitPanelStore.svelte.ts';

export type SourceControlFileActionId =
  | 'view'
  | 'copy-path'
  | 'copy-relative-path'
  | 'open-in-editor'
  | 'reveal-in-finder';

export interface SourceControlFileAction {
  id: SourceControlFileActionId;
  label: string;
  enabled: boolean;
  /** Why the action is off, in plain words. Null while it is available. */
  disabledReason: string | null;
}

const NO_REPOSITORY = 'This panel is not pointed at a repository folder yet.';
const FILE_IS_GONE = 'This file was deleted, so there is nothing on disk to open.';

function action(
  id: SourceControlFileActionId,
  label: string,
  reason: string | null
): SourceControlFileAction {
  return { id, label, enabled: reason === null, disabledReason: reason };
}

/**
 * The menu for one changed file. `root` is the repository folder the panel is
 * pointed at; an empty one means the panel has no folder yet, which leaves only
 * the repository-relative path answerable.
 */
export function sourceControlFileActions(
  file: ProjectGitFileStatus,
  root: string
): SourceControlFileAction[] {
  const noRoot = root.trim() === '' ? NO_REPOSITORY : null;
  const gone = isGitFileDeleted(file) ? FILE_IS_GONE : null;
  return [
    action('view', 'View changes', noRoot ?? gone),
    action('copy-path', 'Copy path', noRoot),
    action('copy-relative-path', 'Copy relative path', null),
    action('open-in-editor', 'Open in editor', noRoot ?? gone),
    action('reveal-in-finder', 'Reveal in Finder', noRoot ?? gone)
  ];
}
