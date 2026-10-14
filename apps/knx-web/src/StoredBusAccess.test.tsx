/** Stored bus access: read-only, hidden until asked, one record at a time (Goal B3). */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { beforeEach, expect, it, vi } from "vitest";
import type { StoredBusAccess, StoredRecord } from "./api";

const api = vi.hoisted(() => ({
  storedBusAccess: vi.fn(),
  revealStoredBusAccess: vi.fn(),
  isStoredBusAccessChanged: vi.fn(),
}));
vi.mock("./api", () => ({ ...api, errorMessage: String }));
import { LineStoredBusAccess, StoredBusAccessDialog } from "./StoredBusAccess";

// RFC 5737 / locally administered, fictional.
const HOST = "198.51.100.23", DEVICE_IP = "192.0.2.77", MAC = "02:00:5e:10:00:01";

function access(overrides: Partial<StoredRecord> = {}): StoredRecord {
  return {
    recordId: "1:aaaaaaaaaaaaaaaa", element: "BusAccess", status: "read", revealed: false, unread: [],
    owner: { kind: "line", id: 7, address: "1.1", name: "Ground floor", reason: null },
    source: { sourcePath: "P-0001/0.xml", xpath: "/KNX/…/Line[@Id='P-0001-0_L-2']/Segment/BusAccess", size: 120, sha256: "a".repeat(64) },
    fields: [
      { name: "Name", part: "attribute", known: true, value: null, hidden: { class: "text", length: 19 } },
      { name: "Type", part: "parameter", known: true, value: "KnxIpTunneling", hidden: null },
      { name: "HostAddress", part: "parameter", known: true, value: null, hidden: { class: "ipv4Address", length: 13 } },
    ],
    ...overrides,
  };
}
function ipConfig(): StoredRecord {
  return {
    recordId: "0:bbbbbbbbbbbbbbbb", element: "IPConfig", status: "read", revealed: false, unread: [],
    owner: { kind: "unattributed", id: null, address: null, name: null, reason: "notKeyed" },
    source: { sourcePath: "P-0001/0.xml", xpath: "/KNX/…/DeviceInstance/IPConfig", size: 60, sha256: "b".repeat(64) },
    fields: [
      { name: "IPAddress", part: "attribute", known: true, value: null, hidden: { class: "ipv4Address", length: 10 } },
      { name: "MACAddress", part: "attribute", known: true, value: null, hidden: { class: "macAddress", length: 17 } },
    ],
  };
}
const revealedAccess = access({ revealed: true, fields: [
  { name: "Name", part: "attribute", known: true, value: "Synthetic interface", hidden: null },
  { name: "Type", part: "parameter", known: true, value: "KnxIpTunneling", hidden: null },
  { name: "HostAddress", part: "parameter", known: true, value: HOST, hidden: null },
] });

beforeEach(() => {
  vi.clearAllMocks();
  api.storedBusAccess.mockResolvedValue({ records: [ipConfig(), access()] } satisfies StoredBusAccess);
  api.revealStoredBusAccess.mockResolvedValue(revealedAccess);
  api.isStoredBusAccessChanged.mockReturnValue(false);
});

async function render(node: React.ReactNode) {
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(node));
  await act(async () => {});
  const button = (text: string) => [...host.querySelectorAll<HTMLButtonElement>("button")].filter(b => b.textContent === text);
  return { host, button, cleanup: async () => { await act(async () => root.unmount()); host.remove(); } };
}

it("lists every stored record read-only, with endpoints hidden and owners stated", async () => {
  const v = await render(<StoredBusAccessDialog onClose={() => {}} />);
  const text = v.host.textContent ?? "";
  expect(text).toContain("KNXBench never connects to these addresses and never uses them as a gateway");
  expect(text).toContain("On line 1.1 Ground floor");
  expect(text).toContain("Not assigned: the stored record names no line or device.");
  expect(text).toContain("hidden (IPv4 address, 13 characters)");
  expect(text).toContain("hidden (MAC address, 17 characters)");
  expect(text).toContain("KnxIpTunneling");
  for (const secret of [HOST, DEVICE_IP, MAC]) expect(text).not.toContain(secret);
  // Nothing here connects or writes: only Show values, the source details and Close.
  const labels = [...v.host.querySelectorAll("button")].map(b => b.textContent);
  expect(labels.filter(l => l !== "Show values" && l !== "Close")).toEqual([]);
  expect(v.host.querySelectorAll("input, select, textarea")).toHaveLength(0);
  await v.cleanup();
});

it("shows one record's values only when asked, and hides them again without asking the server", async () => {
  const v = await render(<StoredBusAccessDialog onClose={() => {}} />);
  const record = () => v.host.querySelector<HTMLElement>('[data-record-id="1:aaaaaaaaaaaaaaaa"]')!;
  await act(async () => record().querySelector<HTMLButtonElement>("button")!.click());
  expect(api.revealStoredBusAccess).toHaveBeenCalledExactlyOnceWith("1:aaaaaaaaaaaaaaaa");
  expect(record().textContent).toContain(HOST);
  // The other record stays hidden.
  expect(v.host.querySelector('[data-record-id="0:bbbbbbbbbbbbbbbb"]')!.textContent).not.toContain(DEVICE_IP);
  await act(async () => v.button("Hide values")[0].click());
  expect(record().textContent).not.toContain(HOST);
  expect(api.revealStoredBusAccess).toHaveBeenCalledOnce();
  await v.cleanup();
});

it("reads the list again when the record changed, and says so", async () => {
  api.revealStoredBusAccess.mockRejectedValue(new Error("gone"));
  api.isStoredBusAccessChanged.mockReturnValue(true);
  const v = await render(<StoredBusAccessDialog onClose={() => {}} />);
  await act(async () => v.button("Show values")[1].click());
  await act(async () => {});
  expect(v.host.textContent).toContain("The project changed; the list was read again.");
  expect(api.storedBusAccess).toHaveBeenCalledTimes(2);
  await v.cleanup();
});

it("says what was kept but not read, and where every record came from", async () => {
  api.storedBusAccess.mockResolvedValue({ records: [
    access({ recordId: "2:c", status: "tooLarge", fields: [] }),
    access({ recordId: "3:d", unread: ["parameterNotSplit", "Extra"] }),
  ] });
  const v = await render(<StoredBusAccessDialog onClose={() => {}} />);
  const text = v.host.textContent ?? "";
  expect(text).toContain("Not read: larger than 64 KiB. Kept unchanged.");
  expect(text).toContain("does not split into key=value parts");
  expect(text).toContain("Kept, not read: Extra");
  expect(text).toContain("/KNX/…/Line[@Id='P-0001-0_L-2']/Segment/BusAccess");
  expect(text).toContain("a".repeat(64));
  await v.cleanup();
});

it("a line shows only its own records, and nothing at all when it has none", async () => {
  const v = await render(<LineStoredBusAccess lineId={7} />);
  expect(v.host.querySelectorAll("[data-record-id]")).toHaveLength(1);
  expect(v.host.textContent).toContain("Stored bus access (from ETS)");
  await v.cleanup();
  const none = await render(<LineStoredBusAccess lineId={8} />);
  expect(none.host.textContent).toBe("");
  await none.cleanup();
});

it("an empty project says so", async () => {
  api.storedBusAccess.mockResolvedValue({ records: [] });
  const v = await render(<StoredBusAccessDialog onClose={() => {}} />);
  expect(v.host.textContent).toContain("This project stores no bus access or IP configuration.");
  await v.cleanup();
});

it("two inspectors on one line still name their own heading (no duplicate ids)", async () => {
  // The properties panel and the structure workspace show the same line at once.
  const v = await render(<><LineStoredBusAccess lineId={7} /><LineStoredBusAccess lineId={7} /></>);
  const sections = [...v.host.querySelectorAll("section.stored-bus-access-line")];
  expect(sections).toHaveLength(2);
  const ids = sections.map(section => section.getAttribute("aria-labelledby"));
  expect(new Set(ids).size).toBe(2);
  for (const [i, section] of sections.entries()) expect(section.querySelector("h3")?.id).toBe(ids[i]);
  await v.cleanup();
});
