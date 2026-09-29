/** Loss-aware, local bus-monitor capture and export regressions. */
import { describe, expect, it } from "vitest";
import type { BusTelegramRow } from "./api";
import { CAPTURE_CAPACITY, appendCapturedRows, serializeBusCapture } from "./busMonitorCapture";

const row = (overrides: Partial<BusTelegramRow> = {}): BusTelegramRow => ({
  seq: 1,
  timestamp: "2026-09-29T01:02:03Z",
  source: "1.1.5",
  destination: "1/2/3",
  destinationName: "=HYPERLINK(\"unsafe\")",
  service: "GroupValueWrite",
  rawPayload: "0x01 (6-bit)",
  decoded: null,
  ...overrides,
});

const provenance = {
  sessionId: 7,
  serverIncarnation: "test-process",
  status: "closed" as const,
  serverDroppedBefore: 6,
  clientPrunedCount: 2,
  exportedAt: "2026-09-29T01:10:00Z",
};

describe("bus-monitor capture JSON v1", () => {
  it("roundtrips raw, decoded, error and closed-marker rows with loss metadata, without spreadsheet cells", () => {
    const rows = [
      row({ seq: 1 }),
      row({ seq: 2, decoded: { kind: "value", dpt: "DPST-1-1", text: "On" } }),
      row({ seq: 3, decoded: { kind: "error", dpt: "DPT-40", reason: "unsupportedDpt", text: "unsupported datapoint type", error: "unsupported datapoint type" } }),
      row({ seq: 4, decoded: { kind: "error", dpt: "DPST-1-1", reason: "decodeFailed", text: "wrong payload length", error: "wrong payload length" } }),
      row({ seq: 5, service: "SessionClosed", rawPayload: "gateway closed", decoded: null }),
    ];
    const contents = serializeBusCapture(rows, provenance);
    const document = JSON.parse(contents);
    expect(contents).toMatch(/^\{/);
    expect(document).toMatchObject({
      format: "knxbench-bus-monitor", version: 1, capacity: CAPTURE_CAPACITY,
      sessionId: 7, serverIncarnation: "test-process", status: "closed",
      serverDroppedBefore: 6, clientPrunedCount: 2, exportedAt: "2026-09-29T01:10:00Z",
    });
    expect(document.rows).toEqual(rows);
    expect(document.rows[0].destinationName).toBe('=HYPERLINK("unsafe")');
    expect(document.notice).toMatch(/not a complete bus history/i);
  });

  it("preserves formula-looking Unicode, quotes and newlines as inert JSON strings", () => {
    const destinationName = '=HYPERLINK("go")\t\r\nÜbertragung';
    const document = JSON.parse(serializeBusCapture([row({ destinationName })], provenance));
    expect(document.rows[0].destinationName).toBe(destinationName);
  });

  it("refuses more rows than its advertised retained capacity", () => {
    const rows = Array.from({ length: CAPTURE_CAPACITY + 1 }, (_, seq) => row({ seq }));
    expect(() => serializeBusCapture(rows, provenance)).toThrow(/capacity/);
  });

  it("prunes only the oldest client rows with an explicit counter; the server gap stays separate", () => {
    const initial = Array.from({ length: CAPTURE_CAPACITY }, (_, seq) => row({ seq }));
    const added = appendCapturedRows(initial, [row({ seq: CAPTURE_CAPACITY })]);
    expect(added.rows).toHaveLength(CAPTURE_CAPACITY);
    expect(added.rows[0].seq).toBe(1);
    expect(added.rows.at(-1)?.seq).toBe(CAPTURE_CAPACITY);
    expect(added.pruned).toBe(1);
    const snapshot = JSON.parse(serializeBusCapture(added.rows, { ...provenance, clientPrunedCount: added.pruned }));
    expect(snapshot.serverDroppedBefore).toBe(6);
    expect(snapshot.clientPrunedCount).toBe(1);
  });
});
