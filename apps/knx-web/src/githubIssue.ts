/** Builds a prefilled GitHub "new issue" URL, shortening the body to what a browser will send. */

// T29. This module is the whole of the "open a GitHub issue" path, and it
// deliberately does very little: it builds a URL. No token, no credential,
// no `POST`, no API client. The application never authenticates to GitHub
// and cannot file an issue on the user's behalf — it opens a page with the
// fields filled in, and the user reads it and decides whether to press the
// button. Nothing leaves the machine before that.

export const ISSUE_BASE_URL = "https://github.com/KNXBench-Labs/KNXBench/issues/new";

/**
 * How long the finished URL may get. GitHub itself accepts long query
 * strings, but the browsers and desktop shells in between do not all agree
 * on how long: the practical floor across them is around 8 KB, so that is
 * the budget. A URL that gets silently dropped by a shell is a worse
 * outcome than a body that says it was shortened.
 */
export const MAX_ISSUE_URL_LENGTH = 8000;

/**
 * What a truncated body ends with. It names the remedy rather than just
 * the fact — the full text is in the zip, and attaching it is one drag
 * away, so the user is never left with a half-report and no way to
 * complete it.
 */
export const TRUNCATION_NOTICE =
  "\n\n---\n\n*(This report was shortened to fit in a URL. The complete version is in the " +
  "debug-report zip — please attach it to this issue.)*";

export interface IssueUrl {
  url: string;
  /** True when the body was shortened. The dialog says so out loud; it is
   * never left for the user to notice in the GitHub tab. */
  truncated: boolean;
}

function compose(title: string, body: string): string {
  const query = new URLSearchParams({ title, body });
  return `${ISSUE_BASE_URL}?${query.toString()}`;
}

/**
 * The prefilled issue URL for `title` and `body`, shortened if the result
 * would exceed `maxLength`.
 *
 * Shortening is a binary search over the body's length rather than a
 * proportional guess, because percent-encoding makes the relationship
 * between body length and URL length wildly non-linear — one emoji or one
 * German umlaut costs several URL characters, a run of ASCII letters costs
 * one each. The search works on code points (`Array.from`), not UTF-16 code
 * units, so a cut never lands inside a surrogate pair and produces a lone
 * half of an astral character.
 */
export function buildIssueUrl(
  title: string,
  body: string,
  maxLength: number = MAX_ISSUE_URL_LENGTH,
): IssueUrl {
  const whole = compose(title, body);
  if (whole.length <= maxLength) return { url: whole, truncated: false };

  const points = Array.from(body);
  // Longest prefix length whose URL still fits. `low` is always known to
  // fit (the empty body plus the notice may not, but that is the floor this
  // can offer and the caller gets it rather than nothing).
  let low = 0;
  let high = points.length;
  while (low < high) {
    const mid = Math.ceil((low + high) / 2);
    const candidate = compose(title, points.slice(0, mid).join("") + TRUNCATION_NOTICE);
    if (candidate.length <= maxLength) {
      low = mid;
    } else {
      high = mid - 1;
    }
  }

  return {
    url: compose(title, points.slice(0, low).join("") + TRUNCATION_NOTICE),
    truncated: true,
  };
}
