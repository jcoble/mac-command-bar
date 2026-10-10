export const USER_SEND_ANCHOR_OFFSET_PX = 12;

export interface ConversationSendAnchorRequest {
  requestId: number;
  conversationId: string;
  userItemId: string;
}

/** A turn counts as running from a local send until the backend's turn ends.
 * A new session's first message has only the backend's turn. */
export function sendTurnRunning(localTurnActive: boolean, activeTurnId: string | null): boolean {
  return localTurnActive || !!activeTurnId;
}
