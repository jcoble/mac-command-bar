<script lang="ts">
  /**
   * /next — the Slice 1 shell orchestrator.
   *
   * Thin by construction: it owns NO terminal state (that is `terminalService`)
   * and NO rail state (that is `sessionRailStore`). It wires the two together
   * and performs IO **only inside explicit functions** — never in an `$effect`.
   *
   * Launch IO is deliberately tiny (the constitution's rule): list the surviving
   * PTYs, reconcile them against the stored owned sessions, re-attach the ones
   * that are still alive, and scan for resumable agent sessions. No LSP, no git,
   * no source scan.
   *
   * Two ordering obligations that are easy to get wrong and impossible to see:
   *  - `hydrateOwned` runs AFTER `reconcileOwnedSessions` (hydrate persists
   *    exactly what it is given, so it must be given the reconciled list).
   *  - after `startOwned` resolves, its returned PTY id is written back with
   *    `updateOwnedSession` — reload re-attach reads it out of localStorage.
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

  /**
   * Hosts land before the service is ready (TerminalSurface mounts them from
   * the store, the service finishes async init later), so they are parked here
   * and drained by whichever path needs one.
   */
  const pendingHosts = new Map<string, HTMLElement>();
  /** Owned ids whose surviving PTY still needs `adoptExisting` once its host mounts. */
  const awaitingReattach = new Set<string>();

  let service: ReturnType<typeof createTerminalService> | null = null;
  let disposed = false;

  function describeError(error: unknown): string {
    return error instanceof Error ? error.message : String(error);
  }

  /** EXPLICIT IO: scan for resumable agent sessions (Tauri first, bridge fallback). */
  async function scanRail(): Promise<void> {
    if (rail.scanning) return;
    rail.scanning = true;
    rail.error = null;
    try {
      countInvoke('list_agent_sessions');
      const sessions =
        (await listAgentSessionsFromTauri()) ?? (await listAgentSessionsFromLocalBridge()) ?? [];
      if (!disposed) setAvailable(sessions);
    } catch (error) {
      if (!disposed) rail.error = `session scan failed: ${describeError(error)}`;
    } finally {
      rail.scanning = false;
    }
  }

  /** Park a freshly mounted host, and re-attach immediately if one is owed. */
  function registerHost(ownedId: string, host: HTMLElement): void {
    pendingHosts.set(ownedId, host);
    void reattachIfPending(ownedId);
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

  /**
   * Re-attach one reload survivor. Guarded by `awaitingReattach` so the init
   * drain and `registerHost` can both call it without double-adopting.
   */
  async function reattachIfPending(ownedId: string): Promise<void> {
    if (!service || disposed || !awaitingReattach.has(ownedId)) return;
    awaitingReattach.delete(ownedId);
    const host = pendingHosts.get(ownedId);
    const session = rail.owned.find((entry) => entry.ownedId === ownedId);
    if (!host || !session) return;
    const attached = await service.adoptExisting(session, host);
    if (!attached) {
      updateOwnedSession(ownedId, { state: 'exited', ptySessionId: null });
      return;
    }
    if (rail.activeOwnedId === null) await selectOwned(ownedId);
  }

  /** Focus an owned session and let the manager make its terminal the visible one. */
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

  /** EXPLICIT IO: the ONLY path that kills a PTY. */
  async function closeOwned(ownedId: string): Promise<void> {
    pendingHosts.delete(ownedId);
    awaitingReattach.delete(ownedId);
    await service?.closeOwned(ownedId);
    removeOwnedSession(ownedId);
    const next = rail.owned.find((entry) => entry.state !== 'exited');
    if (rail.activeOwnedId === null && next) await selectOwned(next.ownedId);
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
        const { owned, reattachable } = reconcileOwnedSessions(loadStoredOwned(), live);
        // Claim the survivors BEFORE the hosts can mount, so `registerHost`
        // never races past a re-attach that has not been queued yet.
        for (const session of reattachable) awaitingReattach.add(session.ownedId);
        hydrateOwned(owned);
        for (const session of reattachable) {
          await hostFor(session.ownedId);
          await reattachIfPending(session.ownedId);
        }

        await scanRail();
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
    <footer class="invoke-counter">invokes: {invokeCounts.total}</footer>
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
