/** WP-51.3b: the auto-open predicate and tracker. */
import { describe, expect, it } from "vitest";
import { autoOpenKey, createAutoOpenTracker, hasEligibleCandidates, shouldAutoOpen } from "./autoOpen";
import { proposal } from "./missingNames.test-fixtures";

const base = { key: "srv|1", openedFor: null, overlayOpen: false, saving: false };

describe("autoOpen", () => {
  it("builds the key from both incarnations", () => {
    expect(autoOpenKey({ server_incarnation: "a", project_incarnation: 2 })).toBe("a|2");
    expect(autoOpenKey({ server_incarnation: "a", project_incarnation: 3 })).not.toBe("a|2");
    expect(autoOpenKey({})).toBe("|");
  });
  it("opens when nothing blocks it", () => expect(shouldAutoOpen(base)).toBe(true));
  it("never opens read-only", () => expect(shouldAutoOpen({ ...base, readOnly: true })).toBe(false));
  it("does not open over another overlay", () => expect(shouldAutoOpen({ ...base, overlayOpen: true })).toBe(false));
  it("does not open while saving", () => expect(shouldAutoOpen({ ...base, saving: true })).toBe(false));
  it("does not reopen for the same key", () => expect(shouldAutoOpen({ ...base, openedFor: "srv|1" })).toBe(false));
  it("allows a new auto-open after the key changes", () => {
    const tracker = createAutoOpenTracker();
    tracker.markOpened("srv|1");
    expect(shouldAutoOpen({ ...base, openedFor: tracker.openedFor() })).toBe(false);
    expect(shouldAutoOpen({ ...base, key: "srv|2", openedFor: tracker.openedFor() })).toBe(true);
  });
  it("is not marked by manual use", () => {
    const tracker = createAutoOpenTracker();
    expect(tracker.openedFor()).toBeNull();
    expect(shouldAutoOpen({ ...base, openedFor: tracker.openedFor() })).toBe(true);
  });
  it("needs at least one proposal", () => {
    expect(hasEligibleCandidates({ items: [] })).toBe(false);
    expect(hasEligibleCandidates({ items: [proposal("area", 1, "Area 1")] })).toBe(true);
  });
});
