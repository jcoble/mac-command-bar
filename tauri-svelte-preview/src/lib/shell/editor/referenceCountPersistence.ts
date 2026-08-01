/**
 * Durable count cache for page/webview refreshes.
 *
 * IndexedDB is used instead of localStorage because projects can legitimately
 * produce thousands of lens rows and this app already stores other workspace
 * state in localStorage. Counts expire in the in-memory store; this layer only
 * gives a fresh page the entries it can validate against that same lifetime.
 */
import type { StoredReferenceCount } from './referenceCountBatcher.ts';

// Bump the database name whenever count semantics change. Keeping a previous
// algorithm's fast but wrong numbers is worse than one honest cold recount.
const databaseName = 'mcb-code-intelligence-v2';
const databaseVersion = 1;
const storeName = 'reference-counts';

interface PersistedReferenceCount extends StoredReferenceCount {
  id: string;
}

function entryId(entry: StoredReferenceCount): string {
  return JSON.stringify([entry.project, entry.file, entry.key]);
}

function openDatabase(): Promise<IDBDatabase | null> {
  if (typeof window === 'undefined' || !('indexedDB' in window)) return Promise.resolve(null);
  return new Promise((resolve) => {
    const request = window.indexedDB.open(databaseName, databaseVersion);
    request.onupgradeneeded = () => {
      const database = request.result;
      if (!database.objectStoreNames.contains(storeName)) {
        database.createObjectStore(storeName, { keyPath: 'id' });
      }
    };
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => resolve(null);
    request.onblocked = () => resolve(null);
  });
}

export async function loadPersistedReferenceCounts(): Promise<StoredReferenceCount[]> {
  const database = await openDatabase();
  if (!database) return [];
  return new Promise((resolve) => {
    const transaction = database.transaction(storeName, 'readonly');
    const request = transaction.objectStore(storeName).getAll();
    request.onsuccess = () => {
      resolve(
        (request.result as PersistedReferenceCount[]).map(
          ({ id: _id, ...entry }) => entry
        )
      );
    };
    request.onerror = () => resolve([]);
    transaction.oncomplete = () => database.close();
    transaction.onabort = () => database.close();
  });
}

export async function replacePersistedReferenceCounts(
  entries: readonly StoredReferenceCount[]
): Promise<void> {
  const database = await openDatabase();
  if (!database) return;
  await new Promise<void>((resolve) => {
    const transaction = database.transaction(storeName, 'readwrite');
    const store = transaction.objectStore(storeName);
    store.clear();
    for (const entry of entries) {
      if (entry.project === null) continue;
      const persisted: PersistedReferenceCount = { ...entry, id: entryId(entry) };
      store.put(persisted);
    }
    transaction.oncomplete = () => resolve();
    transaction.onerror = () => resolve();
    transaction.onabort = () => resolve();
  });
  database.close();
}
