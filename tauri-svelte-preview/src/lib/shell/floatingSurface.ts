export type FrameScheduler = (callback: FrameRequestCallback) => number;

/** Let an opened floating surface paint before starting optional background IO. */
export function afterFloatingSurfacePaint(
  run: () => void,
  schedule: FrameScheduler = requestAnimationFrame
): void {
  schedule(() => schedule(() => run()));
}
