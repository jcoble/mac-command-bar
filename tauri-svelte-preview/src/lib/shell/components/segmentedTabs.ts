/**
 * segmentedTabs.ts — where the selected pill sits in a segmented tab strip.
 *
 * The segments in the strip touch each other, so the arithmetic is the whole
 * of it: the pill's left edge is everything measured before the selected
 * segment, and its width is that segment's own. Keeping it here means the
 * moving part can be tested without a browser.
 */

import type { LucideIcon } from '@lucide/svelte';

/** One tab. A tab with an icon shows the icon and says its label on hover. */
export interface SegmentedTabItem {
  id: string;
  label: string;
  icon?: LucideIcon;
  testId?: string;
}

/** The pill's place in the track, in pixels from the track's left edge. */
export interface IndicatorFrame {
  left: number;
  width: number;
}

/** No pill: nothing is selected, or the strip has not been measured yet. */
const HIDDEN: IndicatorFrame = { left: 0, width: 0 };

export function indicatorFrame(widths: readonly number[], index: number): IndicatorFrame {
  if (index < 0 || index >= widths.length) return HIDDEN;
  let left = 0;
  for (let before = 0; before < index; before += 1) left += widths[before];
  return { left, width: widths[index] };
}
