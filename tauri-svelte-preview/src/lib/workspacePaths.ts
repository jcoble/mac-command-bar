/** A workspace path carries its machine so late reads cannot switch hosts. */
const remotePrefix = 'assembly-remote://';
export function remoteWorkspacePath(profileId: string, path: string): string {
  if (!profileId || !path.startsWith('/')) throw new Error('Remote workspace needs a saved machine and absolute path');
  return `${remotePrefix}${encodeURIComponent(profileId)}${path}`;
}
export function parseRemoteWorkspacePath(path: string): { profileId: string; path: string } | null {
  if (!path.startsWith(remotePrefix)) return null;
  const slash = path.indexOf('/', remotePrefix.length);
  if (slash < 0) throw new Error('Invalid remote workspace path');
  return { profileId: decodeURIComponent(path.slice(remotePrefix.length, slash)), path: path.slice(slash) };
}

/** Resolve a provider file-edit path against the session folder that owns it. */
export function workspaceChangePath(root: string, path: string): string {
  const trimmed = path.trim();
  if (!trimmed || trimmed.startsWith(remotePrefix)) return trimmed;
  const remote = parseRemoteWorkspacePath(root);
  if (remote) {
    const nativePath = trimmed.startsWith('/')
      ? trimmed
      : `${remote.path.replace(/\/+$/, '')}/${trimmed.replace(/^\/+/, '')}`;
    return remoteWorkspacePath(remote.profileId, nativePath);
  }
  if (trimmed.startsWith('/')) return trimmed;
  return root ? `${root.replace(/\/+$/, '')}/${trimmed.replace(/^\/+/, '')}` : trimmed;
}
export function sessionWorkspaceRoot(session: { cwd: string; projectPath?: string | null; executionEnvironment?: string; remoteProfileId?: string | null }): string {
  const path = session.cwd.trim() || session.projectPath?.trim() || '';
  if (session.executionEnvironment !== 'remote') return path;
  return path && session.remoteProfileId ? remoteWorkspacePath(session.remoteProfileId, path) : '';
}

const pathKeys = new Set(['root', 'path', 'directory', 'source', 'target', 'roots', 'paths', 'projectRoot', 'projectPath', 'worktreePath', 'checkoutPath', 'gitRoot', 'localRoot', 'projectFilter']);
/** Hosted PR commands wrap their root in one request object. */
const nestedRequestCommands = new Set(['list_github_pull_requests', 'read_github_pull_request_file', 'submit_github_pull_request_review', 'reply_github_pull_request_comment', 'merge_github_pull_request']);
export function remoteWorkspaceRequest(args: Record<string, unknown>, command = ''): { profileId: string; args: Record<string, unknown> } | null {
  let profileId: string | null = null;
  let localAbsolute = false;
  const decode = (value: unknown): unknown => {
    if (Array.isArray(value)) return value.map(decode);
    if (typeof value !== 'string') return value;
    const remote = parseRemoteWorkspacePath(value);
    if (!remote) { if (value.startsWith('/')) localAbsolute = true; return value; }
    if (profileId && profileId !== remote.profileId) throw new Error('A workspace operation cannot cross machines');
    profileId = remote.profileId;
    return remote.path;
  };
  const decodeKeys = (fields: Record<string, unknown>): Record<string, unknown> =>
    Object.fromEntries(Object.entries(fields).map(([key, value]) => [key, pathKeys.has(key) ? decode(value) : value]));
  const nested = nestedRequestCommands.has(command);
  const result = Object.fromEntries(Object.entries(decodeKeys(args)).map(([key, value]) => [
    key,
    nested && value && typeof value === 'object' && !Array.isArray(value) ? decodeKeys(value as Record<string, unknown>) : value
  ]));
  if (profileId && localAbsolute) throw new Error('A workspace operation cannot mix Mac and remote paths');
  return profileId ? { profileId, args: result } : null;
}
export function qualifyWorkspaceResult(value: unknown, profileId: string, key = ''): unknown {
  if (typeof value === 'string') return pathKeys.has(key) && value.startsWith('/') ? remoteWorkspacePath(profileId, value) : value;
  if (Array.isArray(value)) return value.map((item) => qualifyWorkspaceResult(item, profileId, key));
  if (value && typeof value === 'object') return Object.fromEntries(Object.entries(value).map(([name, item]) => [name.startsWith('/') ? remoteWorkspacePath(profileId, name) : name, qualifyWorkspaceResult(item, profileId, name)]));
  return value;
}

/** SQLite stores native paths; each client projects them onto its saved machine. */
export function mapWorkspaceSnapshotPaths(value: unknown, profileId: string | null, key = ''): unknown {
  const keys = new Set(['openPaths', 'activePath', 'selectedPath', 'diffRoot', 'filesInspectionRoot', 'sourceControlInspectionRoot', 'root', 'path']);
  const mapPath = (path: string): string => {
    const native = parseRemoteWorkspacePath(path)?.path ?? path;
    return profileId && native.startsWith('/') ? remoteWorkspacePath(profileId, native) : native;
  };
  if (typeof value === 'string') return keys.has(key) || key === 'expandedPathsByRoot' ? mapPath(value) : value;
  if (Array.isArray(value)) return value.map((item) => mapWorkspaceSnapshotPaths(item, profileId, key));
  if (value && typeof value === 'object') return Object.fromEntries(Object.entries(value).map(([name, item]) => [
    name.startsWith('/') || name.startsWith(remotePrefix) ? mapPath(name) : name,
    mapWorkspaceSnapshotPaths(item, profileId, key === 'expandedPathsByRoot' ? key : name)
  ]));
  return value;
}
