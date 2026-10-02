/**
 * The three marks that stand for "the agent is working".
 *
 * One is chosen per turn from the turn's own id, so a person watching the app
 * sees a different small figure each time rather than the same blinking dot for
 * hours. The choice is a pure function of the seed: the same turn keeps the
 * same spinner across a re-render, a scroll, or a reload, and nothing here
 * reaches for a random number that would change under the reader's eyes.
 *
 * `parts` is how many elements the drawing needs; the component renders that
 * many empty spans and the stylesheet shapes and moves them by name.
 */

export type WorkingSpinnerKind = 'flat' | '3d';

export interface WorkingSpinner {
  /** Name used both as the stylesheet's hook and as the value of `data-spinner`. */
  id: string;
  kind: WorkingSpinnerKind;
  parts: number;
}

export const SPINNERS: WorkingSpinner[] = [
  { id: 'cube', kind: '3d', parts: 6 },
  { id: 'facet', kind: '3d', parts: 4 },
  { id: 'ribbon', kind: '3d', parts: 8 }
];

/**
 * Which spinner a turn gets. The seed is any string that is stable for the
 * length of the turn — a turn id, or the session id when a row has no turn to
 * name yet.
 */
export function pickSpinner(seed: string): string {
  let hash = 0;
  for (let index = 0; index < seed.length; index += 1) {
    hash = (hash * 31 + seed.charCodeAt(index)) % 2147483647;
  }
  return SPINNERS[hash % SPINNERS.length].id;
}
