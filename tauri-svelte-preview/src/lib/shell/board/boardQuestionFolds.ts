// Which Build Board questions the owner folded, kept across closing the board and app restarts.

export type Folds = Record<string, boolean>;
export const FOLDS_KEY = 'assembly.board.questionFolds';

/** A question the owner never folded starts expanded while open and collapsed once answered. */
export const isFoldOpen = (folds: Folds, id: string, answered: boolean): boolean => folds[id] ?? !answered;

export function readFolds(storage: Storage): Folds {
  try {
    const value: unknown = JSON.parse(storage.getItem(FOLDS_KEY) ?? '{}');
    return value && typeof value === 'object' && !Array.isArray(value) ? value as Folds : {};
  } catch {
    return {};
  }
}

/** Saves the folds of the questions still on the board and drops the rest. A machine missing from
 *  this read (unreachable for now) keeps its folds, since its questions weren't listed. Ids are `<machine>:<n>`. */
export function writeFolds(storage: Storage, folds: Folds, boardIds: string[], machines: string[]): void {
  const listed = (id: string): boolean => machines.includes(id.slice(0, id.lastIndexOf(':')));
  const kept = Object.fromEntries(Object.entries(folds).filter(([id]) => boardIds.includes(id) || !listed(id)));
  storage.setItem(FOLDS_KEY, JSON.stringify(kept));
}
