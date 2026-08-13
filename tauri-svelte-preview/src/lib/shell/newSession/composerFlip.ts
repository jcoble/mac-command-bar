export type ComposerFlipReceipt = {
  ran: boolean;
  durationMs: number;
  deltaX: number;
  deltaY: number;
  reducedMotion: boolean;
};

export type ComposerFlipOptions = {
  docked: boolean;
  onComplete(receipt: ComposerFlipReceipt): void;
};

const FLIP_DURATION_MS = 180;
const FLIP_EASING = 'cubic-bezier(0.4, 0, 0.2, 1)';

export function composerFlip(node: HTMLElement, options: ComposerFlipOptions) {
  let docked = options.docked;
  let previousRect = node.getBoundingClientRect();
  let animation: Animation | null = null;
  let measureFrame: number | null = null;

  function finish(next: ComposerFlipOptions, receipt: ComposerFlipReceipt): void {
    node.dataset.flipRan = String(receipt.ran);
    node.dataset.flipDuration = String(receipt.durationMs);
    node.dataset.flipDelta = `${receipt.deltaX.toFixed(2)},${receipt.deltaY.toFixed(2)}`;
    node.dataset.flipReducedMotion = String(receipt.reducedMotion);
    next.onComplete(receipt);
  }

  return {
    update(next: ComposerFlipOptions): void {
      const changed = next.docked !== docked;
      const fromRect = previousRect;
      docked = next.docked;
      if (!changed) {
        previousRect = node.getBoundingClientRect();
        return;
      }

      animation?.cancel();
      animation = null;
      if (measureFrame !== null) cancelAnimationFrame(measureFrame);
      measureFrame = requestAnimationFrame(() => {
        measureFrame = null;
        const currentRect = node.getBoundingClientRect();
        previousRect = currentRect;
        const deltaX = fromRect.left - currentRect.left;
        const deltaY = fromRect.top - currentRect.top;
        const reducedMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
        const tooSmall = Math.abs(deltaX) < 0.5 && Math.abs(deltaY) < 0.5;
        if (reducedMotion || tooSmall) {
          finish(next, {
            ran: false,
            durationMs: 0,
            deltaX,
            deltaY,
            reducedMotion
          });
          return;
        }

        animation = node.animate(
          [
            { transform: `translate(${deltaX}px, ${deltaY}px)` },
            { transform: 'translate(0, 0)' }
          ],
          { duration: FLIP_DURATION_MS, easing: FLIP_EASING }
        );
        void animation.finished
          .catch(() => undefined)
          .then(() => finish(next, {
            ran: true,
            durationMs: FLIP_DURATION_MS,
            deltaX,
            deltaY,
            reducedMotion: false
          }));
      });
    },
    destroy(): void {
      if (measureFrame !== null) cancelAnimationFrame(measureFrame);
      animation?.cancel();
    }
  };
}
