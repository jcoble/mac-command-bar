/** The owner's answers to what waits on them, shared by Agents > Inbox and the Board's "Messages for you". */
import { sessionRowJump } from '../components/sessionRowJump.ts';
import { adoptSession } from '../conversation/conversationService.ts';
import type { OwnedSession } from '../ownedSessions.ts';
import { answerWorkflowPause, approveWorkflowGate } from './workflowService.ts';
import { workflowMachine } from './workflowTypes.ts';

export type InboxDecision = 'approve' | 'another-round' | 'accept' | 'stop';

/**
 * Answers a waiting step on the machine that owns its run (`mac` for this Mac).
 * The decision itself is not cancelled: stopping it partway on the other machine
 * could leave a gate approved with its next step never started.
 */
export async function answerInboxStep(
  step: { machine: string; runId: string; nodeId: string },
  decision: InboxDecision
): Promise<void> {
  const machine = workflowMachine(step.machine);
  const idempotencyKey = `agents:inbox:${crypto.randomUUID()}`;
  if (decision === 'approve') {
    await approveWorkflowGate({ runId: step.runId, nodeId: step.nodeId, approval: { approved: true }, idempotencyKey }, machine);
  } else {
    await answerWorkflowPause({ runId: step.runId, nodeId: step.nodeId, decision, reply: null, idempotencyKey }, machine);
  }
}

/**
 * Opens a waiting session at its latest message, adopting it first. `current` turns false
 * when a newer click or the view closing made this open stale.
 */
export async function openInboxSession(ownedId: string, signal: AbortSignal, current: () => boolean): Promise<void> {
  const adopted = await adoptSession(ownedId, signal);
  if (!current()) return;
  if (!adopted) throw new Error('That session is not available yet.');
  await sessionRowJump(ownedId, 'session');
}

/** What a waiting session waits for, in words. */
export function waitingOn(session: OwnedSession): string {
  if (session.pendingPermission) return 'Waiting for permission';
  if (session.runtimeState === 'waiting-approval') return 'Waiting for approval';
  return 'Waiting for your answer';
}
