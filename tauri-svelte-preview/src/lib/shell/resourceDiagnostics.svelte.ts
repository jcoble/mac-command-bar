/**
 * Current development-only resource counts.
 *
 * This intentionally keeps no samples or history. It only mirrors the resources
 * whose lifecycle is explicit enough to count from create/dispose points.
 */
export const resourceDiagnostics = $state({
  loadedConversationProjections: 0,
  loadedConversationEventBytes: 0,
  loadedChildTranscriptBytes: 0,
  conversationRenderedRows: 0,
  conversationVirtualRows: 0,
  conversationMeasuredElementCacheEntries: 0,
  sentAttachmentMapEntries: 0,
  sentAttachmentCount: 0,
  tauriRootListeners: 0,
  tauriEventSubscribers: 0,
  tauriChannels: 0,
  fileWatchers: 0,
  objectUrls: 0,
  attachmentObjectUrls: 0,
  animationFrames: 0,
  loadedTreeNodes: 0,
  loadedGitHistoryRows: 0,
  activeDiffs: 0,
  xtermViews: 0,
  codeMirrorEditorViews: 0,
  codeMirrorEditorStates: 0,
  codeMirrorDocBytes: 0,
  codeMirrorUndoDepth: 0,
  openTabDocumentBytes: 0,
  mergeViews: 0,
  mergeDocBytes: 0,
  elementVisibilityWatchers: 0,
  elementVisibilityObserverActive: false,
  railElapsedWatchers: 0,
  railElapsedIntervalMs: null as number | null,
  semanticWaitingSpots: 0,
  semanticSchedulerWaiting: 0,
  semanticSchedulerInFlight: 0,
  semanticReferenceRequests: 0,
  semanticRetryTimers: 0,
  semanticHeldUntilReady: 0,
  semanticRememberedTargets: 0,
  sourcePreviewCacheEntries: 0,
  rememberedReferenceCountEntries: 0
});

export function setConversationProjectionDiagnostics(
  projections: number,
  eventBytes: number,
  childTranscriptBytes: number
): void {
  if (!import.meta.env.DEV) return;
  resourceDiagnostics.loadedConversationProjections = Math.max(0, projections);
  resourceDiagnostics.loadedConversationEventBytes = Math.max(0, Math.trunc(eventBytes));
  resourceDiagnostics.loadedChildTranscriptBytes = Math.max(
    0,
    Math.trunc(childTranscriptBytes)
  );
}

export function setConversationTimelineDiagnostics(
  renderedRows: number,
  virtualRows: number,
  measuredRowCacheEntries: number
): void {
  if (!import.meta.env.DEV) return;
  resourceDiagnostics.conversationRenderedRows = Math.max(0, Math.trunc(renderedRows));
  resourceDiagnostics.conversationVirtualRows = Math.max(0, Math.trunc(virtualRows));
  resourceDiagnostics.conversationMeasuredElementCacheEntries = Math.max(
    0,
    Math.trunc(measuredRowCacheEntries)
  );
}

export function setSentAttachmentDiagnostics(
  mapEntries: number,
  attachmentCount: number
): void {
  if (!import.meta.env.DEV) return;
  resourceDiagnostics.sentAttachmentMapEntries = Math.max(0, Math.trunc(mapEntries));
  resourceDiagnostics.sentAttachmentCount = Math.max(0, Math.trunc(attachmentCount));
}

export function setLoadedTreeNodes(nodes: number): void {
  if (!import.meta.env.DEV) return;
  resourceDiagnostics.loadedTreeNodes = Math.max(0, nodes);
}

export function setGitSurfaceDiagnostics(historyRows: number, activeDiffs: number): void {
  if (!import.meta.env.DEV) return;
  resourceDiagnostics.loadedGitHistoryRows = Math.max(0, historyRows);
  resourceDiagnostics.activeDiffs = Math.max(0, activeDiffs);
}

export function trackTauriListener(unlisten: () => void): () => void {
  if (!import.meta.env.DEV) return unlisten;
  resourceDiagnostics.tauriRootListeners += 1;
  let listening = true;
  return () => {
    if (!listening) return;
    listening = false;
    try {
      unlisten();
    } finally {
      resourceDiagnostics.tauriRootListeners = Math.max(
        0,
        resourceDiagnostics.tauriRootListeners - 1
      );
    }
  };
}

export function trackTauriSubscriber(unsubscribe: () => void): () => void {
  if (import.meta.env.DEV) resourceDiagnostics.tauriEventSubscribers += 1;
  let subscribed = true;
  return () => {
    if (!subscribed) return;
    subscribed = false;
    try {
      unsubscribe();
    } finally {
      if (import.meta.env.DEV) {
        resourceDiagnostics.tauriEventSubscribers = Math.max(
          0,
          resourceDiagnostics.tauriEventSubscribers - 1
        );
      }
    }
  };
}

export function trackTauriChannel(): () => void {
  if (import.meta.env.DEV) resourceDiagnostics.tauriChannels += 1;
  let active = true;
  return () => {
    if (!active) return;
    active = false;
    if (import.meta.env.DEV) {
      resourceDiagnostics.tauriChannels = Math.max(0, resourceDiagnostics.tauriChannels - 1);
    }
  };
}

export function trackFileWatcher(unwatch: () => void): () => void {
  if (!import.meta.env.DEV) return unwatch;
  resourceDiagnostics.fileWatchers += 1;
  let watching = true;
  return () => {
    if (!watching) return;
    watching = false;
    try {
      unwatch();
    } finally {
      resourceDiagnostics.fileWatchers = Math.max(0, resourceDiagnostics.fileWatchers - 1);
    }
  };
}

type ObjectUrlOwner = 'attachment' | 'other';

const trackedObjectUrls = new Map<string, ObjectUrlOwner>();

function publishObjectUrlDiagnostics(): void {
  resourceDiagnostics.objectUrls = trackedObjectUrls.size;
  resourceDiagnostics.attachmentObjectUrls = [...trackedObjectUrls.values()]
    .filter((owner) => owner === 'attachment').length;
}

export function createTrackedObjectUrl(
  value: Blob | MediaSource,
  owner: ObjectUrlOwner = 'other'
): string {
  const url = URL.createObjectURL(value);
  if (import.meta.env.DEV) {
    trackedObjectUrls.set(url, owner);
    publishObjectUrlDiagnostics();
  }
  return url;
}

export function revokeTrackedObjectUrl(url: string): void {
  URL.revokeObjectURL(url);
  if (!import.meta.env.DEV) return;
  trackedObjectUrls.delete(url);
  publishObjectUrlDiagnostics();
}

const trackedAnimationFrames = new Set<number>();

export function requestTrackedAnimationFrame(callback: FrameRequestCallback): number {
  const frame = requestAnimationFrame((now) => {
    if (import.meta.env.DEV) {
      trackedAnimationFrames.delete(frame);
      resourceDiagnostics.animationFrames = trackedAnimationFrames.size;
    }
    callback(now);
  });
  if (import.meta.env.DEV) {
    trackedAnimationFrames.add(frame);
    resourceDiagnostics.animationFrames = trackedAnimationFrames.size;
  }
  return frame;
}

export function cancelTrackedAnimationFrame(frame: number): void {
  cancelAnimationFrame(frame);
  if (!import.meta.env.DEV) return;
  trackedAnimationFrames.delete(frame);
  resourceDiagnostics.animationFrames = trackedAnimationFrames.size;
}

export function textBytes(text: string): number {
  if (!import.meta.env.DEV) return 0;
  if (typeof TextEncoder === 'undefined') return text.length;
  return new TextEncoder().encode(text).byteLength;
}

export function addXtermView(delta: 1 | -1): void {
  if (!import.meta.env.DEV) return;
  resourceDiagnostics.xtermViews = Math.max(0, resourceDiagnostics.xtermViews + delta);
}

export function addCodeMirrorEditorView(delta: 1 | -1): void {
  if (!import.meta.env.DEV) return;
  resourceDiagnostics.codeMirrorEditorViews = Math.max(
    0,
    resourceDiagnostics.codeMirrorEditorViews + delta
  );
}

export function setCodeMirrorEditorStateCount(count: number): void {
  if (!import.meta.env.DEV) return;
  resourceDiagnostics.codeMirrorEditorStates = Math.max(0, Math.trunc(count));
}

export function setCodeMirrorDocBytes(bytes: number): void {
  if (!import.meta.env.DEV) return;
  resourceDiagnostics.codeMirrorDocBytes = Math.max(0, Math.trunc(bytes));
}

export function setCodeMirrorUndoDepth(depth: number): void {
  if (!import.meta.env.DEV) return;
  resourceDiagnostics.codeMirrorUndoDepth = Math.max(0, Math.trunc(depth));
}

export function setOpenTabDocumentBytes(bytes: number): void {
  if (!import.meta.env.DEV) return;
  resourceDiagnostics.openTabDocumentBytes = Math.max(0, Math.trunc(bytes));
}

export function addMergeView(delta: 1 | -1): void {
  if (!import.meta.env.DEV) return;
  resourceDiagnostics.mergeViews = Math.max(0, resourceDiagnostics.mergeViews + delta);
}

export function setMergeDocBytes(bytes: number): void {
  if (!import.meta.env.DEV) return;
  resourceDiagnostics.mergeDocBytes = Math.max(0, Math.trunc(bytes));
}

export function setElementVisibilityDiagnostics(watchers: number, observerActive: boolean): void {
  if (!import.meta.env.DEV) return;
  resourceDiagnostics.elementVisibilityWatchers = Math.max(0, watchers);
  resourceDiagnostics.elementVisibilityObserverActive = observerActive;
}

export function setRailElapsedDiagnostics(watchers: number, intervalMs: number | null): void {
  if (!import.meta.env.DEV) return;
  resourceDiagnostics.railElapsedWatchers = Math.max(0, watchers);
  resourceDiagnostics.railElapsedIntervalMs = intervalMs;
}

export function setSourceIntelligenceDiagnostics(values: {
  semanticWaitingSpots: number;
  semanticSchedulerWaiting: number;
  semanticSchedulerInFlight: number;
  semanticReferenceRequests: number;
  semanticRetryTimers: number;
  semanticHeldUntilReady: number;
  semanticRememberedTargets: number;
  sourcePreviewCacheEntries: number;
  rememberedReferenceCountEntries: number;
}): void {
  if (!import.meta.env.DEV) return;
  resourceDiagnostics.semanticWaitingSpots = Math.max(0, values.semanticWaitingSpots);
  resourceDiagnostics.semanticSchedulerWaiting = Math.max(0, values.semanticSchedulerWaiting);
  resourceDiagnostics.semanticSchedulerInFlight = Math.max(0, values.semanticSchedulerInFlight);
  resourceDiagnostics.semanticReferenceRequests = Math.max(0, values.semanticReferenceRequests);
  resourceDiagnostics.semanticRetryTimers = Math.max(0, values.semanticRetryTimers);
  resourceDiagnostics.semanticHeldUntilReady = Math.max(0, values.semanticHeldUntilReady);
  resourceDiagnostics.semanticRememberedTargets = Math.max(0, values.semanticRememberedTargets);
  resourceDiagnostics.sourcePreviewCacheEntries = Math.max(0, values.sourcePreviewCacheEntries);
  resourceDiagnostics.rememberedReferenceCountEntries = Math.max(
    0,
    values.rememberedReferenceCountEntries
  );
}
