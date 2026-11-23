/** Runs the tabs checklist (WP-49.4a) against the real Inspector device tablist. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { expect, it, vi } from "vitest";
import type { DeviceDetail } from "./bindings/DeviceDetail";
import type { ProjectTree } from "./bindings/ProjectTree";
import { DeviceWorkspace } from "./deviceComparisons.fixture";
import { describeTabsChecklist, type TabsAdapter } from "./test/tabsChecklist";

const api = vi.hoisted(() => ({ deviceParameters: vi.fn().mockResolvedValue({ programId: null, sections: [], stale: [], diagnostics: [] }), setComObjectDpt: vi.fn(), setParameterValue: vi.fn() }));
vi.mock("./api", () => ({ ...api, errorMessage: String }));
vi.mock("./features/rtf/client", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./features/rtf/client")>()),
  createInstructionClient: () => ({ read: vi.fn(() => new Promise(() => {})), preview: vi.fn(), apply: vi.fn(), cancel: vi.fn().mockResolvedValue(undefined) }),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const tree: ProjectTree = { schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false, group_address_style: "ThreeLevel", installations: [] };
const detail: DeviceDetail = { id: 9, name: "Example", address: null, description: null, com_objects: [], product: { product_ref: null, program_ref: null, catalog: null, resolution: "NoReference" } };

let host: HTMLDivElement | undefined;
let root: Root | undefined;

/** The real workspace inside a parent that reports every keydown reaching it. */
const adapter: TabsAdapter = {
  name: "Inspector device tabs",
  async mount(parentKeyDown) {
    host = document.createElement("div");
    document.body.append(host);
    root = createRoot(host);
    await act(async () => root!.render(<div onKeyDown={parentKeyDown}><DeviceWorkspace detail={detail} tree={tree} onApplied={() => {}} /></div>));
  },
  async unmount() {
    await act(async () => root!.unmount());
    host!.remove();
    host = undefined; root = undefined;
  },
  tablist: () => host!.querySelector<HTMLElement>('[role="tablist"]')!,
  tabs: () => [...host!.querySelectorAll<HTMLElement>('[role="tab"]')],
  panels: () => [...host!.querySelectorAll<HTMLElement>('[role="tabpanel"]')],
  controls: "all",
  homeEnd: "present",
  panelStop: "tab-stop",
};

describeTabsChecklist(adapter);

it("Inspector device tabs: the tablist is named after the device and has one panel per tab", async () => {
  await adapter.mount(() => {});
  try {
    expect(adapter.tablist().getAttribute("aria-label")).toBe("Example");
    expect(adapter.tabs()).toHaveLength(7);
    expect(adapter.panels()).toHaveLength(7);
  } finally { await adapter.unmount(); }
});

it("Inspector device tabs: the last (configuration) tab's aria-controls reaches its panel, not an orphan id", async () => {
  await adapter.mount(() => {});
  try {
    const last = adapter.tabs().at(-1)!;
    expect(last.getAttribute("aria-controls")).toBe("device-panel-9-6");
    const panel = document.getElementById("device-panel-9-6")!;
    expect(panel.getAttribute("aria-labelledby")).toBe(last.id);
    expect(panel.hidden).toBe(true);
  } finally { await adapter.unmount(); }
});
