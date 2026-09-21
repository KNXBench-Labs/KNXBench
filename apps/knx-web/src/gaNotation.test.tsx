/** Tests the group-address notation preference: rendering, input, search, and exclusions. */
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
  GA_NOTATION_KEY,
  groupAddressLevels,
  groupAddressMatches,
  groupAddressSpellings,
  loadGaNotation,
  setGaNotation,
  useGaNotation,
} from "./gaNotation";
import { matchEntries } from "./searchMatch";
import type { SearchEntry } from "./treeUtils";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { InstallationNode } from "./bindings/InstallationNode";
import { resetSettingsForTests, settingsStorage } from "./settingsStore";

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

it("renders one address in both notations and leaves everything else alone", () => {
  expect(formatGroupAddress("1/2/3", "slash")).toBe("1/2/3");
  expect(formatGroupAddress("1/2/3", "dot")).toBe("1.2.3");
  expect(formatGroupAddress("4/100", "dot")).toBe("4.100");
  expect(formatGroupAddress("4/100", "slash")).toBe("4/100");

  // A `Free`-style address is a plain decimal with no separator at all.
  // Neither setting has anything to change about it.
  expect(formatGroupAddress("2307", "dot")).toBe("2307");
  expect(formatGroupAddress("2307", "slash")).toBe("2307");
  expect(groupAddressLevels("2307")).toBeNull();

  // Not a blind `replace`: a name, a path, a DPT id and a mixed-separator
  // string all come back untouched under either setting.
  for (const notation of ["slash", "dot"] as const) {
    expect(formatGroupAddress("Kitchen / Ceiling", notation)).toBe("Kitchen / Ceiling");
    expect(formatGroupAddress("/home/knx/projects/example.knxdb", notation)).toBe(
      "/home/knx/projects/example.knxdb",
    );
    expect(formatGroupAddress("DPST-1-1", notation)).toBe("DPST-1-1");
    expect(formatGroupAddress("1/2.3", notation)).toBe("1/2.3");
    expect(formatGroupAddress("1/2/3/4", notation)).toBe("1/2/3/4");
    expect(formatGroupAddress("", notation)).toBe("");
  }
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

it("keeps the preference across a reload and falls back for anything unrecognised", () => {
  expect(loadGaNotation(settingsStorage)).toBe("slash");

  setGaNotation("dot");
  expect(loadGaNotation(settingsStorage)).toBe("dot");

  // A reload is a fresh module state reading the same persisted record —
  // which is what `resetSettingsForTests()` plus the untouched
  // `localStorage` cache reproduces here.
  const cache = localStorage.getItem("knx-desktop:settings-cache");
  resetSettingsForTests();
  localStorage.setItem("knx-desktop:settings-cache", cache!);
  expect(loadGaNotation(settingsStorage)).toBe("dot");

  settingsStorage.setItem(GA_NOTATION_KEY, "semicolons");
  expect(loadGaNotation(settingsStorage)).toBe("slash");
});

it("re-renders every reader when the preference changes", () => {
  function Reader() {
    return <span className="probe">{formatGroupAddress("1/2/3", useGaNotation())}</span>;
  }
  const host = mount(<Reader />);
  expect(host.querySelector(".probe")!.textContent).toBe("1/2/3");
  act(() => setGaNotation("dot"));
  expect(host.querySelector(".probe")!.textContent).toBe("1.2.3");
});

// ---------------------------------------------------------------------
// Search and filter, in both notations, under either setting.
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

it("renders the table in the chosen notation without touching the data behind it", () => {
  const before = JSON.stringify(installation());

  let host = renderTable();
  expect(addressCells(host)).toEqual(["1/2/3", "2307"]);

  setGaNotation("dot");
  host = renderTable();
  expect(addressCells(host)).toEqual(["1.2.3", "2307"]);

  // The projection the server sent is a display input, never rewritten in
  // place — the closest seam the frontend has to "what gets persisted does
  // not change when the preference changes".
  expect(JSON.stringify(installation())).toBe(before);
  expect(addresses[0]!.address).toBe("1/2/3");
});

it("filters the table by either notation with the dotted one displayed", () => {
  setGaNotation("dot");
  const host = renderTable();

  for (const [needle, expected] of [
    ["1/2/3", ["1.2.3"]],
    ["1.2.3", ["1.2.3"]],
    ["1/2", ["1.2.3"]],
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

it("sends the canonical address whichever notation is selected or typed", async () => {
  for (const notation of ["slash", "dot"] as const) {
    for (const typed of ["1/2/3", "1.2.3"]) {
      setGaNotation(notation);
      await sendFrom(typed);
      expect(apiMock.writeBusValue).toHaveBeenLastCalledWith(
        "1/2/3",
        "DPST-1-1",
        "off",
        null,
      );
    }
  }
});
