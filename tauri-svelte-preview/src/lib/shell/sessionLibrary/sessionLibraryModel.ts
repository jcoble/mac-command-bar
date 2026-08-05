/**
 * Pure Session Library projection and identity rules.
 *
 * The rail/runtime stores remain the authorities. This file only projects
 * their current records into a searchable, paged list; it never persists an
 * archive and never starts a provider. Identity intentionally never includes a
 * title: two same-title sessions in different worktrees are different rows.
 */
import type { AgentSession } from '../../tauriSource.ts';
import type { AgentRuntimeState, OwnedSession } from '../ownedSessions.ts';

export type SessionLibraryState = 'working' | 'done' | 'settled' | 'resumable';
export type SessionLibrarySource = 'owned' | 'provider';

export interface SessionLibraryRecord {
  /** Stable row identity. Owned rows use ownedId; provider rows use a compound key. */
  key: string;
  source: SessionLibrarySource;
  ownedId: string | null;
  provider: string;
  nativeSessionId: string | null;
  canonicalCwd: string;
  title: string;
  description: string | null;
  projectPath: string | null;
  model: string | null;
  state: SessionLibraryState;
  runtimeState: AgentRuntimeState | null;
  lastActivity: string | null;
  /** The best existing-authority timestamp for date filtering/display. */
  updatedAt: string | null;
  owned: OwnedSession | null;
  available: AgentSession | null;
}

export interface SessionLibraryFilters {
  query?: string;
  provider?: string;
  project?: string;
  worktree?: string;
  state?: SessionLibraryState | 'all';
  model?: string;
  dateFrom?: string | null;
  dateTo?: string | null;
}

export interface SessionLibraryPage {
  items: SessionLibraryRecord[];
  page: number;
  pageSize: number;
  totalItems: number;
  totalPages: number;
}

export interface SessionLibraryGroup {
  state: SessionLibraryState;
  label: string;
  items: SessionLibraryRecord[];
}

/** Normalize paths for identity without guessing anything about their content. */
export function canonicalCwd(value: string | null | undefined): string {
  const raw = (value ?? '').trim().replaceAll('\\', '/').replace(/\/+/g, '/');
  if (!raw) return '';
  if (raw === '/') return raw;
  const withoutTrailingSlash = raw.replace(/\/$/, '');
  // Preserve drive-letter paths while making their separator spelling stable.
  if (/^[A-Z]:/.test(withoutTrailingSlash)) {
    return `${withoutTrailingSlash[0].toLowerCase()}${withoutTrailingSlash.slice(1)}`;
  }
  return withoutTrailingSlash;
}

export function canonicalProvider(value: string | null | undefined): string {
  return (value ?? '').trim().toLowerCase();
}

/**
 * Provider identity is `(provider,nativeSessionId,canonicalCwd)`. The title is
 * deliberately absent, and `null` native ids remain meaningful alongside the
 * cwd rather than collapsing every fresh session into one row.
 */
export function sessionIdentityKey(input: {
  provider: string | null | undefined;
  nativeSessionId: string | null | undefined;
  cwd: string | null | undefined;
}): string {
  return [
    canonicalProvider(input.provider),
    input.nativeSessionId?.trim() || '',
    canonicalCwd(input.cwd)
  ]
    .map((part) => encodeURIComponent(part))
    .join('|');
}

export function deriveOwnedLibraryState(session: Pick<OwnedSession, 'completedAt' | 'settledAt'>):
  | 'working'
  | 'done'
  | 'settled' {
  // Settled is an explicit user transition. No timestamp age, title, or process
  // state participates in this decision.
  if (typeof session.settledAt === 'string' && session.settledAt.length > 0) return 'settled';
  if (typeof session.completedAt === 'string' && session.completedAt.length > 0) return 'done';
  return 'working';
}

function providerForOwned(session: Pick<OwnedSession, 'agent' | 'viaCmux'>): string {
  return canonicalProvider(session.viaCmux ? `cmux-${session.agent}` : session.agent);
}

function bestUpdatedAt(session: OwnedSession): string | null {
  return session.lastActivity ?? session.settledAt ?? session.completedAt ?? null;
}

export function ownedSessionLibraryRecord(session: OwnedSession): SessionLibraryRecord {
  const provider = providerForOwned(session);
  const cwd = canonicalCwd(session.cwd || session.projectPath);
  return {
    key: `owned:${session.ownedId}`,
    source: 'owned',
    ownedId: session.ownedId,
    provider,
    nativeSessionId: session.nativeSessionId,
    canonicalCwd: cwd,
    title: session.title || session.ownedId,
    description: session.latestTurnPreview,
    projectPath: session.projectPath,
    model: null,
    state: deriveOwnedLibraryState(session),
    runtimeState: session.runtimeState ?? null,
    lastActivity: session.lastActivity,
    updatedAt: bestUpdatedAt(session),
    owned: session,
    available: null
  };
}

export function providerSessionLibraryRecord(session: AgentSession): SessionLibraryRecord {
  const provider = canonicalProvider(session.provider);
  const cwd = canonicalCwd(session.projectPath);
  const identity = sessionIdentityKey({ provider, nativeSessionId: session.id, cwd });
  return {
    key: `provider:${identity}`,
    source: 'provider',
    ownedId: null,
    provider,
    nativeSessionId: session.id || null,
    canonicalCwd: cwd,
    title: session.title || session.id,
    description: session.description ?? session.latestTurnPreview ?? null,
    projectPath: session.projectPath,
    model: session.model ?? null,
    state: 'resumable',
    runtimeState: null,
    lastActivity: session.lastActivity,
    updatedAt: session.lastActivity,
    owned: null,
    available: session
  };
}

/**
 * Merge the current owned rail and provider scanner snapshots. Owned records
 * win when the same identity appears in both, and duplicate ownedIds are also
 * collapsed before compound identity matching.
 */
export function buildSessionLibrary(
  owned: readonly OwnedSession[],
  available: readonly AgentSession[]
): SessionLibraryRecord[] {
  const records: SessionLibraryRecord[] = [];
  const ownedIds = new Set<string>();
  const identities = new Set<string>();

  for (const session of owned) {
    if (!session || ownedIds.has(session.ownedId)) continue;
    ownedIds.add(session.ownedId);
    const record = ownedSessionLibraryRecord(session);
    const identity = sessionIdentityKey({
      provider: record.provider,
      nativeSessionId: record.nativeSessionId,
      cwd: record.canonicalCwd
    });
    if (identities.has(identity)) continue;
    identities.add(identity);
    records.push(record);
  }

  for (const session of available) {
    if (!session || !session.id) continue;
    const record = providerSessionLibraryRecord(session);
    const identity = sessionIdentityKey({
      provider: record.provider,
      nativeSessionId: record.nativeSessionId,
      cwd: record.canonicalCwd
    });
    if (identities.has(identity)) continue;
    identities.add(identity);
    records.push(record);
  }
  return records;
}

function textFor(record: SessionLibraryRecord): string {
  return [
    record.title,
    record.description,
    record.provider,
    record.model,
    record.projectPath,
    record.canonicalCwd,
    record.nativeSessionId,
    record.state
  ]
    .filter(Boolean)
    .join(' ')
    .toLowerCase();
}

function includesFilter(value: string | null | undefined, filter: string | undefined): boolean {
  const needle = filter?.trim().toLowerCase();
  return !needle || (value ?? '').toLowerCase().includes(needle);
}

function dateInRange(value: string | null, from: string | null | undefined, to: string | null | undefined): boolean {
  if (!from && !to) return true;
  const stamp = value ? Date.parse(value) : Number.NaN;
  if (!Number.isFinite(stamp)) return false;
  if (from) {
    const start = Date.parse(from);
    if (Number.isFinite(start) && stamp < start) return false;
  }
  if (to) {
    const end = Date.parse(to);
    if (Number.isFinite(end) && stamp > end) return false;
  }
  return true;
}

export function filterSessionLibrary(
  records: readonly SessionLibraryRecord[],
  filters: SessionLibraryFilters = {}
): SessionLibraryRecord[] {
  const query = filters.query?.trim().toLowerCase();
  return records.filter((record) => {
    if (query && !textFor(record).includes(query)) return false;
    if (!includesFilter(record.provider, filters.provider)) return false;
    if (!includesFilter(record.projectPath, filters.project)) return false;
    if (!includesFilter(record.canonicalCwd, filters.worktree)) return false;
    if (filters.state && filters.state !== 'all' && record.state !== filters.state) return false;
    if (!includesFilter(record.model, filters.model)) return false;
    return dateInRange(record.updatedAt, filters.dateFrom, filters.dateTo);
  });
}

export function paginateSessionLibrary(
  records: readonly SessionLibraryRecord[],
  page = 1,
  pageSize = 50
): SessionLibraryPage {
  const safePageSize = Number.isInteger(pageSize) && pageSize > 0 ? pageSize : 50;
  const totalItems = records.length;
  const totalPages = Math.max(1, Math.ceil(totalItems / safePageSize));
  const safePage = Math.min(Math.max(1, Number.isInteger(page) ? page : 1), totalPages);
  const start = (safePage - 1) * safePageSize;
  return {
    items: records.slice(start, start + safePageSize),
    page: safePage,
    pageSize: safePageSize,
    totalItems,
    totalPages
  };
}

export function groupSessionLibrary(records: readonly SessionLibraryRecord[]): SessionLibraryGroup[] {
  const order: SessionLibraryState[] = ['working', 'resumable', 'done', 'settled'];
  const labels: Record<SessionLibraryState, string> = {
    working: 'Working',
    resumable: 'Resumable',
    done: 'Done',
    settled: 'Settled'
  };
  return order
    .map((state) => ({ state, label: labels[state], items: records.filter((record) => record.state === state) }))
    .filter((group) => group.items.length > 0);
}

// Friendly aliases for callers/tests that prefer “merge” terminology.
export const mergeSessionLibrary = buildSessionLibrary;
export const applySessionLibraryFilters = filterSessionLibrary;
