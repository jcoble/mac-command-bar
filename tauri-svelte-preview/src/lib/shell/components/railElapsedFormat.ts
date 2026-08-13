/**
 * How the rail prints a session's age.
 *
 * The age is read once when a row mounts and never ticks. The rail used to
 * carry a shared per-second clock here, and it cost the whole app its
 * snappiness — the hover-to-highlight time grew the longer the rail lived —
 * so the clock is gone and only the reading remains.
 */

/**
 * The rail's reading of an age, coarsening as it grows: seconds under a minute
 * ("38s"), whole minutes under an hour ("11m"), whole hours beyond that ("2h").
 * A row that has been going for days still reads in hours, because that is the
 * number a person compares against the other rows.
 */
export function formatRailElapsed(elapsedMs: number): string {
  const totalSeconds = Math.max(0, Math.floor(elapsedMs / 1_000));
  if (totalSeconds < 60) return `${totalSeconds}s`;

  const totalMinutes = Math.floor(totalSeconds / 60);
  if (totalMinutes < 60) return `${totalMinutes}m`;

  return `${Math.floor(totalMinutes / 60)}h`;
}
