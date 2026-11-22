/** Unit tests for bus-monitor column ordering: chronological time, sequence ties (F09). */
import { describe, expect, it } from "vitest";
import { sortMonitorRows, type MonitorCell } from "./monitorRowOrder";

interface Row { seq: number; timestamp: string; label: string }
const row = (seq: number, timestamp: string, label = "x"): Row => ({ seq, timestamp, label });
const time = (r: Row): MonitorCell => ({ kind: "time", value: r.timestamp });
const seqs = (rows: Row[]) => rows.map((r) => r.seq);

describe("sortMonitorRows", () => {
  const rows = [
    row(1, "2026-11-22T10:00:00.100+00:00"),
    row(2, "2026-11-22T10:00:00.099999+00:00"),
    row(3, "2026-11-22T10:00:00+00:00"),
    row(4, "2026-11-22T11:00:00.5+01:00"),
    row(5, "2026-11-22T10:00:00.5Z"),
    row(6, "2026-11-22T10:00:00.000000001Z"),
  ];

  it("orders time ascending across widths, offsets and sub-millisecond digits", () => {
    // 3 (.0), 6 (.000000001), 2 (.099999), 1 (.1), then 4 and 5 (equal .5, arrival order).
    expect(seqs(sortMonitorRows(rows, false, time))).toEqual([3, 6, 2, 1, 4, 5]);
  });

  it("orders time descending but keeps arrival order inside equal instants", () => {
    expect(seqs(sortMonitorRows(rows, true, time))).toEqual([4, 5, 1, 2, 6, 3]);
  });

  it("breaks ties by sequence even when the input arrives shuffled", () => {
    const shuffled = [row(9, "2026-11-22T10:00:00Z"), row(2, "2026-11-22T10:00:00.0Z"), row(5, "2026-11-22T10:00:00.000Z")];
    expect(seqs(sortMonitorRows(shuffled, false, time))).toEqual([2, 5, 9]);
    expect(seqs(sortMonitorRows(shuffled, true, time))).toEqual([2, 5, 9]);
  });

  it("does not mutate its input", () => {
    const copy = [...rows];
    sortMonitorRows(rows, false, time);
    expect(rows).toEqual(copy);
  });

  it("keeps numbers numeric and text natural-numeric with a sequence tie-break", () => {
    const labelled = [row(3, "t", "1.1.10"), row(1, "t", "1.1.2"), row(2, "t", "1.1.2")];
    expect(seqs(sortMonitorRows(labelled, false, (r) => ({ kind: "text", value: r.label })))).toEqual([1, 2, 3]);
    expect(seqs(sortMonitorRows(labelled, true, (r) => ({ kind: "text", value: r.label })))).toEqual([3, 1, 2]);
    expect(seqs(sortMonitorRows(labelled, true, (r) => ({ kind: "number", value: r.seq })))).toEqual([3, 2, 1]);
  });
});
