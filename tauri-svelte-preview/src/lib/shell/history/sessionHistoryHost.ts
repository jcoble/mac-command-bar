import {
  listAgentSessionsForProjectFromTauri,
  readAgentSessionDetailsFromTauri,
  readRemoteSavedSessionDetailsFromTauri,
  listAgentSessionsFromLocalBridge,
  listAgentSessionsFromTauri
} from '$lib/tauriSource.ts';
import type { AgentSession } from '$lib/tauriSource.ts';
import { rail, setAvailable } from '$lib/shell/stores/sessionRailStore.svelte.ts';
import {
  createSessionLibraryService,
  registerSessionLibraryHost
} from '$lib/shell/history/library/sessionLibraryService.ts';
import type { SessionLibraryRecord } from '$lib/shell/history/library/sessionLibraryModel.ts';

export interface SessionHistoryHostOptions {
  openOwned(ownedId: string): Promise<void>;
  deleteOwned(ownedId: string): Promise<void>;
}

export interface SessionHistoryHostHandle {
  rescan(stopSignal?: AbortSignal): Promise<void>;
  release(): void;
}

async function listLocalProviderSessions(projectPath?: string): Promise<AgentSession[]> {
  try {
    const nativeSessions = projectPath
      ? await listAgentSessionsForProjectFromTauri(projectPath)
      : await listAgentSessionsFromTauri();
    return nativeSessions ?? (await listAgentSessionsFromLocalBridge()) ?? [];
  } catch {
    return (await listAgentSessionsFromLocalBridge()) ?? [];
  }
}

/** This Mac's scan, and for the whole list each connected machine's own scan, labelled with that machine. This Mac's
 * rows go to `onLocal` as soon as they are read, so a slow machine (its scan may wait 15 s) never holds them back. */
async function listProviderSessions(projectPath?: string, onLocal?: (local: AgentSession[]) => void): Promise<AgentSession[]> {
  const machines = projectPath ? [] : Object.keys(rail.remoteConnections).filter((id) => rail.remoteConnections[id] === 'connected');
  rail.scanningMachines = [...new Set([...rail.scanningMachines, ...machines])];
  rail.runningElsewhere = [];
  const remote = Promise.all(machines.map(async (machine) => {
    try {
      return ((await listAgentSessionsFromTauri(machine)) ?? []).map((session) => ({ ...session, machine }));
    } catch (error) {
      console.warn(`[history] could not scan ${machine}`, error);
      return [];
    } finally {
      rail.scanningMachines = rail.scanningMachines.filter((id) => id !== machine);
    }
  }));
  const local = await listLocalProviderSessions(projectPath);
  onLocal?.(local);
  return [...local, ...(await remote).flat()];
}

/** Own the History panel's scanner and action bridge for the primary shell. */
export function registerSessionHistoryHost(options: SessionHistoryHostOptions): SessionHistoryHostHandle {
  let generation = 0;
  const service = createSessionLibraryService(
    {
      getOwnedSessions: () => rail.owned,
      getAvailableSessions: () => rail.available,
      listProviderSessions,
      readRemoteSavedSessionDetails: readRemoteSavedSessionDetailsFromTauri,
      readProviderSessionDetails: async (logPath, machine = 'local') => machine === 'local'
        ? (await readAgentSessionDetailsFromTauri(logPath)) ?? []
        : ((await readAgentSessionDetailsFromTauri(logPath, machine)) ?? []).map((session) => ({ ...session, machine }))
    },
    {
      onOpen: async (record: SessionLibraryRecord) => {
        if (record.ownedId) await options.openOwned(record.ownedId);
      },
      onDelete: async (record: SessionLibraryRecord) => {
        if (record.ownedId) await options.deleteOwned(record.ownedId);
      }
    }
  );
  const releaseRegistration = registerSessionLibraryHost({ service, rescan });

  async function rescan(stopSignal?: AbortSignal): Promise<void> {
    if (stopSignal?.aborted) return;
    const scanGeneration = ++generation;
    rail.scanning = true;
    try {
      const current = (): boolean => !stopSignal?.aborted && scanGeneration === generation;
      // This Mac's rows show at once, beside the rows other machines gave last time.
      const sessions = await listProviderSessions(undefined, (local) => {
        if (current()) setAvailable([...local, ...rail.available.filter((session) => (session.machine ?? 'local') !== 'local')]);
      });
      if (!current()) return;
      setAvailable(sessions);
    } finally {
      if (!stopSignal?.aborted && scanGeneration === generation) rail.scanning = false;
    }
  }

  return {
    rescan,
    release(): void {
      generation += 1;
      rail.scanning = false;
      service.release();
      releaseRegistration();
    }
  };
}
