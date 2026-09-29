/** Bounded bus-monitor statistics over retained, real telegrams. */
import { describe, expect, it } from "vitest";
import type { BusTelegramRow } from "./api";
import { CAPTURE_CAPACITY, appendCapturedRows } from "./busMonitorCapture";
import { calculateBusMonitorStatistics, STATISTICS_TOP_LIMIT } from "./busMonitorStatistics";

const row = (overrides: Partial<BusTelegramRow> = {}): BusTelegramRow => ({
  seq: 1,
  timestamp: "2026-09-29T01:00:00Z",
  source: "1.1.5",
  destination: "1/2/3",
  destinationName: null,
  service: "GroupValueWrite",
  rawPayload: "0x01 (6-bit)",
  decoded: null,
  ...overrides,
});

describe("retained bus statistics", () => {
  it("counts actual services, group destinations and sending devices without treating a close marker as traffic", () => {
    const stats = calculateBusMonitorStatistics([
      row({ seq: 1 }),
      row({ seq: 2, source: "1.1.6" }),
      row({ seq: 3, destination: "1/2/4", service: "GroupValueRead", source: "1.1.5" }),
      row({ seq: 4, source: "-", destination: "-", service: "SessionClosed", rawPayload: "gateway closed" }),
    ]);
    expect(stats.observedRows).toBe(3);
    expect(stats.services).toEqual([
      { label: "GroupValueWrite", count: 2 },
      { label: "GroupValueRead", count: 1 },
    ]);
    expect(stats.destinations).toEqual([
      { label: "1/2/3", count: 2 },
      { label: "1/2/4", count: 1 },
    ]);
    expect(stats.sources).toEqual([
      { label: "1.1.5", count: 2 },
      { label: "1.1.6", count: 1 },
    ]);
  });

  it("bounds both retained rows and every displayed ranking, breaking count ties deterministically", () => {
    const incoming = Array.from({ length: CAPTURE_CAPACITY * 5 }, (_, seq) => row({
      seq,
      destination: `1/${seq % 20}/${seq % 200}`,
      source: `1.1.${seq % 100}`,
      service: `Service-${seq % 80}`,
    }));
    const capture = appendCapturedRows([], incoming);
    expect(capture.rows).toHaveLength(CAPTURE_CAPACITY);
    expect(capture.pruned).toBe(incoming.length - CAPTURE_CAPACITY);
    const stats = calculateBusMonitorStatistics(capture.rows);
    expect(stats.observedRows).toBe(CAPTURE_CAPACITY);
    expect(stats.services).toHaveLength(STATISTICS_TOP_LIMIT);
    expect(stats.destinations).toHaveLength(STATISTICS_TOP_LIMIT);
    expect(stats.sources).toHaveLength(STATISTICS_TOP_LIMIT);
    expect(stats.services.map((entry) => entry.label)).toEqual(
      [...stats.services.map((entry) => entry.label)].sort(),
    );
  });
});
