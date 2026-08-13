import { invoke } from '@tauri-apps/api/core';

import type { SessionBrowserAnnotation, SessionBrowserRect } from './sessionBrowserOps.ts';

export interface StoredSessionAnnotation {
  id: number;
  ownedId: string;
  url: string;
  rectJson: string;
  note: string;
  createdAtMs: number;
}

export type SessionAnnotationCommandInvoker = <T>(
  command: string,
  args: Record<string, unknown>
) => Promise<T>;

function parseRect(rectJson: string): SessionBrowserRect {
  const parsed: unknown = JSON.parse(rectJson);
  if (
    typeof parsed !== 'object' ||
    parsed === null ||
    !('x' in parsed) ||
    !('y' in parsed) ||
    !('width' in parsed) ||
    !('height' in parsed) ||
    typeof parsed.x !== 'number' ||
    typeof parsed.y !== 'number' ||
    typeof parsed.width !== 'number' ||
    typeof parsed.height !== 'number'
  ) {
    throw new Error('The stored session annotation rectangle is invalid');
  }
  return { x: parsed.x, y: parsed.y, width: parsed.width, height: parsed.height };
}

export function presentStoredSessionAnnotation(
  row: StoredSessionAnnotation,
  marker: number
): SessionBrowserAnnotation {
  return {
    id: row.id,
    marker,
    rect: parseRect(row.rectJson),
    comment: row.note,
    url: row.url,
    createdAt: new Date(row.createdAtMs).toISOString()
  };
}

export async function addStoredSessionAnnotation(
  ownedId: string,
  url: string,
  rect: SessionBrowserRect,
  note: string,
  invokeCommand: SessionAnnotationCommandInvoker = invoke
): Promise<StoredSessionAnnotation> {
  return invokeCommand<StoredSessionAnnotation>('agent_conversation_add_session_annotation', {
    ownedId,
    url,
    rectJson: JSON.stringify(rect),
    note
  });
}

export async function listStoredSessionAnnotations(
  ownedId: string,
  invokeCommand: SessionAnnotationCommandInvoker = invoke
): Promise<StoredSessionAnnotation[]> {
  return invokeCommand<StoredSessionAnnotation[]>('agent_conversation_list_session_annotations', {
    ownedId
  });
}

export async function deleteStoredSessionAnnotation(
  id: number,
  invokeCommand: SessionAnnotationCommandInvoker = invoke
): Promise<void> {
  await invokeCommand<void>('agent_conversation_delete_session_annotation', { id });
}
