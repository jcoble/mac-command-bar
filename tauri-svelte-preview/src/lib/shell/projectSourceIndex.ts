/**
 * The source records already collected for each project.
 *
 * The explorer owns the scan and this module only shares its finished result.
 * Keeping this pure (no Svelte state and no backend calls) lets the editor use
 * the same project index without importing the explorer UI.
 */
import { normalizeProjectPath, type SourceRecord } from '../sourceData.ts';

const recordsByRoot = new Map<string, readonly SourceRecord[]>();
/**
 * How many projects keep their index. One record per file in a repository, so
 * every root visited in one run would otherwise be retained until reload; three
 * covers the current project and switching back and forth with two others.
 */
const rootLimit = 3;

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
  // Delete first: `Map.set` on a key that is already there does not move it to
  // the end, and the oldest key is the one evicted.
  recordsByRoot.delete(key);
  recordsByRoot.set(key, records);
  while (recordsByRoot.size > rootLimit) {
    const oldest = recordsByRoot.keys().next().value;
    if (oldest === undefined) break;
    recordsByRoot.delete(oldest);
  }
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

/**
 * Test/support hook; a normal project switch keeps its index, and only the
 * fourth-oldest project loses one.
 */
export function forgetAllProjectSourceRecords(): void {
  recordsByRoot.clear();
}
