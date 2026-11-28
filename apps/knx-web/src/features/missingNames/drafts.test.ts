/** WP-51: missing-name drafts keep edits, warn on duplicates and re-propose unedited children. */
import { describe, expect, it } from "vitest";
import { targetKey } from "./client";
import {
  applyReevaluation, draftParents, duplicateKeys, groupByKind, isSelectable, keepReproposals, mergePage, nameProblem,
  occurrenceKey, selectedEntries, setName, setSelected,
} from "./drafts";
import { proposal } from "./missingNames.test-fixtures";

const area = proposal("area", 1, "Area 1");
const line = proposal("line", 2, "1.2 Line");
const device = proposal("device", 4, null);
const base = () => mergePage(new Map(), [device, line, area]);

describe("missing-name drafts", () => {
  it("seeds unselected drafts from suggestions and keeps existing drafts on a later page", () => {
    let drafts = base();
    expect(drafts.get("area:1")).toMatchObject({ name: "Area 1", edited: false, selected: false });
    expect(drafts.get("device:4")?.name).toBe("");
    drafts = setName(drafts, "area:1", "Hall");
    drafts = mergePage(drafts, [proposal("area", 1, "Other"), proposal("device", 5, "D")]);
    expect(drafts.get("area:1")?.name).toBe("Hall");
    expect(drafts.size).toBe(4);
  });

  it("flags blank and over-long names by code points", () => {
    const drafts = mergePage(new Map(), [proposal("device", 1, "😀😀", { max: 2 }), proposal("device", 2, "abc", { max: 2 })]);
    expect(nameProblem(drafts.get("device:1")!)).toBeNull();
    expect(nameProblem(drafts.get("device:2")!)).toBe("tooLong");
    expect(nameProblem(base().get("device:4")!)).toBe("blank");
  });

  it("marks a typed name as edited and returns the same map for no change", () => {
    const drafts = base();
    expect(setName(drafts, "area:1", "Area 1")).toBe(drafts);
    expect(setName(drafts, "area:1", "Hall").get("area:1")).toMatchObject({ name: "Hall", edited: true });
  });

  it("selects only uniquely identified targets", () => {
    const dup = proposal("device", 7, "D", { identity: { status: "duplicate_id", occurrences: 2 } });
    const zero = proposal("device", 0, "Z", { identity: { status: "invalid_zero_id" } });
    expect(isSelectable(dup)).toBe(false);
    let drafts = mergePage(base(), [dup, zero]);
    drafts = setSelected(drafts, ["area:1", "device:7", "device:0"], true);
    expect(selectedEntries(drafts)).toEqual([{ target: area.candidate.target, name: "Area 1" }]);
    expect(selectedEntries(setSelected(drafts, ["area:1"], false))).toEqual([]);
  });

  it("offers only selected, admissible parent drafts as reevaluation context", () => {
    let drafts = setSelected(base(), ["area:1", "line:2", "device:4"], true);
    expect(draftParents(drafts).map((entry) => targetKey(entry.target))).toEqual(["line:2", "area:1"]);
    drafts = setName(drafts, "area:1", "  ");
    expect(draftParents(drafts).map((entry) => targetKey(entry.target))).toEqual(["line:2"]);
  });

  it("warns on the server flag while unchanged and on equal selected names of one kind", () => {
    let drafts = mergePage(new Map(), [proposal("device", 1, "Lamp", { duplicate_label: true }), proposal("device", 2, "Dim"),
      proposal("device", 3, "Dim"), proposal("line", 3, "Dim")]);
    expect([...duplicateKeys(drafts)]).toEqual(["device:1"]);
    drafts = setSelected(drafts, ["device:2", "device:3", "line:3"], true);
    expect([...duplicateKeys(drafts)].sort()).toEqual(["device:1", "device:2", "device:3"]);
    drafts = setName(drafts, "device:1", "Lamp 2");
    expect(duplicateKeys(drafts).has("device:1")).toBe(false);
  });

  it("re-proposes unedited children and keeps manual edits on a parent change", () => {
    let drafts = mergePage(base(), [proposal("line", 3, "1.3")]);
    drafts = setName(drafts, "line:3", "Mine");
    const result = { invalidated_targets: [line.candidate.target, { kind: "line", id: 3 } as const, { kind: "line", id: 99 } as const],
      proposals: [proposal("line", 2, "Hall 1.2"), proposal("line", 3, "Hall 1.3")] };
    drafts = applyReevaluation(drafts, result);
    expect(drafts.get("line:2")).toMatchObject({ name: "Hall 1.2", recheck: "updated", edited: false });
    expect(drafts.get("line:3")).toMatchObject({ name: "Mine", recheck: "kept", edited: true });
    expect(drafts.get("line:3")?.proposal.proposed_name).toBe("Hall 1.3");
    expect(drafts.has("line:99")).toBe(false);
    expect(setName(drafts, "line:2", "x").get("line:2")?.recheck).toBeNull();
  });

  it("seeds a child first seen on a later page from its kept re-proposal", () => {
    let kept = keepReproposals(new Map(), { invalidated_targets: [line.candidate.target, { kind: "line", id: 3 } as const],
      proposals: [proposal("line", 2, "Hall 1.2"), proposal("line", 3, "Hall 1.3")] });
    kept = keepReproposals(kept, { invalidated_targets: [{ kind: "line", id: 3 }], proposals: [] });
    expect([...kept.keys()]).toEqual(["line:2"]);
    const drafts = mergePage(new Map(), [line, proposal("line", 3, "1.3"), area], kept);
    expect(drafts.get("line:2")).toMatchObject({ name: "Hall 1.2", recheck: "updated", edited: false });
    expect(drafts.get("line:2")?.proposal.proposed_name).toBe("Hall 1.2");
    expect(drafts.get("line:3")).toMatchObject({ name: "1.3", recheck: null });
    const edited = setName(drafts, "line:2", "Mine");
    expect(mergePage(edited, [line], kept).get("line:2")?.name).toBe("Mine");
  });

  it("groups page items by kind in project order and keys occurrences apart", () => {
    expect(groupByKind([device, line, area]).map((group) => group.kind)).toEqual(["area", "line", "device"]);
    const second = proposal("device", 4, null, { occurrence: 1 });
    expect(occurrenceKey(device)).not.toBe(occurrenceKey(second));
  });
});
