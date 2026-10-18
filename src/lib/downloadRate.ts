// How fast an install goes, from the progress events it sends: percent
// over time, turned into bytes per second once the download's size is
// known. Kept free of Svelte so it can be tested.

export interface Sample {
  /** Milliseconds, as from `Date.now()`. */
  at: number;
  percent: number;
}

/** How long a window of samples the rate is taken over. */
const WINDOW_MS = 20_000;

/** Adds a sample, dropping the ones older than the window. */
export function addSample(samples: Sample[], sample: Sample): Sample[] {
  const kept = samples.filter((s) => sample.at - s.at <= WINDOW_MS && s.percent <= sample.percent);
  return [...kept, sample];
}

export interface Estimate {
  bytesPerSecond: number;
  secondsLeft: number;
}

/**
 * The speed and the time left, from the oldest and newest samples in the
 * window; `null` while there's too little to tell.
 */
export function estimate(
  samples: Sample[],
  totalBytes: number | null | undefined,
): Estimate | null {
  if (!totalBytes || samples.length < 2) return null;
  const first = samples[0];
  const last = samples[samples.length - 1];
  const seconds = (last.at - first.at) / 1000;
  const percent = last.percent - first.percent;
  if (seconds < 2 || percent <= 0) return null;
  const bytesPerSecond = ((percent / 100) * totalBytes) / seconds;
  const secondsLeft = (((100 - last.percent) / 100) * totalBytes) / bytesPerSecond;
  return { bytesPerSecond, secondsLeft };
}

/** "3 min", "1 h 5 min", "< 1 min". */
export function formatDuration(secs: number): string {
  if (secs < 60) return "< 1 min";
  const hours = Math.floor(secs / 3600);
  const minutes = Math.round((secs % 3600) / 60);
  return hours === 0 ? `${minutes} min` : `${hours} h ${minutes} min`;
}
