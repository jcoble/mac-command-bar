import { isNativeTauriRuntime } from '$lib/tauriSource.ts';

export interface NotionTaskDetailBlock {
  kind: string;
  text: string;
  checked: boolean | null;
  url: string | null;
}

export interface NotionTaskDetail {
  blocks: NotionTaskDetailBlock[];
}

export async function readNotionTaskDetail(
  sourceTaskId: string,
  signal: AbortSignal
): Promise<NotionTaskDetail> {
  if (signal.aborted) throw signal.reason ?? new DOMException('Task closed', 'AbortError');
  if (!isNativeTauriRuntime()) throw new Error('Notion Tasks is available in the desktop app.');
  const { invoke } = await import('@tauri-apps/api/core');
  const detail = await invoke<NotionTaskDetail>('read_notion_task_detail', { sourceTaskId });
  if (signal.aborted) throw signal.reason ?? new DOMException('Task closed', 'AbortError');
  return detail;
}

export async function updateNotionTaskStatus(
  sourceTaskId: string,
  status: string,
  signal: AbortSignal
): Promise<void> {
  if (signal.aborted) throw signal.reason ?? new DOMException('Task closed', 'AbortError');
  if (!isNativeTauriRuntime()) throw new Error('Notion Tasks is available in the desktop app.');
  const { invoke } = await import('@tauri-apps/api/core');
  await invoke('update_notion_task_status', { sourceTaskId, status });
  if (signal.aborted) throw signal.reason ?? new DOMException('Task closed', 'AbortError');
}
