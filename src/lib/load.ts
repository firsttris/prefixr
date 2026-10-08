// Runs a view's initial loads side by side and rejects with the first
// failure once all of them have settled, so a view can show why it's empty
// instead of looking like there's nothing there.
export async function loadAll(...loads: Promise<unknown>[]): Promise<void> {
  const results = await Promise.allSettled(loads);
  const failed = results.find((result) => result.status === "rejected");
  if (failed) throw failed.reason;
}
