/**
 * sourceCodeLensKeys.ts — the two names behind the "N references" number that
 * Monaco draws above a symbol.
 *
 * There are two of them and they are easy to confuse:
 *
 *  - the **lens id**, which is how Monaco decides whether the lens it is
 *    looking at is the same one it saw a moment ago, and which also carries the
 *    spot back to us when the user clicks the number; and
 *  - the **count key**, which is how the editor remembers a number it has
 *    already been given.
 *
 * Neither may contain the model's version number. It used to, and the effect
 * was that a single keystroke gave every lens on screen a new identity and a
 * new question to ask, all at once. The number counts the symbol's name across
 * the project's saved files, which typing in the open buffer does not change,
 * so where the symbol sits is identity enough and an old number is simply let
 * go after a while.
 *
 * Kept out of `MonacoSourceEditor.svelte` so these rules can be tested without
 * a browser or a Monaco instance.
 */

/** Where a code lens sits, and what it is about. */
export type SourceCodeLensSpot = {
	symbolName: string;
	/** 1-based. */
	line: number;
	/** 1-based. */
	column: number;
};

/** Marks an id as ours; anything else on the lens is somebody else's. */
export const sourceCodeLensIdPrefix = 'mcb-ref-count';

/**
 * How long one counted number is reused before it is asked for again. Same
 * span as the count memory in `shell/editor/sourceIntelligence.ts`, which is
 * what the re-ask lands in.
 */
export const sourceCodeLensCountMemoryMs = 30_000;

/**
 * The lens id. `encodeURIComponent` escapes `:`, so however odd the symbol
 * name is it can never split the id into more pieces than this builds.
 */
export function sourceCodeLensId(spot: SourceCodeLensSpot): string {
	return [
		sourceCodeLensIdPrefix,
		spot.line,
		spot.column,
		encodeURIComponent(spot.symbolName)
	].join(':');
}

/** Read a lens id back, or `null` when it is not one of ours or is damaged. */
export function sourceCodeLensSpotFromId(id: string | undefined): SourceCodeLensSpot | null {
	if (!id?.startsWith(`${sourceCodeLensIdPrefix}:`)) return null;
	const [, line, column, encodedSymbolName] = id.split(':');
	const parsedLine = Number(line);
	const parsedColumn = Number(column);
	const symbolName = encodedSymbolName ? decodeURIComponent(encodedSymbolName) : '';
	if (!symbolName || !Number.isFinite(parsedLine) || !Number.isFinite(parsedColumn)) {
		return null;
	}

	return { symbolName, line: parsedLine, column: parsedColumn };
}

/**
 * Which file, which line, which name. The column is left out on purpose: text
 * typed earlier on the same line moves the symbol sideways without changing
 * the number above it.
 */
export function sourceCodeLensCountKey(modelUri: string, spot: SourceCodeLensSpot): string {
	return `${modelUri}::${spot.line}:${spot.symbolName}`;
}

/**
 * What the margin says before anyone has counted anything.
 *
 * The row appears the moment the file does, so the reader can see there is
 * something to click, and the real number replaces this line as soon as it
 * arrives. There is deliberately no number in this sentence: a placeholder
 * digit would be a number nobody counted, and the reader has no way to tell
 * one of those from a real one.
 */
export const sourceCodeLensPendingTitle = 'counting references…';

/**
 * The finished sentence for a number we were actually given.
 *
 * "at least N" is for a number that came out of the plain-text search over the
 * project — that search stops when it runs out of time or hits its ceiling, so
 * the tally covers only the files it reached and the real number can only be
 * higher. A number from the language server is exact and is stated as it is.
 */
export function formatSourceCodeLensTitle(count: number, atLeast: boolean): string {
	const word = count === 1 ? 'reference' : 'references';
	return atLeast ? `at least ${count} ${word}` : `${count} ${word}`;
}
