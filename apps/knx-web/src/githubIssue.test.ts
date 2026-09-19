/** Tests for the prefilled GitHub issue URL: what it contains, and how it shrinks. */
import { describe, expect, it } from "vitest";
import {
  buildIssueUrl,
  ISSUE_BASE_URL,
  MAX_ISSUE_URL_LENGTH,
  TRUNCATION_NOTICE,
} from "./githubIssue";

function bodyOf(url: string): string {
  return new URL(url).searchParams.get("body") ?? "";
}

function titleOf(url: string): string {
  return new URL(url).searchParams.get("title") ?? "";
}

describe("buildIssueUrl", () => {
  it("points at this repository's new-issue page and carries both fields verbatim", () => {
    const { url, truncated } = buildIssueUrl("It exploded", "# Report\n\nwith *markdown* & an & sign");
    expect(url.startsWith(`${ISSUE_BASE_URL}?`)).toBe(true);
    expect(titleOf(url)).toBe("It exploded");
    expect(bodyOf(url)).toBe("# Report\n\nwith *markdown* & an & sign");
    expect(truncated).toBe(false);
  });

  it("leaves a body that already fits completely alone", () => {
    const body = "x".repeat(100);
    const { url, truncated } = buildIssueUrl("t", body);
    expect(bodyOf(url)).toBe(body);
    expect(bodyOf(url)).not.toContain(TRUNCATION_NOTICE.trim());
    expect(truncated).toBe(false);
  });

  it("shortens an oversized body, stays inside the limit, and says it shortened it", () => {
    const body = "y".repeat(50_000);
    const { url, truncated } = buildIssueUrl("t", body);
    expect(truncated).toBe(true);
    expect(url.length).toBeLessThanOrEqual(MAX_ISSUE_URL_LENGTH);
    expect(bodyOf(url).length).toBeLessThan(body.length);
    expect(bodyOf(url).startsWith("yyy")).toBe(true);
  });

  it("tells the user where the rest of the report is", () => {
    const { url } = buildIssueUrl("t", "z".repeat(50_000));
    expect(bodyOf(url)).toContain("debug-report zip");
    expect(bodyOf(url).endsWith(TRUNCATION_NOTICE)).toBe(true);
  });

  it("keeps the whole notice even when the body is cut to almost nothing", () => {
    // A limit barely above the base URL: the body has to give up nearly
    // everything, but the notice explaining why must survive intact.
    const limit = ISSUE_BASE_URL.length + 600;
    const { url, truncated } = buildIssueUrl("t", "q".repeat(5_000), limit);
    expect(truncated).toBe(true);
    expect(url.length).toBeLessThanOrEqual(limit);
    expect(bodyOf(url).endsWith(TRUNCATION_NOTICE)).toBe(true);
  });

  it("never cuts an astral character in half, at any limit", () => {
    // Each of these is one code point and two UTF-16 code units. Cutting by
    // `.slice()` on the string lands between the two halves for roughly a
    // quarter of the possible limits, and a lone surrogate does not survive
    // percent-encoding — it comes back as U+FFFD. One limit would therefore
    // prove nothing: the sweep is the test.
    const body = "\u{1F9F1}".repeat(400);
    for (let limit = 500; limit < 800; limit += 1) {
      const { url } = buildIssueUrl("t", body, limit);
      const returned = new URL(url).searchParams.get("body") ?? "";
      expect(returned, `limit ${limit} produced a broken character`).not.toContain("\uFFFD");
      const cut = returned.replace(TRUNCATION_NOTICE, "");
      expect(Array.from(cut).every((c) => c === "\u{1F9F1}")).toBe(true);
    }
  });

  it("accounts for percent-encoding rather than counting raw characters", () => {
    // Each of these encodes to nine URL characters, so a builder that
    // measured the body instead of the URL would overshoot ninefold.
    const { url, truncated } = buildIssueUrl("t", "ä\n".repeat(3_000));
    expect(truncated).toBe(true);
    expect(url.length).toBeLessThanOrEqual(MAX_ISSUE_URL_LENGTH);
  });
});
