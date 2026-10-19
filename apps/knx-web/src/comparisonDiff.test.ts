/** Byte marking and the differences-only view never hide a differing or unread byte. */
import { describe, expect, it } from "vitest";
import { comparisonRowView, markRawBytes, provenEqual, type ComparisonChange } from "./comparisonDiff";
import type { DecodedComparisonRow } from "./deviceComparisons";

function raw(project: string | null, device: string | null, address = 0x4400): DecodedComparisonRow {
  return { kind: "raw", label: "", address, project, device, reason: null };
}
function decoded(kind: string, project: string | null, device: string | null, reason: string | null = null): DecodedComparisonRow {
  return { kind, label: "1", address: 0x4003, project, device, reason };
}
const marks = (side: { mark: string }[] | null) => side?.map((b) => b.mark);

describe("markRawBytes", () => {
  it("marks a single differing byte on both sides and nothing else", () => {
    const m = markRawBytes(raw("12 03 A5", "12 04 A5"), []);
    expect(marks(m.project)).toEqual(["equal", "different", "equal"]);
    expect(marks(m.device)).toEqual(["equal", "different", "equal"]);
    expect(m.device!.map((b) => b.text)).toEqual(["12", "04", "A5"]);
    expect(m.project!.map((b) => b.text)).toEqual(["12", "03", "A5"]);
    expect([m.different, m.unknown]).toEqual([1, 0]);
  });
  it("marks an adjacent run byte by byte", () => {
    const m = markRawBytes(raw("01 02 03 04", "01 FF FE 04"), []);
    expect(marks(m.device)).toEqual(["equal", "different", "different", "equal"]);
    expect(m.different).toBe(2);
  });
  it("marks separate runs without marking the equal bytes between them", () => {
    const m = markRawBytes(raw("01 02 03 04 05", "FF 02 03 04 FE"), []);
    expect(marks(m.project)).toEqual(["different", "equal", "equal", "equal", "different"]);
    expect(m.different).toBe(2);
  });
  it("marks every byte equal when all were read and match", () => {
    const m = markRawBytes(raw("21 22", "21 22"), []);
    expect(marks(m.device)).toEqual(["equal", "equal"]);
    expect([m.different, m.unknown]).toEqual([0, 0]);
  });
  it("marks bytes without a known counterpart as unread, not as differing", () => {
    const m = markRawBytes(raw(null, "AA BB"), []);
    expect(m.project).toBeNull();
    expect(marks(m.device)).toEqual(["unknown", "unknown"]);
    expect([m.different, m.unknown]).toEqual([0, 2]);
  });
  it("marks unequal lengths and malformed tokens as unread", () => {
    expect(marks(markRawBytes(raw("01 02", "01"), []).project)).toEqual(["equal", "unknown"]);
    const m = markRawBytes(raw("01 02", "01 ??"), []);
    expect(marks(m.device)).toEqual(["equal", "unknown"]);
    expect(m.device![1].text).toBe("??");
    expect([m.different, m.unknown]).toEqual([0, 1]);
  });
  it("uses the server's byte-level changes so a reported difference is never shown as equal", () => {
    const changes: ComparisonChange[] = [{ address: 0x4401, segment: null, device: [0xBB], project: [0xCC] },
      { address: 0x5000, segment: null, device: [1], project: [2] }];
    const unread = markRawBytes(raw(null, "AA BB"), changes);
    expect(marks(unread.device)).toEqual(["unknown", "different"]);
    expect([unread.different, unread.unknown]).toEqual([1, 1]);
    const contradicted = markRawBytes(raw("21 22", "21 22"), changes);
    expect(marks(contradicted.device)).toEqual(["equal", "different"]);
  });
});

describe("provenEqual", () => {
  it("proves only fully read, equal raw ranges and known, equal decoded rows", () => {
    expect(provenEqual(raw("21 22", "21 22"), [])).toBe(true);
    expect(provenEqual(raw("21 22", "21 23"), [])).toBe(false);
    expect(provenEqual(raw(null, "21 22"), [])).toBe(false);
    expect(provenEqual(raw("21", "21 22"), [])).toBe(false);
    expect(provenEqual(raw("", ""), [])).toBe(false);
    expect(provenEqual(raw("21 22", "21 22"), [{ address: 0x4400, segment: null, device: [0x21], project: [0x30] }])).toBe(false);
    expect(provenEqual({ ...raw("21", "21"), reason: "explained" }, [])).toBe(false);
    expect(provenEqual(decoded("groupAddress", "2/2/3", "2/2/3"), [])).toBe(true);
    expect(provenEqual(decoded("groupAddress", "2/2/3", "2/2/4"), [])).toBe(false);
    expect(provenEqual(decoded("parameter", "12", null, "parameterNotDecoded"), [])).toBe(false);
    expect(provenEqual(decoded("parameter", null, null), [])).toBe(false);
    expect(provenEqual(decoded("parameter", "12", "12", "parameterNotDecoded"), [])).toBe(false);
    expect(provenEqual(decoded("futureKind", "12", "12"), [])).toBe(false);
  });
});

describe("comparisonRowView", () => {
  const rows: DecodedComparisonRow[] = [
    decoded("groupAddress", "2/2/3", "2/2/3"),
    decoded("groupAddress", "2/2/3", "2/2/4"),
    decoded("parameter", "12", null, "parameterNotDecoded"),
    decoded("association", null, null),
    raw("21 22", "21 22", 0x4000),
    raw("01 02 03", "01 02 04", 0x4100),
    raw(null, "AA", 0x4200),
  ];
  it("opens with differing and unknown rows only and counts the hidden equal rows", () => {
    const view = comparisonRowView(rows, [], false);
    expect(view.visible.map((v) => v.index)).toEqual([1, 2, 3, 5, 6]);
    expect(view.visible.every((v) => v.row === rows[v.index])).toBe(true);
    expect([view.hidden, view.equal]).toEqual([2, 2]);
  });
  it("keeps every original row and read range, in order, when all rows are shown", () => {
    const view = comparisonRowView(rows, [], true);
    expect(view.visible.map((v) => v.row)).toEqual(rows);
    expect(view.visible.filter((v) => v.row.kind === "raw").map((v) => v.row.address)).toEqual([0x4000, 0x4100, 0x4200]);
    expect([view.hidden, view.equal]).toEqual([0, 2]);
  });
  it("hides everything only when everything is proven equal, and never mutates the response", () => {
    const equal = [decoded("groupAddress", "2/2/3", "2/2/3"), raw("21", "21")];
    const before = JSON.stringify(equal);
    const view = comparisonRowView(equal, [], false);
    expect(view.visible).toEqual([]); expect(view.hidden).toBe(2);
    expect(JSON.stringify(equal)).toBe(before);
  });
  it("handles a result without any raw range", () => {
    const view = comparisonRowView([decoded("association", "TSAP 1", "TSAP 1"), decoded("association", "TSAP 1", null)], [], false);
    expect(view.visible.map((v) => v.index)).toEqual([1]); expect(view.hidden).toBe(1);
  });
});
