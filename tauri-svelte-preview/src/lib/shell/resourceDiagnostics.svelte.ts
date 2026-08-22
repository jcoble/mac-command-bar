/**
 * Current development-only resource counts.
 *
 * This intentionally keeps no samples or history. It only mirrors the resources
 * whose lifecycle is explicit enough to count from create/dispose points.
 */
export const resourceDiagnostics = $state({
  loadedConversationProjections: 0,
  loadedConversationEventBytes: 0,
  tauriRootListeners: 0,
  xtermViews: 0,
  codeMirrorEditorViews: 0,
  codeMirrorDocBytes: 0,
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
  eventBytes: number
): void {
  if (!import.meta.env.DEV) return;
  resourceDiagnostics.loadedConversationProjections = Math.max(0, projections);
  resourceDiagnostics.loadedConversationEventBytes = Math.max(0, Math.trunc(eventBytes));
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

export function setCodeMirrorDocBytes(bytes: number): void {
  if (!import.meta.env.DEV) return;
  resourceDiagnostics.codeMirrorDocBytes = Math.max(0, Math.trunc(bytes));
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
