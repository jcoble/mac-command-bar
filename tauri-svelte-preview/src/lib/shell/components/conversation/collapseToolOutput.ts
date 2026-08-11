/**
 * How much of a tool's output the transcript shows before a person asks for
 * the rest.
 *
 * A build log or a test run can be hundreds of lines. Printed whole, one tool
 * call pushes everything else off the screen, so anything over a dozen lines
 * shows its opening six and offers the remainder behind one control.
 */

/** Output longer than this collapses. Twelve lines still fit comfortably. */
export const COLLAPSE_THRESHOLD_LINES = 12;

/** How many lines stay on screen once output has collapsed. */
export const COLLAPSED_LINE_COUNT = 6;

export interface CollapsedToolOutput {
  /** The lines to render right now. */
  visible: string[];
  /** How many lines are hidden — zero when everything is shown. */
  hiddenCount: number;
}

/** Split raw tool output into lines, tolerating either newline convention. */
export function toolOutputLines(text: string): string[] {
  if (!text.trim()) return [];
  return text.replaceAll('\r\n', '\n').replaceAll('\r', '\n').replace(/\n+$/, '').split('\n');
}

/**
 * Decide what a collapsed tool output shows. Returns a fresh array, so the
 * caller can hold on to the full list unchanged.
 */
export function collapseToolOutput(lines: readonly string[]): CollapsedToolOutput {
  if (lines.length <= COLLAPSE_THRESHOLD_LINES) {
    return { visible: [...lines], hiddenCount: 0 };
  }
  return {
    visible: lines.slice(0, COLLAPSED_LINE_COUNT),
    hiddenCount: lines.length - COLLAPSED_LINE_COUNT
  };
}
