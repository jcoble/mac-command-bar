export type ConversationScrollMotion = 'instant' | 'smooth';
export const USER_SEND_ANCHOR_OFFSET_PX = 12;

export interface ConversationSendAnchorRequest {
  requestId: number;
  conversationId: string;
  previousUserItemId: string | null;
}

interface PendingAnchor {
  previousUserItemId: string | null;
  motion: ConversationScrollMotion;
}

export interface ConversationScrollAnchorState {
  pendingAnchor: PendingAnchor | null;
  programmaticMotion: 'idle' | 'smooth';
  pinnedToBottom: boolean;
}

export type ConversationScrollAnchorEvent =
  | { type: 'send'; previousUserItemId: string | null; reducedMotion: boolean }
  | { type: 'user-items-changed'; userItemIds: readonly string[] }
  | { type: 'stream-growth' }
  | { type: 'user-input' }
  | { type: 'jump-to-latest'; reducedMotion: boolean }
  | { type: 'animation-finished' };

export type ConversationScrollAction =
  | { type: 'none' }
  | { type: 'anchor-user'; itemId: string; motion: ConversationScrollMotion; offsetPx: number }
  | { type: 'cancel-programmatic-scroll' }
  | { type: 'scroll-to-latest'; motion: ConversationScrollMotion };

export interface ConversationScrollDecision {
  state: ConversationScrollAnchorState;
  action: ConversationScrollAction;
}

export const initialConversationScrollAnchorState: ConversationScrollAnchorState = {
  pendingAnchor: null,
  programmaticMotion: 'idle',
  pinnedToBottom: false
};

function motionFor(reducedMotion: boolean): ConversationScrollMotion {
  return reducedMotion ? 'instant' : 'smooth';
}

function nextUserItemId(
  userItemIds: readonly string[],
  previousUserItemId: string | null
): string | null {
  if (userItemIds.length === 0) return null;
  if (previousUserItemId === null) return userItemIds[0] ?? null;
  const previousIndex = userItemIds.indexOf(previousUserItemId);
  return previousIndex >= 0 ? userItemIds[previousIndex + 1] ?? null : null;
}

/** Pure decision model shared by the transcript and its script-level contract test. */
export function decideConversationScroll(
  state: ConversationScrollAnchorState,
  event: ConversationScrollAnchorEvent
): ConversationScrollDecision {
  if (event.type === 'send') {
    return {
      state: {
        pendingAnchor: {
          previousUserItemId: event.previousUserItemId,
          motion: motionFor(event.reducedMotion)
        },
        programmaticMotion: 'idle',
        pinnedToBottom: false
      },
      action: { type: 'none' }
    };
  }

  if (event.type === 'user-items-changed' && state.pendingAnchor) {
    const itemId = nextUserItemId(event.userItemIds, state.pendingAnchor.previousUserItemId);
    if (!itemId) return { state, action: { type: 'none' } };
    const motion = state.pendingAnchor.motion;
    return {
      state: {
        pendingAnchor: null,
        programmaticMotion: motion === 'smooth' ? 'smooth' : 'idle',
        pinnedToBottom: false
      },
      action: { type: 'anchor-user', itemId, motion, offsetPx: USER_SEND_ANCHOR_OFFSET_PX }
    };
  }

  if (event.type === 'user-input') {
    return {
      state: { ...state, pendingAnchor: null, programmaticMotion: 'idle', pinnedToBottom: false },
      action: state.programmaticMotion === 'smooth'
        ? { type: 'cancel-programmatic-scroll' }
        : { type: 'none' }
    };
  }

  if (event.type === 'jump-to-latest') {
    const motion = motionFor(event.reducedMotion);
    return {
      state: {
        pendingAnchor: null,
        programmaticMotion: motion === 'smooth' ? 'smooth' : 'idle',
        pinnedToBottom: true
      },
      action: { type: 'scroll-to-latest', motion }
    };
  }

  if (event.type === 'animation-finished') {
    return { state: { ...state, programmaticMotion: 'idle' }, action: { type: 'none' } };
  }

  if (event.type === 'stream-growth' && state.pinnedToBottom) {
    return { state, action: { type: 'scroll-to-latest', motion: 'instant' } };
  }

  // Streaming is inert unless Jump to latest explicitly pinned the bottom.
  return { state, action: { type: 'none' } };
}
