/**
 * Session Library's imperative boundary.
 *
 * Construction is inert. A provider scan or user action only happens when its
 * corresponding method is called, and all effects are injected by the caller.
 * The service has no archive, no localStorage key, and no second rail/runtime
 * authority.
 */
import type { AgentSession } from '../../tauriSource.ts';
import type { OwnedSession } from '../ownedSessions.ts';
import {
  buildSessionLibrary,
  type SessionLibraryRecord
} from './sessionLibraryModel.ts';

export type SessionLibraryAction = 'resume' | 'open' | 'fork' | 'archive' | 'delete';
export type SessionLibraryPlacement = 'center' | 'right';

export interface SessionLibraryActionHandlers {
  onResume?(record: SessionLibraryRecord): void | Promise<void>;
  onOpen?(record: SessionLibraryRecord): void | Promise<void>;
  onFork?(record: SessionLibraryRecord): void | Promise<void>;
  onArchive?(record: SessionLibraryRecord): void | Promise<void>;
  onDelete?(record: SessionLibraryRecord): void | Promise<void>;
}

export interface SessionLibrarySource {
  /** Explicit provider ACP `session/list` adapter; not called at construction. */
  listProviderSessions?(): Promise<AgentSession[]>;
  /** Existing rail authority, read only when refresh is explicitly requested. */
  getOwnedSessions?(): readonly OwnedSession[];
  /** Existing scanner/rail authority, read only when refresh is explicitly requested. */
  getAvailableSessions?(): readonly AgentSession[];
}

export interface SessionLibraryService {
  refresh(): Promise<SessionLibraryRecord[]>;
  resume(record: SessionLibraryRecord): Promise<void>;
  open(record: SessionLibraryRecord): Promise<void>;
  fork(record: SessionLibraryRecord): Promise<void>;
  archive(record: SessionLibraryRecord): Promise<void>;
  delete(record: SessionLibraryRecord): Promise<void>;
}

export interface SessionLibraryMountHandle {
  readonly placement: SessionLibraryPlacement;
  readonly active: boolean;
  release(): void;
}

export interface SessionLibraryMountCoordinator {
  readonly mountedPlacement: SessionLibraryPlacement | null;
  mount(placement: SessionLibraryPlacement): SessionLibraryMountHandle;
  move(placement: SessionLibraryPlacement): SessionLibraryMountHandle;
}

/**
 * Runtime-only host lease for the one canonical workspace. It has no storage
 * and no provider calls: the controller uses `move` when the same workspace
 * changes center/right placement, so a second host cannot be mounted by
 * accident while the first one is still live.
 */
export function createSessionLibraryMountCoordinator(): SessionLibraryMountCoordinator {
  let current: { placement: SessionLibraryPlacement; active: boolean } | null = null;

  const handleFor = (entry: { placement: SessionLibraryPlacement; active: boolean }): SessionLibraryMountHandle => ({
    placement: entry.placement,
    get active(): boolean {
      return entry.active;
    },
    release(): void {
      if (!entry.active) return;
      entry.active = false;
      if (current === entry) current = null;
    }
  });

  const mount = (placement: SessionLibraryPlacement): SessionLibraryMountHandle => {
    if (current) throw new Error(`Session Library already mounted in ${current.placement}`);
    const entry = { placement, active: true };
    current = entry;
    return handleFor(entry);
  };

  return {
    get mountedPlacement(): SessionLibraryPlacement | null {
      return current?.placement ?? null;
    },
    mount,
    move(placement): SessionLibraryMountHandle {
      if (current) handleFor(current).release();
      return mount(placement);
    }
  };
}

async function run(
  handler: ((record: SessionLibraryRecord) => void | Promise<void>) | undefined,
  record: SessionLibraryRecord
): Promise<void> {
  await handler?.(record);
}

/**
 * Create the one action/refresh adapter shared by center and right placements.
 * It deliberately captures callbacks, not session data; both placements can
 * therefore reuse it without duplicating a scan or archive.
 */
export function createSessionLibraryService(
  source: SessionLibrarySource = {},
  handlers: SessionLibraryActionHandlers = {}
): SessionLibraryService {
  return {
    async refresh(): Promise<SessionLibraryRecord[]> {
      const owned = source.getOwnedSessions?.() ?? [];
      const current = source.getAvailableSessions?.() ?? [];
      const provider = source.listProviderSessions ? await source.listProviderSessions() : [];
      return buildSessionLibrary(owned, [...current, ...provider]);
    },
    resume: (record) => run(handlers.onResume, record),
    open: (record) => run(handlers.onOpen, record),
    fork: (record) => run(handlers.onFork, record),
    archive: (record) => run(handlers.onArchive, record),
    delete: (record) => run(handlers.onDelete, record)
  };
}

/** A no-op service is safe for a presentational host until the controller wires actions. */
export const inertSessionLibraryService: SessionLibraryService = createSessionLibraryService();

/**
 * The service and rescan the History panel should use.
 *
 * The panel is mounted by the right column and takes only the three props every
 * panel takes, but the actions it offers — resume, open, archive, delete — are
 * the page's, because only the page owns the rail and the terminal service. So
 * the page registers them once, the same register-then-call shape
 * `sessionLibraryNavigation.ts` and `stackService.ts` already use, and the panel
 * reads whatever is registered.
 */
export interface SessionLibraryHost {
  service: SessionLibraryService;
  /** Look for agent sessions again. */
  rescan?(): void | Promise<void>;
}

let registeredHost: SessionLibraryHost = { service: inertSessionLibraryService };

export function registerSessionLibraryHost(host: SessionLibraryHost): void {
  registeredHost = host;
}

export function sessionLibraryHost(): SessionLibraryHost {
  return registeredHost;
}
