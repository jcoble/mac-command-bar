import type { SourcePreview, SourceRecord } from './sourceData';

export async function listSourceFilesFromTauri(
  root: string,
  query = '',
  limit = 300
): Promise<SourceRecord[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceRecord[]>('list_source_files', {
    root,
    limit,
    query: query.trim() || null
  });
}

export async function readSourceFromTauri(record: SourceRecord): Promise<SourcePreview | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  const preview = await invoke<SourcePreview>('read_source_file', { path: record.path });
  return {
    ...preview,
    relativePath: record.relativePath,
    language: record.language,
    byteCount: record.byteCount
  };
}

function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}
