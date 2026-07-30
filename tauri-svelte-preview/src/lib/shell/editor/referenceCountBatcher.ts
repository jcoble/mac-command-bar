/**
 * referenceCountBatcher.ts — turns many "how many references?" questions into
 * one.
 *
 * Monaco asks for the "N references" number above each symbol separately, as
 * each one scrolls into view, and it asks again every time the file changes.
 * Answering each question on its own means reading the whole project once per
 * symbol, which on a large project is what used to lock the app up — so the
 * counts were simply switched off there, and the margin stayed blank.
 *
 * This module holds the two rules that fixed that:
 *
 *  - **One question for all of them.** The first symbol asked about opens a
 *    short window; every symbol asked about while it is open travels in the
 *    same request, and they all get their answer from its result.
 *  - **An answer keeps for a while.** A project-wide count changes when files
 *    elsewhere change, not when the reader types, so answers are kept for a
 *    stretch instead of being thrown away on every keystroke.
 *
 * No backend calls and no Svelte state live here — the caller supplies the way
 * to ask and, in tests, the clock. `sourceIntelligence.ts` wires it up.
 */
import type { SourceReferenceCountResult } from '../../sourceData.ts';

/**
 * One symbol's number for the margin. `atLeast` is true when the pass that
 * produced it did not read every file — it ran out of time, the project had
 * more files than one walk collects, or some files could not be read — so the
 * real number can only be this or higher, and the margin must say so.
 */
export interface CodeLensCount {
  count: number;
  atLeast: boolean;
}

/** A count that has already been worked out, and when. */
export interface RememberedCount {
  value: CodeLensCount;
  countedAt: number;
}

/** Counts kept for a while, then let go. */
export interface CountMemory {
  /** The remembered count, or `undefined` when there is nothing worth using. */
  get(key: string): CodeLensCount | undefined;
  remember(key: string, value: CodeLensCount): void;
  forget(): void;
  readonly size: number;
}

export interface CountMemoryOptions {
  /** How long a remembered count stays good for, in milliseconds. */
  cacheMs: number;
  /** The clock, so tests can move time without waiting. */
  now?: () => number;
}

export function createCountMemory(options: CountMemoryOptions): CountMemory {
  const now = options.now ?? Date.now;
  const remembered = new Map<string, RememberedCount>();

  return {
    get(key: string): CodeLensCount | undefined {
      const entry = remembered.get(key);
      if (!entry) return undefined;
      if (now() - entry.countedAt >= options.cacheMs) {
        remembered.delete(key);
        return undefined;
      }
      return entry.value;
    },
    remember(key: string, value: CodeLensCount): void {
      remembered.set(key, { value, countedAt: now() });
    },
    forget(): void {
      remembered.clear();
    },
    get size(): number {
      return remembered.size;
    }
  };
}

export interface ReferenceCountBatcherOptions {
  /**
   * Count every one of these symbols across the project in one pass. Answering
   * with `null` means the count could not be taken at all; symbols missing from
   * a result that says it is approximate were not reached in time.
   */
  countReferences(symbolNames: string[]): Promise<SourceReferenceCountResult | null>;
  /** How long the window stays open for more symbols, in milliseconds. */
  windowMs: number;
  /** How long an answer stays good for, in milliseconds. */
  cacheMs: number;
  /** Counts at or above this are reported as this, matching the margin's "50+". */
  maxCount: number;
  /** The clock, so tests can move time without waiting. */
  now?: () => number;
}

export interface ReferenceCountBatcher {
  /** How many lines mention this symbol, or `null` when that is not known. */
  count(symbolName: string): Promise<CodeLensCount | null>;
  /** Drop every remembered count (the project changed underneath us). */
  forget(): void;
}

export function createReferenceCountBatcher(
  options: ReferenceCountBatcherOptions
): ReferenceCountBatcher {
  const memory = createCountMemory({ cacheMs: options.cacheMs, now: options.now });
  let waiting = new Map<string, ((count: CodeLensCount | null) => void)[]>();
  let windowTimer: ReturnType<typeof setTimeout> | null = null;

  async function askForWaitingSymbols(): Promise<void> {
    windowTimer = null;
    const batch = waiting;
    waiting = new Map();

    const result = await options.countReferences([...batch.keys()]).catch(() => null);

    for (const [symbolName, callers] of batch) {
      const count = countFromResult(result, symbolName, options.maxCount);
      if (count !== null) memory.remember(symbolName, count);
      for (const caller of callers) caller(count);
    }
  }

  return {
    count(symbolName: string): Promise<CodeLensCount | null> {
      const remembered = memory.get(symbolName);
      if (remembered !== undefined) return Promise.resolve(remembered);

      return new Promise((resolve) => {
        const callers = waiting.get(symbolName);
        if (callers) {
          callers.push(resolve);
          return;
        }

        waiting.set(symbolName, [resolve]);
        windowTimer ??= setTimeout(askForWaitingSymbols, options.windowMs);
      });
    },
    forget(): void {
      memory.forget();
    }
  };
}

/**
 * What one symbol's number should be, given the pass's result.
 *
 * A pass that ran out of time may never have opened the files a symbol lives
 * in, so a zero from it means "not counted", and the margin is better left
 * blank than told there are no references. A non-zero tally from such a pass
 * is real but incomplete — it comes back marked `atLeast` so the margin says
 * "at least N" instead of stating a number nobody actually counted.
 */
export function countFromResult(
  result: SourceReferenceCountResult | null,
  symbolName: string,
  maxCount: number
): CodeLensCount | null {
  const counted = result?.counts?.[symbolName];
  if (typeof counted !== 'number') return null;
  const partial = result?.approximate === true;
  if (partial && counted === 0) return null;
  const shown = Math.min(counted, maxCount);
  return { count: shown, atLeast: partial || shown < counted };
}
