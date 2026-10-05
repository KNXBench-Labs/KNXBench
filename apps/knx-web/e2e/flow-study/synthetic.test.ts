/** U19 study: the synthetic stream is deterministic and emits each event at its scheduled time. */
import { describe, expect, it } from "vitest";
import { eventStream, syntheticInstallation } from "./synthetic";

describe("synthetic installation", () => {
  it("is deterministic, uses synthetic addresses and reaches the requested edge count", () => {
    const a = syntheticInstallation(3, 40, 90);
    expect(syntheticInstallation(3, 40, 90)).toEqual(a);
    expect(a.devices.every((id) => id.startsWith("9."))).toBe(true);
    expect(a.directedEdges).toBeGreaterThanOrEqual(90);
  });
});

describe("event stream", () => {
  it("makes an event due exactly at its scheduled time, not one period later", () => {
    const stream = eventStream(syntheticInstallation(1, 10, 12), 1, 4, 1000);
    expect(stream.due(999)).toHaveLength(0);
    expect(stream.due(1000).map((event) => event.observedAtMs)).toEqual([1000]);
    expect(stream.due(1249)).toHaveLength(0);
    expect(stream.due(1250).map((event) => event.observedAtMs)).toEqual([1250]);
    expect(stream.due(2000).map((event) => event.seq)).toEqual([3, 4, 5]);
  });
});
