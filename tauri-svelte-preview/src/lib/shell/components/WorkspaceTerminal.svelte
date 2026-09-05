<script lang="ts">
  /** A single, lazy PTY for the workspace shown in the bottom dock. */
  import '@xterm/xterm/css/xterm.css';
  import { onDestroy, onMount } from 'svelte';

  import {
    closeTerminalSessionFromTauri,
    resizeTerminalSessionFromTauri,
    startTerminalSessionFromTauri,
    writeTerminalSessionFromTauri,
    type SessionSubscription
  } from '$lib/tauriSource';
  import type { TerminalView } from '$lib/liveConversationTerminals';
  import { subscribeToTerminalOutput } from '$lib/shell/terminalService';
  import { loadXtermModules, makeTerminalView } from '$lib/shell/xtermFactory';

  interface Props {
    root: string;
  }
  let { root }: Props = $props();

  let host: HTMLElement;
  let status = $state<'starting' | 'ready' | 'ended' | 'error'>('starting');
  let error = $state('');
  let sessionId: string | null = null;
  let view: TerminalView | null = null;
  let output: SessionSubscription | null = null;
  let sizes: ResizeObserver | null = null;
  let stop = new AbortController();
  let lastSize = '';

  async function writeInput(data: string, signal: AbortSignal): Promise<void> {
    const id = sessionId;
    if (!id || signal.aborted) return;
    await writeTerminalSessionFromTauri(id, data);
  }

  async function resize(cols: number, rows: number, signal: AbortSignal): Promise<void> {
    const id = sessionId;
    const size = `${cols}x${rows}`;
    if (!id || signal.aborted || lastSize === size) return;
    lastSize = size;
    await resizeTerminalSessionFromTauri(id, cols, rows);
  }

  async function start(signal: AbortSignal): Promise<void> {
    if (!root.trim()) {
      status = 'error';
      error = 'Pick a local workspace to open its terminal.';
      return;
    }

    try {
      const modules = await loadXtermModules();
      if (signal.aborted) return;

      let measured = { cols: 80, rows: 24 };
      view = makeTerminalView(modules, host, {
        onData(data): void {
          void writeInput(data, signal);
        },
        onResize(cols, rows): void {
          measured = { cols, rows };
          void resize(cols, rows, signal);
        }
      });
      view.setVisible(true);
      view.fit();

      output = subscribeToTerminalOutput((payload) => {
        if (payload.sessionId !== sessionId || signal.aborted) return;
        if (payload.data) view?.write(payload.data);
        if (payload.terminated) status = 'ended';
      });

      const session = await startTerminalSessionFromTauri({
        cwd: root,
        ownedId: `workspace-dock:${root}`,
        cols: measured.cols,
        rows: measured.rows
      });
      if (signal.aborted) {
        if (session) await closeTerminalSessionFromTauri(session.sessionId);
        return;
      }
      if (!session) {
        status = 'error';
        error = 'The workspace terminal is available in the desktop app.';
        return;
      }

      sessionId = session.sessionId;
      lastSize = `${session.cols}x${session.rows}`;
      status = 'ready';
      view.focus();

      sizes = new ResizeObserver(() => view?.fit());
      sizes.observe(host);
    } catch (cause) {
      if (signal.aborted) return;
      status = 'error';
      error = cause instanceof Error ? cause.message : String(cause);
    }
  }

  export async function close(): Promise<void> {
    stop.abort();
    sizes?.disconnect();
    sizes = null;
    output?.unsubscribe();
    output = null;
    view?.dispose();
    view = null;
    const id = sessionId;
    sessionId = null;
    if (id) await closeTerminalSessionFromTauri(id);
  }

  export function refit(): void {
    view?.fit();
    view?.focus();
  }

  onMount(() => {
    void start(stop.signal);
  });

  onDestroy(() => {
    void close();
  });
</script>

<div class="terminal-surface" data-testid="workspace-terminal">
  <div class="terminal-host" bind:this={host}></div>
  {#if status !== 'ready'}
    <div class="terminal-status" class:error={status === 'error'}>
      {#if status === 'starting'}Opening terminal…{:else if status === 'ended'}Terminal exited{:else}{error}{/if}
    </div>
  {/if}
</div>

<style>
  .terminal-surface {
    position: relative;
    height: 100%;
    min-height: 0;
    overflow: hidden;
    background: var(--color-surface);
  }

  .terminal-host {
    position: absolute;
    inset: 0;
    padding: 6px 8px;
  }

  .terminal-status {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    background: var(--color-surface);
    color: var(--color-text-2);
    font-size: 12px;
  }

  .terminal-status.error {
    color: var(--color-bad);
  }
</style>
