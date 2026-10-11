/** Verifies publication admission and SemVer ordering without network or project state. */
import { expect, it } from "vitest";
import { selectReleaseUpdate } from "./releaseUpdates";

export function release(tag: string, overrides: Record<string, unknown> = {}) {
  return { tag_name: tag, draft: false, prerelease: tag.includes("-"), published_at: "2026-10-11T20:00:00Z", ...overrides };
}

it("recognizes a published alpha.10 as newer than alpha.9, ignoring build metadata", () => {
  expect(selectReleaseUpdate("0.1.0-alpha.9+g1234567", [release("v0.1.0-alpha.10")]))
    .toEqual({ version: "0.1.0-alpha.10", url: "https://github.com/KNXBench-Labs/KNXBench/releases/tag/v0.1.0-alpha.10", prerelease: true });
});

it.each([
  ["0.1.0-alpha.9", [release("v0.1.0-alpha.9")]],
  ["0.1.0-alpha.10", [release("v0.1.0-alpha.9")]],
  ["1.0.0", [release("v2.0.0-beta.1")]],
  ["1.0.0", [release("v2.0.0", { prerelease: true })]],
  ["1.0.0", [release("v2.0.0", { draft: true })]],
  ["1.0.0", [release("v2.0.0", { published_at: null })]],
  ["1.0.0", [release("v2.0.0", { published_at: "unknown" })]],
  ["unknown", [release("v2.0.0")]],
  ["1.0.0", [release("v02.0.0")]],
  ["0.1.0-alpha.9", [release("v0.1.0-alpha.010")]],
  ["1.0.0", { message: "API rate limit exceeded" }],
  ["1.0.0", [null, {}, release("https://evil.example")]],
])("does not advertise an ineligible publication (%s)", (current, rows) => {
  expect(selectReleaseUpdate(current as string, rows)).toBeNull();
});
it("selects by semantic precedence, not API order or publication date", () => {
  expect(selectReleaseUpdate("0.1.0-alpha.9", [release("v0.1.0-alpha.10"), release("v0.1.0"), release("v0.1.0-rc.1")])?.version).toBe("0.1.0");
});
it("keeps unsafe API links out and constructs the official tag link itself", () => {
  expect(selectReleaseUpdate("1.0.0", [release("v2.0.0", { html_url: "javascript:alert(1)" })])?.url).toBe("https://github.com/KNXBench-Labs/KNXBench/releases/tag/v2.0.0");
});
it.each([
  ["1.0.0-alpha", "v1.0.0-alpha.1"], ["1.0.0-alpha.1", "v1.0.0-beta"],
  ["1.0.0-beta.11", "v1.0.0-rc.1"], ["1.0.0-1", "v1.0.0-alpha"],
  ["1.0.0-alpha.9007199254740992", "v1.0.0-alpha.9007199254740993"],
  ["1.0.0", "v1.1.0"], ["1.9.9", "v2.0.0"],
])("orders %s before %s", (current, tag) => expect(selectReleaseUpdate(current, [release(tag)])).not.toBeNull());
