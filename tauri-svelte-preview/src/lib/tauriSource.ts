import type { SourcePreview, SourceRecord } from './sourceData';

export const defaultSourceScanLimit = 2_000;

export async function listSourceFilesFromTauri(
  root: string,
  query = '',
  limit = defaultSourceScanLimit
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

export async function openSourceFileFromTauri(path: string): Promise<boolean> {
  return runSourceFileAction('open_source_file', path);
}

export async function revealSourceFileFromTauri(path: string): Promise<boolean> {
  return runSourceFileAction('reveal_source_file', path);
}

async function runSourceFileAction(command: string, path: string): Promise<boolean> {
  if (!isTauriRuntime()) {
    return false;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  await invoke(command, { path });
  return true;
}

function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}
