/**
 * The source records already collected for each project.
 *
 * The explorer owns the scan and this module only shares its finished result.
 * Keeping this pure (no Svelte state and no backend calls) lets the editor use
 * the same project index without importing the explorer UI.
 */
import { normalizeProjectPath, type SourceRecord } from '../sourceData.ts';

const recordsByRoot = new Map<string, readonly SourceRecord[]>();

function rootKey(root: string | null | undefined): string {
  return root ? normalizeProjectPath(root) : '';
}

/** Replace one project's index after a completed scan. */
export function setProjectSourceRecords(
  root: string,
  records: readonly SourceRecord[]
): void {
  const key = rootKey(root);
  if (!key) return;
  recordsByRoot.set(key, records);
}

/** The last completed scan for this project, or an empty list before one. */
export function projectSourceRecords(
  root: string | null | undefined
): readonly SourceRecord[] {
  const key = rootKey(root);
  return key ? (recordsByRoot.get(key) ?? []) : [];
}

/** Drop a stale index when a scan definitively fails. */
export function forgetProjectSourceRecords(root: string): void {
  const key = rootKey(root);
  if (key) recordsByRoot.delete(key);
}

/** Test/support hook; normal project switches deliberately keep their index. */
export function forgetAllProjectSourceRecords(): void {
  recordsByRoot.clear();
}
