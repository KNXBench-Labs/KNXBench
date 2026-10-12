/** Shared group-address workspace interaction and lifetime tests. */
// @vitest-environment happy-dom
import { act, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { Selection } from "./selection";
import { useMultiSelection } from "./multiSelection";
import GroupAddressTable from "./GroupAddressTable";
import { resetSettingsForTests } from "./settingsStore";

vi.mock("./api", () => ({
  unlinkComObject: vi.fn(), errorMessage: (e: unknown) => String(e),
}));
let host: HTMLDivElement;
let root: Root;
beforeEach(() => {
  Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
  window.localStorage.clear(); resetSettingsForTests();
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
});
afterEach(async () => { await act(async () => root.unmount()); host.remove(); });

const installation: InstallationNode = {
  id: 1, name: "Example", topology: [], buildings: [], unassigned: [],
  group_ranges: [
    { id: 20, parent: null, name: "Stored main", start: "0/0/0", end: "31/7/255" },
    { id: 21, parent: 20, name: "Stored child", start: "0/0/0", end: "31/7/255" },
  ],
  group_addresses: [
    { id: 30, name: "Alpha", address: "1/0/1", range: 21, dpts: ["DPST-1-1"], links: [] },
    { id: 31, name: "Beta", address: "1/0/2", range: 21, dpts: [], links: [] },
    { id: 32, name: "Unassigned", address: "1/0/3", range: null, dpts: [], links: [] },
  ],
};
function Harness(props: { input?: InstallationNode; context?: string }) {
  const input = props.input ?? installation;
  const [selection, setSelection] = useState<Selection | null>(null);
  const tree: ProjectTree = { schema_version: 1, errors: 0, warnings: 0, can_undo: false,
    can_redo: false, is_modified: false, group_address_style: "ThreeLevel", installations: [input] };
  const shared = useMultiSelection(tree, setSelection);
  return <><GroupAddressTable installation={input} tree={tree} selection={selection}
    multiSelection={shared.multiSelection} onItemClick={shared.onItemClick}
    onTreeUpdate={() => {}} rangeScope={null} projectContextKey={props.context} />
    <output data-selection>{selection?.id ?? "none"}</output>
    <output data-multi>{[...(shared.multiSelection?.ids ?? [])].join(",")}</output></>;
}
const click = async (element: Element, init: MouseEventInit = {}) => {
  await act(async () => element.dispatchEvent(new MouseEvent("click", { bubbles: true, ...init })));
};

it("collapses stored ranges without discarding selection or selecting hidden Shift rows", async () => {
  await act(async () => root.render(<Harness />));
  const main = host.querySelector<HTMLButtonElement>('[data-ga-expand="range:20:0"]');
  expect(main, "production workspace must disclose real stored ranges").not.toBeNull();
  const alpha = host.querySelector<HTMLButtonElement>('[data-ga-select="30"]')!;
  await click(alpha);
  expect(host.querySelector("[data-selection]")!.textContent).toBe("30");
  await click(main!);
  expect(host.querySelector('[data-ga-select="30"]')).toBeNull();
  await click(host.querySelector('[data-ga-select="32"]')!, { shiftKey: true });
  expect(host.querySelector("[data-multi]")!.textContent).toBe("32");
  expect(host.querySelector("[data-selection]")!.textContent).toBe("30");
});

async function chooseView(view: string) {
  const select = host.querySelector<HTMLSelectElement>("[data-ga-view]");
  expect(select, "production workspace must offer a presentation switch").not.toBeNull();
  await act(async () => { select!.value = view; select!.dispatchEvent(new Event("change", { bubbles: true })); });
}
async function search(query: string) {
  const input = host.querySelector<HTMLInputElement>(".address-filter")!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, query);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

it("browses actual range columns while keeping query, selection and scope on view switching", async () => {
  await act(async () => root.render(<Harness />));
  await click(host.querySelector('[data-ga-select="30"]')!);
  await search("Alpha");
  await chooseView("columns");
  expect(host.querySelector("[data-ga-columns]")).not.toBeNull();
  expect(host.querySelector<HTMLInputElement>(".address-filter")!.value).toBe("Alpha");
  await click(host.querySelector('[data-ga-scope="range:20:0"]')!);
  await click(host.querySelector('[data-ga-scope="range:21:0"]')!);
  expect(host.querySelectorAll("[data-ga-select]")).toHaveLength(1);
  await chooseView("tree");
  expect(host.querySelector("[data-ga-current-scope]")!.textContent).toContain("Stored child");
  expect(host.querySelector("[data-selection]")!.textContent).toBe("30");
  expect(host.querySelectorAll("[data-ga-select]")).toHaveLength(1);
});

it("drills through the semantic atlas and retains the same address selection in columns", async () => {
  await act(async () => root.render(<Harness />));
  await chooseView("atlas");
  expect(host.querySelector("[data-ga-atlas]"), "atlas must be mounted in the production workspace").not.toBeNull();
  await click(host.querySelector('[data-ga-scope="range:20:0"]')!);
  await click(host.querySelector('[data-ga-scope="range:21:0"]')!);
  expect([...host.querySelectorAll("[data-ga-select]")].map(button => button.getAttribute("data-ga-select"))).toEqual(["30", "31"]);
  await click(host.querySelector('[data-ga-select="31"]')!);
  expect(host.querySelector("[data-selection]")!.textContent).toBe("31");
  await chooseView("columns");
  expect(host.querySelector("[data-ga-current-scope]")!.textContent).toContain("Stored child");
  expect(host.querySelector('[data-ga-select="31"]')!.closest("tr")!.getAttribute("aria-selected")).toBe("true");
});

it("selects a matrix relationship with separate projected directions and explicit column scope", async () => {
  const link = { device_id: 1, device_name: "Sensor", device_address: "1.1.1", com_object_id: 10,
    com_object_name: "Switch", com_object_number: 0, direction: "Send" };
  const input: InstallationNode = { ...installation, group_addresses: [
    { ...installation.group_addresses[0], links: [link, { ...link, direction: "Receive" }, { ...link, direction: "Future" }] },
    installation.group_addresses[1],
  ] };
  await act(async () => root.render(<Harness input={input} />));
  await chooseView("matrix");
  expect(host.querySelector("[data-ga-matrix]"), "matrix must be mounted in the production workspace").not.toBeNull();
  const cell = host.querySelector<HTMLButtonElement>('[data-ga-cell="address:30:0|device:1:object:10"]')!;
  expect(cell.title).toContain("Send"); expect(cell.title).toContain("Receive"); expect(cell.title).toContain("Future");
  expect(cell.textContent).toContain("S"); expect(cell.textContent).toContain("R"); expect(cell.textContent).toContain("?");
  expect(host.querySelector('[data-ga-absence="address:31:0|device:1:object:10"]')!.textContent).toBe("—");
  expect(host.querySelector("[data-ga-matrix-scope]")!.textContent).toContain("Columns: communication objects with project links");
  await click(cell);
  expect(host.querySelector("[data-selection]")!.textContent).toBe("30");
  const column = host.querySelector<HTMLInputElement>('[data-ga-column="device:1:object:10"]')!;
  await act(async () => column.click());
  expect(host.querySelector("[data-ga-cell]")).toBeNull();
  expect(host.querySelectorAll("[data-ga-select]")).toHaveLength(2);
  await chooseView("tree");
  expect(host.querySelector('[data-ga-select="30"]')!.closest("tr")!.getAttribute("aria-selected")).toBe("true");
});

it("resets local navigation on project replacement but keeps the application view preference", async () => {
  await act(async () => root.render(<Harness context="server:project-a" />));
  await chooseView("columns");
  await click(host.querySelector('[data-ga-scope="range:20:0"]')!);
  await search("Beta");
  await act(async () => root.render(<Harness context="server:project-b" />));
  expect(host.querySelector<HTMLInputElement>(".address-filter")!.value, "a new project must not inherit the previous query").toBe("");
  expect(host.querySelector("[data-ga-current-scope]")!.textContent).toBe("All ranges");
  expect(host.querySelector<HTMLSelectElement>("[data-ga-view]")!.value).toBe("columns");
});

it("renders a visible bounded window in every view and extends it explicitly", async () => {
  const many: InstallationNode = { ...installation, group_ranges: [], group_addresses: Array.from({ length: 1200 }, (_, i) =>
    ({ id: 1000 + i, name: `Synthetic ${i}`, address: String(i), range: null, dpts: [], links: [] })) };
  await act(async () => root.render(<Harness input={many} />));
  for (const view of ["tree", "columns", "matrix"]) {
    await chooseView(view);
    expect(host.querySelectorAll("[data-ga-select]"), `${view} initial window`).toHaveLength(500);
    expect(host.querySelector("[data-ga-window]")!.textContent).toContain("500");
    expect(host.querySelector("[data-ga-window]")!.textContent).toContain("1200");
  }
  await chooseView("tree");
  await click(host.querySelector("[data-ga-more]")!);
  expect(host.querySelectorAll("[data-ga-select]")).toHaveLength(1000);
  await click(host.querySelector('[data-ga-select="1000"]')!);
  await click(host.querySelector('[data-ga-select="1999"]')!, { shiftKey: true });
  expect(host.querySelector("[data-multi]")!.textContent!.split(",")).toHaveLength(1000);
  await click(host.querySelector("[data-ga-all]")!);
  expect(host.querySelectorAll("[data-ga-select]")).toHaveLength(1200);
  expect(host.querySelector("[data-ga-window]")).toBeNull();
});

it("drops deleted ranges and addresses on update instead of keeping stale scope or rows", async () => {
  await act(async () => root.render(<Harness />));
  await chooseView("columns");
  await click(host.querySelector('[data-ga-scope="range:20:0"]')!);
  await click(host.querySelector('[data-ga-scope="range:21:0"]')!);
  expect(host.querySelector("[data-ga-current-scope]")!.textContent).toContain("Stored child");
  const updated: InstallationNode = { ...installation,
    group_ranges: installation.group_ranges.filter(range => range.id !== 21),
    group_addresses: installation.group_addresses.filter(ga => ga.id !== 31).map(ga => ga.id === 30 ? { ...ga, range: 20 } : ga) };
  await act(async () => root.render(<Harness input={updated} />));
  expect(host.querySelector("[data-ga-current-scope]")!.textContent).toBe("All ranges");
  expect(host.querySelector('[data-ga-scope="range:21:0"]')).toBeNull();
  expect(host.querySelector('[data-ga-select="31"]')).toBeNull();
  expect(host.querySelector('[data-ga-select="30"]')).not.toBeNull();
});
