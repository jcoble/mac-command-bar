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

/** This Mac's scan, and for the whole list each connected machine's own scan, labelled with that machine. */
async function listProviderSessions(projectPath?: string): Promise<AgentSession[]> {
  const machines = projectPath ? [] : Object.keys(rail.remoteConnections).filter((id) => rail.remoteConnections[id] === 'connected');
  const remote = await Promise.all(machines.map(async (machine) => {
    try {
      return ((await listAgentSessionsFromTauri(machine)) ?? []).map((session) => ({ ...session, machine }));
    } catch (error) {
      console.warn(`[history] could not scan ${machine}`, error);
      return [];
    }
  }));
  return [...(await listLocalProviderSessions(projectPath)), ...remote.flat()];
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
      const sessions = await listProviderSessions();
      if (stopSignal?.aborted || scanGeneration !== generation) return;
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
