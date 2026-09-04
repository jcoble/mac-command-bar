import { isNativeTauriRuntime } from '$lib/tauriSource.ts';

import type { NotionTaskSettings } from './notionTasks.ts';

interface NotionOAuthAttempt {
  authorizationUrl: string;
  state: string;
  verifier: string;
}

interface NotionOAuthClaim {
  status: 'pending' | 'connected';
  settings: NotionTaskSettings | null;
}

const CLAIM_INTERVAL_MS = 1_000;
const LOGIN_TIMEOUT_MS = 5 * 60 * 1_000;

function assertActive(signal: AbortSignal): void {
  if (signal.aborted) throw signal.reason ?? new DOMException('Notion sign-in stopped', 'AbortError');
}

async function invokeDesktop<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!isNativeTauriRuntime()) throw new Error('Notion sign-in is available in the desktop app.');
  const { invoke } = await import('@tauri-apps/api/core');
  return await invoke<T>(command, args);
}

async function waitForNextClaim(signal: AbortSignal): Promise<void> {
  await new Promise<void>((resolve, reject) => {
    const finish = (): void => {
      signal.removeEventListener('abort', stop);
      resolve();
    };
    const stop = (): void => {
      window.clearTimeout(timeout);
      signal.removeEventListener('abort', stop);
      reject(signal.reason ?? new DOMException('Notion sign-in stopped', 'AbortError'));
    };
    const timeout = window.setTimeout(finish, CLAIM_INTERVAL_MS);
    signal.addEventListener('abort', stop, { once: true });
    if (signal.aborted) stop();
  });
}

export async function connectNotion(signal: AbortSignal): Promise<NotionTaskSettings> {
  assertActive(signal);
  const attempt = await invokeDesktop<NotionOAuthAttempt>('begin_notion_oauth');
  assertActive(signal);
  const { openUrl } = await import('@tauri-apps/plugin-opener');
  await openUrl(attempt.authorizationUrl);

  const expiresAt = Date.now() + LOGIN_TIMEOUT_MS;
  while (Date.now() < expiresAt) {
    await waitForNextClaim(signal);
    assertActive(signal);
    const claim = await invokeDesktop<NotionOAuthClaim>('claim_notion_oauth', {
      state: attempt.state,
      verifier: attempt.verifier
    });
    if (claim.status === 'connected' && claim.settings) return claim.settings;
  }
  throw new Error('Notion sign-in timed out. Try connecting again.');
}
