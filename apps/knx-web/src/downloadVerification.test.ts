/** Verifies plan-warning naming and the tester checklist derived from a download report. */
import { describe, expect, it } from "vitest";
import { checklist, planWarnings, runCompleted, type ChecklistInput } from "./downloadVerification";

const LOW_FIRST = "base-image-low-octet-first: the program declares no ParameterByteOrder … (KNOWN_LIMITATIONS §170, ADR-0120)";

function input(overrides: Partial<ChecklistInput> = {}): ChecklistInput {
  return {
    scope: "complete",
    warnings: [],
    plan: { groupLinks: 2 },
    outcome: { completed: true, written: "yes", restart: "acknowledged" },
    ...overrides,
  };
}

describe("planWarnings", () => {
  it("explains known rules and keeps unknown ones whole", () => {
    expect(planWarnings([LOW_FIRST, "base-image-order-unchecked: could not be checked", "a-future-rule: something", "no rule at all"])).toEqual([
      { rule: "base-image-low-octet-first", explanation: "deviceDownload.warning.baseImageLowOctetFirst", text: LOW_FIRST },
      { rule: "base-image-order-unchecked", explanation: "deviceDownload.warning.baseImageOrderUnchecked", text: "base-image-order-unchecked: could not be checked" },
      { rule: "a-future-rule", explanation: null, text: "a-future-rule: something" },
      { rule: "no rule at all", explanation: null, text: "no rule at all" },
    ]);
    expect(planWarnings([])).toEqual([]);
  });
});

describe("checklist", () => {
  it("asks for group addresses, status and settings after a complete download", () => {
    expect(checklist(input())).toEqual(["groupAddresses", "status", "parameters"]);
  });

  it("adds multi-octet values only for a parameter write that carried the byte-order warning", () => {
    expect(checklist(input({ warnings: [LOW_FIRST] }))).toEqual(["groupAddresses", "status", "parameters", "multiOctet"]);
    expect(checklist(input({ scope: "partial-group-addresses", warnings: [LOW_FIRST] }))).toEqual(["groupAddresses", "status"]);
  });

  it("follows the scope and skips group checks without links", () => {
    expect(checklist(input({ scope: "partial-parameters" }))).toEqual(["parameters"]);
    expect(checklist(input({ scope: "partial-both" }))).toEqual(["groupAddresses", "status", "parameters"]);
    expect(checklist(input({ plan: { groupLinks: 0 } }))).toEqual(["parameters"]);
  });

  it("puts an unconfirmed restart first", () => {
    expect(checklist(input({ outcome: { completed: true, written: "yes", restart: "unconfirmed" } }))[0]).toBe("restart");
  });

  it("has nothing to check after a run that did not complete", () => {
    for (const outcome of [
      null,
      { completed: false, written: "partially" as const, restart: null },
      { completed: false, written: "no" as const, restart: null },
    ]) {
      expect(checklist(input({ outcome }))).toEqual([]);
      expect(runCompleted(input({ outcome }))).toBe(false);
    }
    expect(runCompleted(input())).toBe(true);
  });
});
