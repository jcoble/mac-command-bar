import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { stripTypeScriptTypes } from 'node:module';
import { test } from 'node:test';
const { ChatClient } = await import(new URL('../../ai-client/dist/esm/index.js', import.meta.resolve('@tanstack/ai-svelte')).href);
import { EventType } from '@tanstack/ai/client';

const source = readFileSync(new URL('../src/lib/shell/conversation/conversationConnection.ts', import.meta.url), 'utf8');
const block = source.slice(source.indexOf('function snapshotChunks('), source.indexOf('function admitInitialSnapshot('));

for (const activeTurnId of [undefined, 'old-turn']) {
  await test(`reconnect ${activeTurnId ? 'keeps a running send busy' : 'admits the next send after restart'}`, async () => {
    const chunks: any[] = [];
    let wake: (() => void) | undefined;
    let observed = Promise.withResolvers<void>();
    const selection: any = { workspaceOwnedId: 'session', historyOwnedId: 'session', chat: { messages: [] } };
    const dependencies = {
      EventType, hasOlderHistory: () => false, messagesFromPage: () => [],
      getConversationSession: () => null, historyPosition: () => undefined,
      resetMessageBytes: () => {}, applySelectedConversationSnapshotState: () => {},
      pushMessagesSnapshot: () => {}, restorePageAttachments: () => {}, pageAttachments: () => new Map()
    };
    const snapshotChunks = Function(...Object.keys(dependencies), `${stripTypeScriptTypes(block, { mode: 'strip' })}; return snapshotChunks;`)(...Object.values(dependencies));
    let sends = 0;
    const client = new ChatClient({ id: 'session', queue: 'drop', live: true, connection: {
      async *subscribe(signal: AbortSignal) {
        signal.addEventListener('abort', () => wake?.(), { once: true });
        while (!signal.aborted) {
          if (chunks.length) yield chunks.shift();
          else await new Promise<void>(resolve => { wake = resolve; });
        }
      },
      async send(_messages: unknown, _data: unknown, _signal: unknown, context: { runId: string }) {
        sends++;
        selection.pendingSend = { runId: context.runId, receipt: { turnId: 'old-turn' } };
        observed.resolve();
        if (sends > 1) {
          chunks.push({ type: EventType.RUN_FINISHED, runId: context.runId });
          wake?.();
        }
      }
    }, onChunk: (chunk: any) => { if (chunk.name === 'barrier') observed.resolve(); } });
    try {
      const first = client.sendMessage('Long running turn');
      await observed.promise;
      observed = Promise.withResolvers<void>();
      snapshotChunks(selection, { connection: { generation: 1 }, page: { turns: [], events: [] }, pendingEvents: [], activeTurnId }, (chunk: unknown) => chunks.push(chunk));
      chunks.push({ type: EventType.CUSTOM, name: 'barrier' });
      wake?.();
      await observed.promise;
      assert.equal(client.getIsLoading(), !!activeTurnId);
      if (!activeTurnId) await first;
      await client.sendMessage('RESTART_RECOVERED');
      assert.equal(sends, activeTurnId ? 1 : 2, 'a backend restart must release the client send gate');
    } finally { client.dispose(); }
  });
}
