/**
 * The only workflow module allowed to cross the Tauri boundary.
 *
 * This file deliberately contains no reducer, JSONL reader, phase inference,
 * or optimistic state.  Every command returns the engine's complete typed
 * snapshot; callers land that snapshot in `workflowStore.svelte.ts`.
 */
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import {
  approveWorkflowGateFromTauri,
  cancelWorkflowRunFromTauri,
  createWorkflowRunFromTauri,
  isNativeTauriRuntime,
  listWorkflowRunsFromTauri,
  pauseWorkflowRunFromTauri,
  redirectWorkflowNodeFromTauri,
  resumeWorkflowRunFromTauri,
  retryWorkflowNodeFromTauri,
  skipWorkflowNodeFromTauri,
  startWorkflowRunFromTauri,
  submitWorkflowResultFromTauri
} from '$lib/tauriSource';
import {
  trackTauriListener,
  trackTauriSubscriber
} from '$lib/shell/resourceDiagnostics.svelte';
import type {
  ApproveWorkflowGateDto,
  CancelWorkflowRunDto,
  CreateWorkflowRunDto,
  PauseWorkflowRunDto,
  RedirectWorkflowNodeDto,
  ResumeWorkflowRunDto,
  RetryWorkflowNodeDto,
  SkipWorkflowNodeDto,
  StartWorkflowRunDto,
  SubmitWorkflowResultDto,
  WorkflowRunRecord
} from './workflowTypes';

export const workflowSnapshotEvent = 'workflow-run-updated';

export async function listWorkflowRuns(): Promise<WorkflowRunRecord[] | null> {
  return listWorkflowRunsFromTauri();
}

export async function createWorkflowRun(
  input: CreateWorkflowRunDto
): Promise<WorkflowRunRecord | null> {
  return createWorkflowRunFromTauri(input.definition, input.input, input.idempotencyKey);
}

export async function startWorkflowRun(
  input: StartWorkflowRunDto
): Promise<WorkflowRunRecord | null> {
  return startWorkflowRunFromTauri(input.runId, input.idempotencyKey);
}

export async function pauseWorkflowRun(
  input: PauseWorkflowRunDto
): Promise<WorkflowRunRecord | null> {
  return pauseWorkflowRunFromTauri(input.runId, input.idempotencyKey);
}

export async function resumeWorkflowRun(
  input: ResumeWorkflowRunDto
): Promise<WorkflowRunRecord | null> {
  return resumeWorkflowRunFromTauri(input.runId, input.idempotencyKey);
}

export async function cancelWorkflowRun(
  input: CancelWorkflowRunDto
): Promise<WorkflowRunRecord | null> {
  return cancelWorkflowRunFromTauri(input.runId, input.idempotencyKey);
}

export async function retryWorkflowNode(
  input: RetryWorkflowNodeDto
): Promise<WorkflowRunRecord | null> {
  return retryWorkflowNodeFromTauri(input.runId, input.nodeId, input.idempotencyKey);
}

export async function redirectWorkflowNode(
  input: RedirectWorkflowNodeDto
): Promise<WorkflowRunRecord | null> {
  return redirectWorkflowNodeFromTauri(
    input.runId,
    input.nodeId,
    input.provider,
    input.idempotencyKey
  );
}

export async function skipWorkflowNode(
  input: SkipWorkflowNodeDto
): Promise<WorkflowRunRecord | null> {
  return skipWorkflowNodeFromTauri(input.runId, input.nodeId, input.idempotencyKey);
}

export async function approveWorkflowGate(
  input: ApproveWorkflowGateDto
): Promise<WorkflowRunRecord | null> {
  return approveWorkflowGateFromTauri(
    input.runId,
    input.nodeId,
    input.approval,
    input.idempotencyKey
  );
}

export async function submitWorkflowResult(
  input: SubmitWorkflowResultDto
): Promise<WorkflowRunRecord | null> {
  return submitWorkflowResultFromTauri(
    input.runId,
    input.nodeId,
    input.result,
    input.idempotencyKey
  );
}

type WorkflowSnapshotPayload = WorkflowRunRecord | WorkflowRunRecord[];
type WorkflowSnapshotListener = (snapshot: WorkflowSnapshotPayload) => void;

let unlisten: UnlistenFn | null = null;
let workflowSnapshotSetup: Promise<void> | null = null;
let workflowSnapshotGeneration = 0;
const listeners = new Set<WorkflowSnapshotListener>();

/**
 * Listen for an optional controller push.  A6 currently returns snapshots from
 * commands and does not require a push event, so browser builds and older
 * controllers simply receive a no-op unsubscribe.  The subscription itself is
 * kept here so components never import Tauri event APIs.
 */
export function subscribeWorkflowSnapshots(
  listener: WorkflowSnapshotListener
): () => void {
  listeners.add(listener);
  ensureWorkflowSnapshotSubscription();
  return trackTauriSubscriber(() => {
    listeners.delete(listener);
    if (listeners.size === 0) stopWorkflowSnapshotListener();
  });
}

function ensureWorkflowSnapshotSubscription(): void {
  if (!isNativeTauriRuntime() || unlisten || workflowSnapshotSetup) return;
  const generation = workflowSnapshotGeneration;
  const setup = setupWorkflowSnapshotSubscription(generation);
  workflowSnapshotSetup = setup;
  void clearWorkflowSnapshotSetup(setup);
}

async function setupWorkflowSnapshotSubscription(generation: number): Promise<void> {
  try {
    const stop = await listen<WorkflowSnapshotPayload>(workflowSnapshotEvent, ({ payload }) => {
      for (const current of listeners) current(payload);
    });
    const trackedStop = trackTauriListener(stop);
    if (generation !== workflowSnapshotGeneration || listeners.size === 0) {
      trackedStop();
      return;
    }
    unlisten = trackedStop;
  } catch {
    // An older controller has no event channel; commands remain authoritative.
    unlisten = null;
  }
}

async function clearWorkflowSnapshotSetup(setup: Promise<void>): Promise<void> {
  await setup;
  if (workflowSnapshotSetup === setup) workflowSnapshotSetup = null;
}

function stopWorkflowSnapshotListener(): void {
  workflowSnapshotGeneration += 1;
  unlisten?.();
  unlisten = null;
  workflowSnapshotSetup = null;
}

export function stopWorkflowSnapshotSubscription(): void {
  stopWorkflowSnapshotListener();
  listeners.clear();
}
