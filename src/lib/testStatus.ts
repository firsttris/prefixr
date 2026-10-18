import * as m from "$lib/paraglide/messages";
import { getLocale } from "$lib/i18n/index.svelte";
import { TEST_STATUSES, type GameTest } from "$lib/stores/libraryMeta";

export { TEST_STATUSES };
import type { TestStatus } from "$lib/types";

// How a test result reads: the symbol on a card, its name, and the line
// that explains it (see `commands::library::TestResult`).

export const TEST_SYMBOL: Record<TestStatus, string> = { works: "✓", issues: "!", broken: "✕" };

export function testLabel(status: TestStatus): string {
  switch (status) {
    case "works":
      return m.test_works();
    case "issues":
      return m.test_issues();
    case "broken":
      return m.test_broken();
  }
}

export function testedOn(secs: number): string {
  const date = new Date(secs * 1000).toLocaleDateString(getLocale(), {
    day: "numeric",
    month: "long",
    year: "numeric",
  });
  return m.test_testedOn({ date });
}

/** "Works with issues · tested on 18 October 2026 · Videos stay black". */
export function testSummary(test: GameTest): string {
  const parts = [testLabel(test.result.status), testedOn(test.result.tested_at)];
  if (test.result.note) parts.push(test.result.note);
  if (!test.current) parts.push(m.test_outdated());
  return parts.join(" · ");
}
