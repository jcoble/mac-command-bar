/**
 * resourceSparkline.ts — turn a list of readings into one SVG path.
 *
 * A column of numbers changing every three seconds tells you almost nothing: a
 * process that has been climbing for two minutes and one that just spiked read
 * exactly the same. The shape is the information, so every row that has a
 * history draws one, and the drawing is a pure function of the numbers so it
 * can be read in a test instead of squinted at in a screenshot.
 *
 * The line is scaled to its own series, not to a shared maximum. Each row is
 * answering "is this rising", not "is this bigger than that row", and a shared
 * scale would flatten every quiet row into a straight line at the bottom.
 */

export type SparklineOptions = {
  width: number;
  height: number;
  /** Room left at the top and bottom so a full-height stroke is not clipped. */
  inset?: number;
};

export type SparklineExtent = {
  min: number;
  max: number;
};

/** The lowest and highest reading in a series, ignoring anything unusable. */
export function sparklineExtent(values: readonly number[]): SparklineExtent {
  const usable = values.filter((value) => Number.isFinite(value));
  if (usable.length === 0) return { min: 0, max: 0 };
  return {
    min: Math.min(...usable),
    max: Math.max(...usable)
  };
}

function round(value: number): number {
  return Math.round(value * 100) / 100;
}

/**
 * The `d` attribute for a series, oldest reading on the left.
 *
 * An empty series draws nothing. A series whose readings are all the same —
 * including a series of one — draws a flat line through the middle, which is
 * the honest picture of "nothing has changed".
 */
export function sparklinePath(values: readonly number[], options: SparklineOptions): string {
  const usable = values.filter((value) => Number.isFinite(value));
  if (usable.length === 0) return '';

  const inset = options.inset ?? 1;
  const top = inset;
  const bottom = Math.max(inset, options.height - inset);
  const usableHeight = Math.max(0, bottom - top);
  const { min, max } = sparklineExtent(usable);
  const span = max - min;

  const points = usable.map((value, index) => {
    const x = usable.length === 1 ? options.width : (index / (usable.length - 1)) * options.width;
    const ratio = span === 0 ? 0.5 : (value - min) / span;
    return [round(x), round(bottom - ratio * usableHeight)] as const;
  });

  if (points.length === 1) {
    const [, y] = points[0];
    return `M0,${y} L${round(options.width)},${y}`;
  }
  return points
    .map(([x, y], index) => `${index === 0 ? 'M' : 'L'}${x},${y}`)
    .join(' ');
}

/** A short sentence naming what the line covers, for the label a reader hears. */
export function describeSparkline(
  values: readonly number[],
  unit: 'cpu' | 'memory',
  sampleSeconds = 3
): string {
  const usable = values.filter((value) => Number.isFinite(value));
  if (usable.length === 0) return 'No history yet';
  const seconds = usable.length * sampleSeconds;
  const window = seconds < 60 ? `${seconds}s` : `${Math.round(seconds / 60)}m`;
  return `${unit === 'cpu' ? 'CPU' : 'Memory'} over the last ${window}`;
}
