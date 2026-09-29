/** Tests for the group-address table's range, DPT, link and multi-select behaviour. */
// @vitest-environment happy-dom
import { act, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { GroupAddressLinkNode } from "./bindings/GroupAddressLinkNode";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { Selection } from "./selection";

const apiMock = vi.hoisted(() => ({
  unlinkComObject: vi.fn(),
  batchDeleteGroupAddresses: vi.fn(),
  batchDeleteDevices: vi.fn(),
  batchMoveDevicesToLine: vi.fn(),
  batchMoveDevicesToBuildingPart: vi.fn(),
}));

vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}));

import GroupAddressTable from "./GroupAddressTable";
import BulkActionToolbar from "./BulkActionToolbar";
import { useMultiSelection } from "./multiSelection";

let host: HTMLDivElement | undefined;

afterEach(() => {
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
});

function link(
  comObjectId: number,
  number: number,
  name: string | null,
  direction: string,
  device: { id: number; name: string | null; address: string | null },
): GroupAddressLinkNode {
  return {
    device_id: device.id,
    device_name: device.name,
    device_address: device.address,
    com_object_id: comObjectId,
    com_object_number: number,
    com_object_name: name,
    direction,
  };
}

const sensor = { id: 1, name: "Example push button", address: "1.1.13" };
const actuator = { id: 2, name: "Example actuator", address: "1.1.11" };

// One address per case the table has to render honestly: a resolved DPT
// with one sender and one receiver, a genuine DPT conflict between two
// linked objects, an address nothing links to, and a link whose device is
// missing from the project.
const addresses: GroupAddressNode[] = [
  {
    id: 30,
    name: "Ceiling light",
    address: "1/0/1",
    range: 21,
    dpts: ["DPST-1-1"],
    links: [
      link(10, 0, "Switch", "Send", sensor),
      link(11, 3, "Switch", "Receive", actuator),
    ],
  },
  {
    id: 31,
    name: "Disputed value",
    address: "1/0/2",
    range: 21,
    dpts: ["DPST-1-1", "DPST-5-1"],
    links: [link(12, 1, "Value", "Send", sensor), link(13, 2, "Value", "Receive", actuator)],
  },
  {
    id: 32,
    name: "Nothing links here",
    address: "1/0/3",
    range: null,
    dpts: [],
    links: [],
  },
  {
    id: 33,
    name: "Linked by a ghost",
    address: "1/0/4",
    range: 20,
    dpts: [],
    links: [link(14, 0, null, "Send", { id: 404, name: null, address: null })],
  },
];

function installation(): InstallationNode {
  return {
    id: 0,
    name: "Example installation",
    topology: [],
    buildings: [],
    unassigned: [],
    group_addresses: addresses,
    group_ranges: [
      { id: 20, name: "Lighting", start: "1/0/0", end: "1/7/255", parent: null },
      { id: 21, name: "Ground floor", start: "1/0/0", end: "1/0/255", parent: 20 },
    ],
  };
}

const tree: ProjectTree = {
  schema_version: 11,
  errors: 0,
  warnings: 0,
  can_undo: false,
  can_redo: false,
  is_modified: false,
  group_address_style: "ThreeLevel",
  installations: [installation()],
};

// The wiring `App` provides: one shared multi-selection feeding one
// `BulkActionToolbar`, and a plain click that moves the single `Selection`
// the inspector (and this table's link panel) reads.
function Harness(props: {
  inst?: InstallationNode;
  rangeScope?: number | null;
  onSelect?: (sel: Selection) => void;
  onTreeUpdate?: (next: ProjectTree) => void;
}) {
  const inst = props.inst ?? installation();
  const [selection, setSelection] = useState<Selection | null>(null);
  const { multiSelection, onItemClick, clear } = useMultiSelection({ ...tree, installations: [inst] }, (sel) => {
    setSelection(sel);
    props.onSelect?.(sel);
  });
  return (
    <>
      {multiSelection && multiSelection.ids.size > 0 && (
        <BulkActionToolbar
          multiSelection={multiSelection}
          tree={{ ...tree, installations: [inst] }}
          onTreeUpdate={props.onTreeUpdate ?? (() => {})}
          onDone={clear}
        />
      )}
      <GroupAddressTable
        installation={inst}
        selection={selection}
        multiSelection={multiSelection}
        onItemClick={onItemClick}
        onTreeUpdate={props.onTreeUpdate ?? (() => {})}
        rangeScope={props.rangeScope ?? null}
      />
    </>
  );
}

async function render(props: Parameters<typeof Harness>[0] = {}): Promise<Root> {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => root.render(<Harness {...props} />));
  return root;
}

function row(address: string): HTMLTableRowElement {
  const found = [...host!.querySelectorAll<HTMLTableRowElement>("tbody tr")].find((tr) =>
    tr.querySelector(".mono")?.textContent?.trim() === address,
  );
  if (!found) throw new Error(`no row for ${address}`);
  return found;
}

// React dedupes a plain `node.value = x`: the tracked value it compares
// against is updated by that assignment too, so the change event never
// reaches `onChange`. Same prototype-setter route `Search.test.tsx` uses.
function type(input: HTMLInputElement, value: string) {
  const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!;
  setter.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

async function click(el: HTMLElement, opts: { ctrlKey?: boolean; shiftKey?: boolean } = {}) {
  await act(async () => {
    el.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true, ...opts }));
  });
}

it("explains that Range groups addresses by containment, and routes its tip to group ranges", async () => {
  const root = await render();
  const rangeTip = host!.querySelector<HTMLButtonElement>('[data-help-topic="groupRanges"]');
  expect(rangeTip?.getAttribute("aria-label")).toBe("What a group address range means");
  expect(rangeTip?.getAttribute("aria-describedby")).toBeTruthy();
  expect(rangeTip!.parentElement!.querySelector('[role="tooltip"]')?.textContent).toMatch(/contain|hierarch/i);
  expect(rangeTip!.parentElement!.querySelector('[role="tooltip"]')?.textContent).toMatch(/not a datapoint range/i);
  await act(async () => root.unmount());
});

it("shows each address with its range path, resolved DPT and link directions", async () => {
  const root = await render();
  const cells = [...row("1/0/1").querySelectorAll("td")].map((td) => td.textContent?.trim());
  // checkbox, address, name, range, DPT, links
  expect(cells[2]).toBe("Ceiling light");
  expect(cells[3]).toBe("Lighting / Ground floor");
  expect(cells[4]).toBe("DPST-1-1");
  expect(cells[5]).toBe("1 sending · 1 receiving");
  const unlinked = [...row("1/0/3").querySelectorAll("td")].map((td) => td.textContent?.trim());
  expect(unlinked[3]).toBe("(no range)");
  expect(unlinked[4]).toBe("none stated");
  expect(unlinked[5]).toBe("none");
  await act(async () => root.unmount());
});

it("reports a DPT conflict between linked objects instead of settling it", async () => {
  const root = await render();
  const dpt = row("1/0/2").querySelectorAll("td")[4];
  expect(dpt?.textContent).toContain("DPST-1-1");
  expect(dpt?.textContent).toContain("DPST-5-1");
  expect(dpt?.textContent).toContain("conflicting");
  expect(dpt?.className).toContain("dpt-conflict");
  await act(async () => root.unmount());
});

it("drives the selection from a plain click and lists that address's links with direction", async () => {
  const onSelect = vi.fn();
  const root = await render({ onSelect });
  await click(row("1/0/1").querySelector<HTMLButtonElement>(".table-select")!);
  expect(onSelect).toHaveBeenCalledWith({ kind: "group_address", id: 30 });
  const panel = host!.querySelector(".address-links-panel")!;
  expect(panel.querySelector("h2")?.textContent).toBe("Links · 1/0/1");
  const linkRows = [...panel.querySelectorAll("tbody tr")].map((tr) =>
    [...tr.querySelectorAll("td")].slice(0, 3).map((td) => td.textContent?.trim()),
  );
  expect(linkRows[0]?.[0]).toContain("Example push button");
  expect(linkRows[0]?.[0]).toContain("1.1.13");
  expect(linkRows[0]?.[1]).toContain("Switch");
  expect(linkRows[0]?.[2]).toBe("Send");
  expect(linkRows[1]?.[2]).toBe("Receive");
  await act(async () => root.unmount());
});

it("still shows a link whose device is missing from the project", async () => {
  const root = await render();
  await click(row("1/0/4").querySelector<HTMLButtonElement>(".table-select")!);
  const panel = host!.querySelector(".address-links-panel")!;
  expect(panel.textContent).toContain("Unknown device #404");
  expect(panel.textContent).toContain("Unnamed object");
  await act(async () => root.unmount());
});

it("unlinks a communication object from the address it is selected on", async () => {
  const next = { ...tree };
  apiMock.unlinkComObject.mockResolvedValue(next);
  const onTreeUpdate = vi.fn();
  const root = await render({ onTreeUpdate });
  await click(row("1/0/1").querySelector<HTMLButtonElement>(".table-select")!);
  const unlink = host!.querySelector<HTMLButtonElement>(".address-links-panel tbody button")!;
  await act(async () => unlink.click());
  expect(apiMock.unlinkComObject).toHaveBeenCalledWith(10, 30, "Send");
  expect(onTreeUpdate).toHaveBeenCalledWith(next);
  await act(async () => root.unmount());
});

it("feeds the shared BulkActionToolbar from the row checkboxes", async () => {
  apiMock.batchDeleteGroupAddresses.mockResolvedValue(tree);
  const root = await render();
  const check = (address: string) =>
    row(address).querySelector<HTMLInputElement>('input[type="checkbox"]')!;
  await act(async () => check("1/0/1").click());
  expect(host!.textContent).toContain("1 group address selected");
  await act(async () => check("1/0/2").click());
  expect(host!.textContent).toContain("2 group addresses selected");
  // Toggling the same row back out leaves the other one selected.
  await act(async () => check("1/0/1").click());
  expect(host!.textContent).toContain("1 group address selected");
  const deleteButton = [...host!.querySelectorAll<HTMLButtonElement>(".bulk-action-toolbar button")]
    .find((b) => b.textContent === "Delete")!;
  await act(async () => deleteButton.click());
  expect(apiMock.batchDeleteGroupAddresses).toHaveBeenCalledWith([31]);
  await act(async () => root.unmount());
});

it("filters rows by address, name and DPT", async () => {
  const root = await render();
  const filter = host!.querySelector<HTMLInputElement>(".address-filter")!;
  await act(async () => type(filter, "1/0/"));
  expect(host!.querySelectorAll("tbody tr").length).toBe(4);
  await act(async () => type(filter, "ghost"));
  expect([...host!.querySelectorAll("tbody tr")].map((tr) => tr.querySelector(".mono")?.textContent))
    .toEqual(["1/0/4"]);
  await act(async () => type(filter, "DPST-1-1"));
  // Only the two addresses whose linked objects state DPST-1-1 remain.
  expect([...host!.querySelectorAll("tbody tr")].map((tr) => tr.querySelector(".mono")?.textContent))
    .toEqual(["1/0/1", "1/0/2"]);
  await act(async () => root.unmount());
});

it("spans a shift-click over the visible rows only, skipping one the view has hidden", async () => {
  // The point of passing the table's own row order into the shared handler:
  // scoped to range 20 the visible ids are 30, 31, 33 with 32 hidden between
  // them, so a span from the first row to the last must select three. The
  // full render order the handler falls back to would have swept up 32 as
  // well — which is how this test fails if the `visibleOrder` argument ever
  // goes missing from the call site.
  const root = await render({ rangeScope: 20 });
  await click(row("1/0/1").querySelector<HTMLButtonElement>(".table-select")!);
  await click(row("1/0/4").querySelector<HTMLButtonElement>(".table-select")!, { shiftKey: true });
  expect(host!.textContent).toContain("3 group addresses selected");
  const deleteButton = [...host!.querySelectorAll<HTMLButtonElement>(".bulk-action-toolbar button")]
    .find((b) => b.textContent === "Delete")!;
  await act(async () => deleteButton.click());
  // By id, not merely by count: the hidden 1/0/3 (id 32) is absent.
  expect(apiMock.batchDeleteGroupAddresses).toHaveBeenCalledWith([30, 31, 33]);
  await act(async () => root.unmount());
});

it("tells an empty filter result apart from a project with no group addresses", async () => {
  const root = await render();
  const filter = host!.querySelector<HTMLInputElement>(".address-filter")!;
  await act(async () => type(filter, "no such address"));
  expect(host!.querySelector('[role="status"]')?.textContent).toBe(
    "No group address matches this filter.",
  );
  await act(async () => root.unmount());

  const empty = { ...installation(), group_addresses: [] };
  const second = await render({ inst: empty });
  expect(host!.querySelector('[role="status"]')?.textContent).toContain("No entries yet");
  await act(async () => second.unmount());
});

it("restricts the rows to a scoped range and its nested ranges", async () => {
  const root = await render({ rangeScope: 20 });
  const visible = [...host!.querySelectorAll("tbody tr")].map((tr) =>
    tr.querySelector(".mono")?.textContent?.trim(),
  );
  // 1/0/1 and 1/0/2 sit in the nested "Ground floor" range, 1/0/4 directly
  // in "Lighting"; 1/0/3 has no range at all and is out of scope.
  expect(visible).toEqual(["1/0/1", "1/0/2", "1/0/4"]);
  await act(async () => root.unmount());
});
