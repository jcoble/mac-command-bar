import { isNativeTauriRuntime } from '$lib/tauriSource.ts';

export interface NotionTaskSettings {
  dataSourceId: string;
  hasToken: boolean;
}

export interface NotionTaskRow {
  sourceTaskId: string;
  title: string;
  project: string;
  status: string;
  priority: string | null;
  assignee: string | null;
  dueDate: string | null;
  sourceUrl: string;
  fetchedAtMs: number;
}

export interface NotionTaskRefreshReceipt {
  count: number;
  fetchedAtMs: number;
}

export interface NotionTaskPage {
  tasks: NotionTaskRow[];
  projects: string[];
  statuses: string[];
  hasMore: boolean;
}

async function invokeDesktop<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isNativeTauriRuntime()) throw new Error('Notion Tasks is available in the desktop app.');
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(command, args);
}

export async function readNotionTaskSettings(): Promise<NotionTaskSettings> {
  return await invokeDesktop('read_notion_task_settings');
}

export async function saveNotionTaskSettings(
  dataSourceId: string,
  token: string
): Promise<NotionTaskSettings> {
  return await invokeDesktop('save_notion_task_settings', { dataSourceId, token });
}

export async function clearNotionTaskSettings(): Promise<void> {
  return await invokeDesktop('clear_notion_task_settings');
}

export async function listNotionTasks(
  offset: number,
  limit = 100,
  search = '',
  project = '',
  status = '',
  sortBy = 'taskNumber',
  sortDirection = 'asc'
): Promise<NotionTaskPage> {
  return await invokeDesktop('list_notion_tasks', {
    offset,
    limit,
    search,
    project,
    status,
    sortBy,
    sortDirection
  });
}

export async function refreshNotionTasks(): Promise<NotionTaskRefreshReceipt> {
  return await invokeDesktop('refresh_notion_tasks');
}
