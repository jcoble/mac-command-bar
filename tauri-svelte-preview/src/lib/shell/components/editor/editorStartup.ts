/**
 * editorStartup.ts — the two things that decide whether the code editor's
 * placeholder bars ever go away.
 *
 * The bars are shown until the editor says it is ready. Nothing else in the
 * panel can end them, so every way of failing to become ready has to end in a
 * sentence the reader can act on instead. That is what these two pieces are:
 *
 *  1. `waitForConnectedHost` — the editor is built into an element that the
 *     shell moves into its dock host AFTER the panels inside it have mounted,
 *     so that element can be off the document for a stretch of frames when a
 *     file is already open at launch. Waiting a single frame and giving up
 *     silently is what left the bars on screen with nothing behind them.
 *  2. `editorStartFailureMessage` — the plain sentence for the two ways
 *     starting can end badly: something threw, or nothing happened in time.
 *
 * PURE: no DOM, no Svelte, no timers of its own. The caller supplies the
 * frame wait and the questions about its own element.
 */

/** How long the editor gets to appear before the reader is told it did not. */
export const EDITOR_START_LIMIT_MS = 15_000;

export interface ConnectedHostWait {
  /** Is the element the editor will be built into back in the document? */
  isConnected(): boolean;
  /**
   * Is this editor still wanted? False once the component is torn down, its
   * element is replaced, or the wait has been given up on.
   */
  isWanted(): boolean;
  /** Resolves on the next animation frame. */
  nextFrame(): Promise<void>;
}

/**
 * Wait for the host element to be in the document. Answers true when it is,
 * false when the editor stopped being wanted first — and, in that case only,
 * the caller is right to return without a word, because something else has
 * already taken over.
 */
export async function waitForConnectedHost(wait: ConnectedHostWait): Promise<boolean> {
  while (!wait.isConnected()) {
    if (!wait.isWanted()) return false;
    await wait.nextFrame();
  }
  return wait.isWanted();
}

/**
 * What to show instead of the bars. `relativePath` is the file the reader
 * clicked, so the sentence names the thing that did not open; leave `reason`
 * out for the case where starting simply never finished.
 */
export function editorStartFailureMessage(
  relativePath: string | null | undefined,
  reason?: unknown
): string {
  const file = relativePath?.trim();
  const opening = file
    ? `The code editor could not be started, so ${file} cannot be shown.`
    : 'The code editor could not be started.';
  if (reason === undefined) {
    return `${opening} It did not finish loading in time.`;
  }
  const detail = reason instanceof Error ? reason.message : String(reason);
  return detail.trim() ? `${opening} ${detail}` : opening;
}
