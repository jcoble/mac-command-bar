/**
 * Tells a rail row whether it is on screen.
 *
 * Only a visible working row is allowed to spin its indicator or hold the
 * shared clock, so each row needs to know when it scrolls out of sight. One
 * observer watches every row rather than one observer per row.
 */

export type RailRowVisibilityListener = (visible: boolean) => void;

const listeners = new Map<Element, RailRowVisibilityListener>();
let observer: IntersectionObserver | null = null;

function handle(entries: IntersectionObserverEntry[]): void {
  for (const entry of entries) {
    listeners.get(entry.target)?.(entry.isIntersecting);
  }
}

/**
 * Watch one row. The listener is called whenever the row enters or leaves the
 * scroll area. Where the browser has no observer the row counts as visible,
 * which keeps a working row honest rather than silently frozen.
 */
export function observeRailRowVisibility(
  element: Element,
  listener: RailRowVisibilityListener
): () => void {
  if (typeof IntersectionObserver === 'undefined') {
    listener(true);
    return () => {};
  }

  observer ??= new IntersectionObserver(handle);
  listeners.set(element, listener);
  observer.observe(element);

  return () => {
    listeners.delete(element);
    observer?.unobserve(element);
    if (listeners.size === 0) {
      observer?.disconnect();
      observer = null;
    }
  };
}

/** Whether any row is currently being watched. */
export function railRowVisibilityWatching(): boolean {
  return listeners.size > 0;
}
