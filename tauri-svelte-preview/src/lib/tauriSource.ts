import type {
  SourceDefinitionTarget,
  SourcePreview,
  SourceRecord,
  SourceReferenceTarget,
  SourceScanResult,
  SourceSearchMatch
} from './sourceData';

export const defaultSourceScanLimit = 2_000;
export const expandedSourceScanLimit = 5_000;
export const nativeSourceScanProgressEvent = 'source_scan_progress';

export type NativeSourceScanProgress = {
  scanId: string;
  visitedEntries: number;
  matchedFiles: number;
};

export type ProjectGitFileStatus = {
  relativePath: string;
  indexStatus: string;
  worktreeStatus: string;
  status: string;
  badge: string;
};

export type ProjectGitStatus = {
  branch: string | null;
  ahead: number;
  behind: number;
  files: ProjectGitFileStatus[];
};

export function createSourceScanId(): string {
  return `source-scan-${Date.now()}-${Math.random().toString(36).slice(2, 9)}`;
}

export async function listSourceFilesFromTauri(
  root: string,
  query = '',
  limit = defaultSourceScanLimit,
  scanId: string | null = null
): Promise<SourceScanResult | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceScanResult>('list_source_files', {
    root,
    limit,
    query: query.trim() || null,
    scanId
  });
}

export async function cancelSourceScanFromTauri(scanId: string): Promise<boolean> {
  if (!isTauriRuntime() || !scanId.trim()) {
    return false;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<boolean>('cancel_source_scan', { scanId });
}

export async function listenToSourceScanProgress(
  handler: (progress: NativeSourceScanProgress) => void
): Promise<(() => void) | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { listen } = await import('@tauri-apps/api/event');
  return listen<NativeSourceScanProgress>(nativeSourceScanProgressEvent, (event) => {
    handler(event.payload);
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

export async function writeSourceToTauri(
  record: SourceRecord,
  content: string
): Promise<SourcePreview | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  const preview = await invoke<SourcePreview>('write_source_file', {
    path: record.path,
    content
  });
  return {
    ...preview,
    relativePath: record.relativePath,
    language: record.language
  };
}

export async function openSourceFileFromTauri(path: string): Promise<boolean> {
  return runSourceFileAction('open_source_file', path);
}

export async function revealSourceFileFromTauri(path: string): Promise<boolean> {
  return runSourceFileAction('reveal_source_file', path);
}

export async function readProjectGitStatusFromTauri(
  root: string
): Promise<ProjectGitStatus | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<ProjectGitStatus>('project_git_status', { root });
}

export async function searchSourceFilesFromTauri(
  records: SourceRecord[],
  query: string,
  limit = 50
): Promise<SourceSearchMatch[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceSearchMatch[]>('search_source_files', {
    records,
    query,
    limit
  });
}

export async function findSourceDefinitionsFromTauri(
  records: SourceRecord[],
  symbolName: string,
  limit = 20
): Promise<SourceDefinitionTarget[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceDefinitionTarget[]>('find_source_definitions', {
    records,
    symbolName,
    limit
  });
}

export async function findSourceReferencesFromTauri(
  records: SourceRecord[],
  symbolName: string,
  limit = 50
): Promise<SourceReferenceTarget[] | null> {
  if (!isTauriRuntime()) {
    return null;
  }

  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<SourceReferenceTarget[]>('find_source_references', {
    records,
    symbolName,
    limit
  });
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
