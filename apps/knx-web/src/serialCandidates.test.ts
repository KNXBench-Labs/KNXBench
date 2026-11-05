/** Reject malformed candidate payloads at the real HTTP client before any path can be selected. */
import { afterEach, expect, it, vi } from "vitest";
import { listSerialCandidates } from "./api";
import { admitSerialCandidates, type SerialCandidates } from "./serialCandidates";

const row = { path: "/dev/serial/by-id/usb-fictional-if00", target: "/dev/ttyACM0" };
const available = { status: "available", candidates: [row], truncated: false, omittedEntries: 0 } satisfies SerialCandidates;
afterEach(() => vi.unstubAllGlobals());
it("requests only the passive endpoint explicitly with strict admission", async () => {
  const fetcher = vi.fn().mockResolvedValue(new Response(JSON.stringify(available), { status: 200 }));
  vi.stubGlobal("fetch", fetcher);
  expect(fetcher).not.toHaveBeenCalled();
  expect(await listSerialCandidates()).toEqual(available);
  expect(fetcher).toHaveBeenCalledExactlyOnceWith("/api/bus/serial-candidates", { headers: undefined, method: "GET", cache: "no-store" });
});
it.each<SerialCandidates>([
  { status: "available", candidates: [], truncated: false, omittedEntries: 0 },
  { status: "available", candidates: [row], truncated: true, omittedEntries: 511 },
  { status: "unavailable", reason: "missingDirectory" },
  { status: "unavailable", reason: "permissionDenied" },
  { status: "unavailable", reason: "io" },
  { status: "unsupported" },
])("preserves the exact typed result: %j", value => expect(admitSerialCandidates(value)).toEqual(value));
const malformed: unknown[] = [
  null, [], {}, { status: "available" }, { ...available, extra: true },
  { ...available, candidates: null }, { ...available, truncated: "false" },
  { ...available, omittedEntries: -1 }, { ...available, omittedEntries: 0.5 },
  { ...available, omittedEntries: 512 }, { ...available, omittedEntries: null },
  { status: "unavailable", reason: "busy" }, { status: "unsupported", candidates: [] },
  { status: "unavailable", reason: "io", candidates: [] },
  { ...available, candidates: [row, row] },
  { ...available, candidates: [row, { ...row, path: "/dev/serial/by-id/usb-other" }] },
  { ...available, candidates: Array.from({ length: 65 }, (_, i) => ({ path: `/dev/serial/by-id/usb-${i}`, target: `/dev/ttyUSB${i}` })) },
  ...["", "/dev/ttyUSB0", "/dev/serial/by-id/pci-1", "/dev/serial/by-id/usb-", "/dev/serial/by-id/usb-a/evil", "/dev/serial/by-id/usb-a\\evil", "/dev/serial/by-id/usb-a\n", "/dev/serial/by-id/usb-a ", "/dev/serial/by-id/usb-\u0085", "/dev/serial/by-id/usb-\ud800", `/dev/serial/by-id/usb-${"x".repeat(252)}`].map(path => ({ ...available, candidates: [{ ...row, path }] })),
  ...["/dev/ttyUSB", "/dev/ttyACM1\n", "/dev/nested/ttyUSB1", "/elsewhere/ttyUSB1", "/dev/ttyUSB１", "/dev/ttyUSB1evil"].map(target => ({ ...available, candidates: [{ ...row, target }] })),
  { ...available, candidates: [{ ...row, compatible: true }] },
];
it.each(malformed.map((value, i) => [i, value] as const))("refuses malformed server payload %i through HTTP", async (_, value) => {
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(JSON.stringify(value), { status: 200 })));
  await expect(listSerialCandidates()).rejects.toThrow("Malformed USB-serial candidate response");
});
it.each([404, 500])("preserves HTTP %i without searching or connecting", async status => {
  const fetcher = vi.fn().mockResolvedValue(new Response(JSON.stringify({ error: "synthetic failure" }), { status }));
  vi.stubGlobal("fetch", fetcher);
  await expect(listSerialCandidates()).rejects.toMatchObject({ status });
  expect(fetcher).toHaveBeenCalledTimes(1);
});
