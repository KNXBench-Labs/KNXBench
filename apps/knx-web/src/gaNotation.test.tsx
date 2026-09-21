/** Tests fixed group-address notation plus compatible input and search spellings. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";

const apiMock = vi.hoisted(() => ({
  writeBusValue: vi.fn(),
  unlinkComObject: vi.fn(),
}));

vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}));

import BusComposeForm from "./BusComposeForm";
import GroupAddressTable from "./GroupAddressTable";
import {
  canonicalGroupAddress,
  formatGroupAddress,
  groupAddressLevels,
  groupAddressMatches,
  groupAddressSpellings,
} from "./gaNotation";
import { matchEntries } from "./searchMatch";
import type { SearchEntry } from "./treeUtils";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { InstallationNode } from "./bindings/InstallationNode";
import { resetSettingsForTests } from "./settingsStore";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const hosts: Array<() => void> = [];

afterEach(() => {
  hosts.splice(0).forEach((dispose) => dispose());
  resetSettingsForTests();
  localStorage.clear();
  resetSettingsForTests();
  vi.clearAllMocks();
});

function mount(node: React.ReactNode): HTMLDivElement {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(node));
  hosts.push(() => {
    act(() => root.unmount());
    host.remove();
  });
  return host;
}

// ---------------------------------------------------------------------
// The renderer and the reader.
// ---------------------------------------------------------------------

it("renders group addresses in fixed slash notation and leaves everything else alone", () => {
  expect(formatGroupAddress("1/2/3")).toBe("1/2/3");
  expect(formatGroupAddress("1.2.3")).toBe("1/2/3");
  expect(formatGroupAddress("4.100")).toBe("4/100");
  expect(formatGroupAddress("4/100")).toBe("4/100");

  // A `Free`-style address is a plain decimal with no separator at all.
  // Fixed notation has nothing to change about it.
  expect(formatGroupAddress("2307")).toBe("2307");
  expect(groupAddressLevels("2307")).toBeNull();

  // Not a blind `replace`: a name, a path, a DPT id and a mixed-separator
  // string all come back untouched.
  expect(formatGroupAddress("Kitchen / Ceiling")).toBe("Kitchen / Ceiling");
  expect(formatGroupAddress("/home/knx/projects/example.knxdb")).toBe(
    "/home/knx/projects/example.knxdb",
  );
  expect(formatGroupAddress("DPST-1-1")).toBe("DPST-1-1");
  expect(formatGroupAddress("1/2.3")).toBe("1/2.3");
  expect(formatGroupAddress("1/2/3/4")).toBe("1/2/3/4");
  expect(formatGroupAddress("")).toBe("");
});

it("reads either notation back to the canonical form the API expects", () => {
  expect(canonicalGroupAddress("1/2/3")).toBe("1/2/3");
  expect(canonicalGroupAddress("1.2.3")).toBe("1/2/3");
  expect(canonicalGroupAddress("  1.2.3  ")).toBe("1/2/3");
  expect(canonicalGroupAddress("4.100")).toBe("4/100");
  // Unrecognised text is handed over untouched, so the server's error
  // names what the user actually typed.
  expect(canonicalGroupAddress("2307")).toBe("2307");
  expect(canonicalGroupAddress("not an address")).toBe("not an address");
});

// ---------------------------------------------------------------------
// Search and filter accept both notations while displaying slashes.
// ---------------------------------------------------------------------

it("matches an address in either notation whichever one is displayed", () => {
  expect(groupAddressSpellings("1/2/3")).toEqual(["1/2/3", "1.2.3"]);
  expect(groupAddressSpellings("2307")).toEqual(["2307"]);

  for (const needle of ["1/2", "1.2", "1/2/3", "1.2.3"]) {
    expect(groupAddressMatches("1/2/3", needle)).toBe(true);
  }
  expect(groupAddressMatches("1/2/3", "9/9")).toBe(false);
});

it("finds a group address in the search overlay's index through either notation", () => {
  const entries: SearchEntry[] = [
    { kind: "group_address", id: 1, label: "Ceiling light", address: "1/2/3" },
    // A device's address is an *individual* address: it has exactly one
    // spelling, and a slashed needle must not find it.
    { kind: "device", id: 2, label: "Example push button", address: "1.1.13" },
  ];
  expect(matchEntries(entries, "1.2.3").map((e) => e.id)).toEqual([1]);
  expect(matchEntries(entries, "1/2/3").map((e) => e.id)).toEqual([1]);
  expect(matchEntries(entries, "1.1.13").map((e) => e.id)).toEqual([2]);
  expect(matchEntries(entries, "1/1/13")).toEqual([]);
});

// ---------------------------------------------------------------------
// The table: the same data, two notations, and nothing else moved.
// ---------------------------------------------------------------------

const addresses: GroupAddressNode[] = [
  { id: 30, name: "Ceiling light", address: "1/2/3", range: null, dpts: ["DPST-1-1"], links: [] },
  { id: 31, name: "Free style", address: "2307", range: null, dpts: [], links: [] },
];

function installation(): InstallationNode {
  return {
    id: 0,
    name: "Example installation",
    topology: [],
    buildings: [],
    unassigned: [],
    group_addresses: addresses,
    group_ranges: [],
  };
}

function renderTable(): HTMLDivElement {
  return mount(
    <GroupAddressTable
      installation={installation()}
      selection={null}
      multiSelection={null}
      onItemClick={() => {}}
      onTreeUpdate={() => {}}
      rangeScope={null}
    />,
  );
}

// React tracks an input's value on the DOM node, so assigning `.value`
// directly is invisible to it — the prototype setter is what an actual
// keystroke goes through. Same helper `BusComposeForm.test.tsx` uses.
function typeInto(host: HTMLDivElement, selector: string, value: string): void {
  const input = host.querySelector<HTMLInputElement>(selector)!;
  Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!.call(
    input,
    value,
  );
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

function addressCells(host: HTMLDivElement): string[] {
  return [...host.querySelectorAll("td.ga-address button")].map((b) => b.textContent ?? "");
}

it("renders the table in slash notation without touching the data behind it", () => {
  const before = JSON.stringify(installation());

  let host = renderTable();
  expect(addressCells(host)).toEqual(["1/2/3", "2307"]);

  host = renderTable();
  expect(addressCells(host)).toEqual(["1/2/3", "2307"]);

  // The projection the server sent is a display input, never rewritten in
  // place — the closest seam the frontend has to "what gets persisted does
  // not change when the preference changes".
  expect(JSON.stringify(installation())).toBe(before);
  expect(addresses[0]!.address).toBe("1/2/3");
});

it("filters the table by either notation while slash notation is displayed", () => {
  const host = renderTable();

  for (const [needle, expected] of [
    ["1/2/3", ["1/2/3"]],
    ["1.2.3", ["1/2/3"]],
    ["1/2", ["1/2/3"]],
    ["9/9", []],
  ] as const) {
    act(() => typeInto(host, "input.address-filter", needle));
    expect(addressCells(host)).toEqual([...expected]);
  }
});

// ---------------------------------------------------------------------
// The invariant that matters: the preference never reaches the wire.
// ---------------------------------------------------------------------

async function sendFrom(destination: string): Promise<void> {
  apiMock.writeBusValue.mockResolvedValue({
    service: "GroupValueWrite",
    encodedPayload: "00",
    decodedEcho: { kind: "value", text: "off" },
  });
  const host = mount(
    <BusComposeForm
      destination={destination}
      resolution={{ kind: "single", dpt: "DPST-1-1" }}
      projectOpen
      sessionClosed={false}
      contextStale={false}
    />,
  );
  await act(async () => typeInto(host, "input.bus-compose-value", "off"));
  const send = [...host.querySelectorAll("button")].find((b) => !b.disabled)!;
  await act(async () => {
    send.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await Promise.resolve();
    await Promise.resolve();
  });
}

it("sends the canonical address for either accepted input spelling", async () => {
  for (const typed of ["1/2/3", "1.2.3"]) {
    await sendFrom(typed);
    expect(apiMock.writeBusValue).toHaveBeenLastCalledWith(
      "1/2/3",
      "DPST-1-1",
      "off",
      null,
    );
  }
});
