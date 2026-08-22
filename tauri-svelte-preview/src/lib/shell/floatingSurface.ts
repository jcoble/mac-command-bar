import { requestTrackedAnimationFrame } from './resourceDiagnostics.svelte.ts';

export type FrameScheduler = (callback: FrameRequestCallback) => number;

/** Let an opened floating surface paint before starting optional background IO. */
export function afterFloatingSurfacePaint(
  run: () => void,
  schedule: FrameScheduler = requestTrackedAnimationFrame
): void {
  schedule(() => schedule(() => run()));
}
