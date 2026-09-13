import { invoke as nativeInvoke, type InvokeArgs, type InvokeOptions } from '@tauri-apps/api/core';
import { qualifyWorkspaceResult, remoteWorkspaceRequest } from './workspacePaths';

/** Preserve the existing bridge; only machine-qualified filesystem requests use the server. */
export async function invoke<T>(command: string, args?: InvokeArgs, options?: InvokeOptions): Promise<T> {
  if (!args || args instanceof ArrayBuffer || ArrayBuffer.isView(args) || Array.isArray(args)) return nativeInvoke<T>(command, args, options);
  const remote = remoteWorkspaceRequest(args);
  if (!remote) return nativeInvoke<T>(command, args, options);
  const result = await nativeInvoke<unknown>('remote_workspace', { profileId: remote.profileId, operation: command, args: remote.args });
  return qualifyWorkspaceResult(result, remote.profileId) as T;
}
