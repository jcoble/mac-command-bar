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

export function readNotionTaskSettings(): Promise<NotionTaskSettings> {
  return invokeDesktop('read_notion_task_settings');
}

export function saveNotionTaskSettings(
  dataSourceId: string,
  token: string
): Promise<NotionTaskSettings> {
  return invokeDesktop('save_notion_task_settings', { dataSourceId, token });
}

export function clearNotionTaskSettings(): Promise<void> {
  return invokeDesktop('clear_notion_task_settings');
}

export function listNotionTasks(
  offset: number,
  limit = 100,
  search = '',
  project = '',
  status = ''
): Promise<NotionTaskPage> {
  return invokeDesktop('list_notion_tasks', { offset, limit, search, project, status });
}

export function refreshNotionTasks(): Promise<NotionTaskRefreshReceipt> {
  return invokeDesktop('refresh_notion_tasks');
}
