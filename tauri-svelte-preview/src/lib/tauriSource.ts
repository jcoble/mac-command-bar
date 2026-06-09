import type { SourcePreview, SourceRecord } from './sourceData';

export async function readSourceFromTauri(record: SourceRecord): Promise<SourcePreview | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourcePreview>('read_source_file', { path: record.path });
}

function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}
