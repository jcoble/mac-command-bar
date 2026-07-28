<script lang="ts">
  /**
   * /next — the Slice 1 shell orchestrator. Thin by construction: NO terminal
   * state (`terminalService`), NO rail state (`sessionRailStore`); IO **only in
   * explicit functions**, no `$effect`. Launch IO is tiny by rule: list surviving
   * PTYs, reconcile them against the stored owned sessions, re-attach them (live
   * AND tombstones), scan for resumable agents. No LSP, no git, no source scan.
   * `hydrateOwned` runs AFTER `reconcileOwnedSessions`.
   */
  import { onMount, tick } from 'svelte';

  import SessionRail from '$lib/shell/components/SessionRail.svelte';
  import TerminalSurface from '$lib/shell/components/TerminalSurface.svelte';
  import { countInvoke, invokeCounts } from '$lib/shell/devInvokeCounter.svelte';
  import { adoptAgentSession, reconcileOwnedSessions } from '$lib/shell/ownedSessions';
  import {
    addOwnedSession,
    hydrateOwned,
    loadStoredOwned,
    rail,
    removeOwnedSession,
    setActiveOwned,
    setAvailable,
    updateOwnedSession
  } from '$lib/shell/stores/sessionRailStore.svelte';
  import { createTerminalService, tauriTerminalBackend } from '$lib/shell/terminalService';
  import { loadXtermModules, makeTerminalView } from '$lib/shell/xtermFactory';
  import {
    listAgentSessionsFromLocalBridge,
    listAgentSessionsFromTauri,
    type AgentSession
  } from '$lib/tauriSource';

  /** Hosts mount before the service finishes async init: parked here, drained later. */
  const pendingHosts = new Map<string, HTMLElement>();
  /** Owned ids whose surviving PTY still needs `adoptExisting` once its host mounts. */
  const awaitingReattach = new Set<string>();
  /** ptySessionId -> the PTY's REAL grid (launch `backend.list()`), fed to
   * `adoptExisting`: a HIDDEN host cannot be measured, so without it a survivor's
   * view keeps 80x24 and wraps its replay wrong. */
  const livePtySizes = new Map<string, { cols: number; rows: number }>();

  let service: ReturnType<typeof createTerminalService> | null = null;
  let disposed = false;

  function describeError(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
  }

  /** EXPLICIT IO: scan for resumable agent sessions (Tauri first, bridge second).
   * ONE counted call per rescan, named for the transport that actually ran. */
  async function scanRail(): Promise<void> {
    if (rail.scanning) return;
    rail.scanning = true;
    rail.error = null;
    let nativeError: string | null = null;
    try {
      let sessions: AgentSession[] | null = null;
      try {
        sessions = await listAgentSessionsFromTauri();
      } catch (error) {
        nativeError = describeError(error);
      }
      // A `null` native result with no error = not under Tauri: nothing invoked.
      countInvoke((sessions ?? nativeError) ? 'list_agent_sessions' : 'bridge:agent-sessions');
      sessions ??= await listAgentSessionsFromLocalBridge();
      if (disposed) return;
      setAvailable(sessions ?? []);
      // Surface the native failure only if the bridge produced nothing either.
      if (sessions === null && nativeError) rail.error = `session scan failed: ${nativeError}`;
    } catch (error) {
      if (!disposed) rail.error = `session scan failed: ${nativeError ?? describeError(error)}`;
    } finally {
      rail.scanning = false;
    }
  }

  /** Park a freshly mounted host, and re-attach immediately if one is owed. */
  function registerHost(ownedId: string, host: HTMLElement): void {
    pendingHosts.set(ownedId, host);
    // Fire-and-forget, but never unhandled: this path has no awaiting caller.
    void reattachIfPending(ownedId).catch((error) => {
      if (!disposed) rail.error = `re-attach failed: ${describeError(error)}`;
    });
  }

  /** Wait for TerminalSurface to mount the host for `ownedId`. */
  async function hostFor(ownedId: string): Promise<HTMLElement | null> {
    for (let attempt = 0; attempt < 12; attempt += 1) {
      const host = pendingHosts.get(ownedId);
      if (host) return host;
      await tick();
      if (attempt > 0) await new Promise((resolve) => setTimeout(resolve, 8));
    }
    return pendingHosts.get(ownedId) ?? null;
  }

  /** Re-attach one reload survivor; `awaitingReattach` guards double-adopting. */
  async function reattachIfPending(ownedId: string): Promise<void> {
    if (!service || disposed || !awaitingReattach.has(ownedId)) return;
    awaitingReattach.delete(ownedId);
    const host = pendingHosts.get(ownedId);
    const session = rail.owned.find((entry) => entry.ownedId === ownedId);
    if (!host || !session) return;
    const size = session.ptySessionId ? (livePtySizes.get(session.ptySessionId) ?? null) : null;
    const attached = await service.adoptExisting(session, host, size);
    if (!attached) {
      updateOwnedSession(ownedId, { state: 'exited', ptySessionId: null });
      return;
    }
    if (rail.activeOwnedId === null) await selectOwned(ownedId);
  }

  async function selectOwned(ownedId: string): Promise<void> {
    setActiveOwned(ownedId);
    service?.show(ownedId);
  }

  /** EXPLICIT IO: adopt a scanned session, spawn its PTY, replay the resume command. */
  async function adopt(record: AgentSession): Promise<void> {
    if (!service || disposed) return;
    const owned = adoptAgentSession(record);
    addOwnedSession(owned);
    const host = await hostFor(owned.ownedId);
    if (!host) {
      rail.error = `no terminal host for "${owned.title}"`;
      return;
    }
    const ptySessionId = await service.startOwned(owned, host);
    if (!ptySessionId) {
      updateOwnedSession(owned.ownedId, { state: 'exited' });
      rail.error = `failed to start a terminal for "${owned.title}"`;
      return;
    }
    // Persist the PTY id: reload re-attach reads it back out of localStorage.
    updateOwnedSession(owned.ownedId, { ptySessionId, state: 'live' });
    await selectOwned(owned.ownedId);
  }

  /** EXPLICIT IO: the ONLY path that kills a PTY. `service.closeOwned` never
   * rejects — it reports `{ successor, error }` — so a failed close still hands
   * back the terminal the manager left visible. The row is dropped either way. */
  async function closeOwned(ownedId: string): Promise<void> {
    const session = rail.owned.find((entry) => entry.ownedId === ownedId);
    pendingHosts.delete(ownedId);
    awaitingReattach.delete(ownedId);
    const result = await service?.closeOwned(ownedId, session?.ptySessionId ?? null);
    if (result?.error) {
      rail.error = `close failed for "${session?.title ?? ownedId}": ${describeError(result.error)}`;
    }
    removeOwnedSession(ownedId);
    // Picked BEFORE the await: adopt it only while it still exists.
    const successor = result?.successor ?? null;
    if (successor !== null && rail.owned.some((entry) => entry.ownedId === successor)) {
      setActiveOwned(successor);
    }
  }

  onMount(() => {
    disposed = false;
    void (async () => {
      try {
        const backend = tauriTerminalBackend(countInvoke);
        const modules = await loadXtermModules();
        if (disposed) return;

        service = createTerminalService({
          backend,
          createView: (host, hooks) => makeTerminalView(modules, host, hooks),
          onExit: (ownedId) => updateOwnedSession(ownedId, { state: 'exited' })
        });
        await service.attach();
        if (disposed) return;

        // Rail hydration — the ONLY launch IO (constitution).
        const live = (await backend.list()) ?? [];
        if (disposed) return;
        for (const i of live) livePtySizes.set(i.sessionId, { cols: i.cols, rows: i.rows });
        const { owned, reattachable } = reconcileOwnedSessions(loadStoredOwned(), live);
        // Tombstones re-attach too (final scrollback + a reapable PTY id); live first.
        const attachable = [
          ...reattachable,
          ...owned.filter((entry) => entry.state === 'exited' && entry.ptySessionId)
        ];
        // Claim them BEFORE the hosts mount, or `registerHost` races past.
        for (const session of attachable) awaitingReattach.add(session.ownedId);
        hydrateOwned(owned);
        // One survivor's failure must cost neither the others their re-attach nor
        // the Resume group its scan: collect, keep going, report once.
        const failed: string[] = [];
        for (const session of attachable) {
          await hostFor(session.ownedId);
          try {
            await reattachIfPending(session.ownedId);
          } catch (error) {
            failed.push(`"${session.title}" (${describeError(error)})`);
          }
        }
        // scanRail CLEARS rail.error, so the re-attach report goes after it.
        await scanRail();
        if (failed.length > 0 && !disposed) {
          const prefix = rail.error ? `${rail.error}; ` : '';
          rail.error = `${prefix}could not re-attach ${failed.join(', ')}`;
        }
      } catch (error) {
        if (!disposed) rail.error = `shell start-up failed: ${describeError(error)}`;
      }
    })();

    return () => {
      disposed = true;
      // dispose() drops views + the listener ONLY. Every PTY survives.
      service?.dispose();
      service = null;
      pendingHosts.clear();
      awaitingReattach.clear();
      livePtySizes.clear();
    };
  });
</script>

<svelte:head>
  <title>CommandBar · next</title>
</svelte:head>

<main class="next-shell">
  <aside class="next-rail">
    <SessionRail
      owned={rail.owned}
      available={rail.available}
      activeOwnedId={rail.activeOwnedId}
      scanning={rail.scanning}
      onSelect={selectOwned}
      onAdopt={adopt}
      onClose={closeOwned}
      onRescan={scanRail}
    />
  </aside>

  <section class="next-main">
    <TerminalSurface owned={rail.owned} activeOwnedId={rail.activeOwnedId} {registerHost} />
  </section>

  {#if rail.error}
    <footer class="next-error">{rail.error}</footer>
  {/if}

  {#if import.meta.env.DEV}
    <footer class="invoke-counter">invokes: {invokeCounts.total} (+{invokeCounts.input} input)</footer>
  {/if}
</main>

<style>
  .next-shell {
    position: relative;
    display: grid;
    grid-template-columns: 264px 1fr;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
    background: #101014;
    color: #d8d8e0;
  }

  .next-rail {
    min-width: 0;
    border-right: 1px solid #22222c;
    overflow: hidden;
  }

  .next-main {
    min-width: 0;
    height: 100%;
    overflow: hidden;
  }

  .next-error,
  .invoke-counter {
    position: absolute;
    bottom: 8px;
    border-radius: 5px;
    font-family: ui-monospace, Menlo, monospace;
    font-size: 10px;
    padding: 3px 8px;
    pointer-events: none;
  }

  .next-error {
    left: 50%;
    transform: translateX(-50%);
    background: rgba(255, 85, 85, 0.16);
    color: #ff9d9d;
  }

  .invoke-counter {
    right: 10px;
    background: rgba(16, 16, 20, 0.82);
    color: #6d6d7d;
  }
</style>
