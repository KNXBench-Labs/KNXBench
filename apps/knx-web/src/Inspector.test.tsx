/** Tests for Inspector's collapsed delete-restriction message on non-empty group ranges. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { GroupRangeNode } from "./bindings/GroupRangeNode";
import type { DeviceDetail } from "./bindings/DeviceDetail";
import type { Selection } from "./selection";

const apiMock = vi.hoisted(() => ({
  deviceParameters: vi.fn().mockResolvedValue({
    programId: null,
    sections: [],
    stale: [],
    diagnostics: [],
  }),
  moveDeviceToLine: vi.fn(),
  moveDeviceToBuildingPart: vi.fn(),
  setComObjectFlag: vi.fn(),
  linkComObject: vi.fn(),
  unlinkComObject: vi.fn(),
  setIndividualAddress: vi.fn(),
}));

vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}));

import Inspector from "./Inspector";
import { UI_LANGUAGE_STORAGE_KEY, resetUiLanguageForTests } from "./uiLanguage";
import { resetSettingsForTests, setSetting } from "./settingsStore";

let host: HTMLDivElement | undefined;
let root: Root | undefined;

afterEach(async () => {
  if (root) {
    await act(async () => root!.unmount());
    root = undefined;
  }
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
  resetSettingsForTests();
  resetUiLanguageForTests();
});

it.each([
  ["Stairway", "Treppenhaus"], ["RoomPart", "Raumteil"], ["Area", "Bereich"],
  ["Ground", "Grundstück"], ["Segment", "Segment"],
])("localizes the %s building kind in the Inspector", async (kind, label) => {
  setSetting(UI_LANGUAGE_STORAGE_KEY, "de");
  const tree = twoInstallationTree();
  tree.installations[0].buildings = [{ id: 501, name: "Test", kind, children: [], devices: [] }];
  await renderInspector({ kind: "building_part", id: 501 }, tree);
  expect(host!.querySelector(".inspector-description")?.textContent).toBe(label);
});

function ga(id: number, name: string, address: string): GroupAddressNode {
  return { id, name, address, range: null, dpts: [], links: [] };
}

function groupRange(id: number, name: string, start: string, end: string): GroupRangeNode {
  return { id, name, start, end, parent: null };
}

// Two installations: the second one carries the group address / group
// range under test, so `canDelete`/`canEdit` (Inspector's own
// `installations[0]`-only gate) come back false — the same shape
// `restrictedToFirstInstallationMessage`'s six original call sites all
// existed to report.
function twoInstallationTree(): ProjectTree {
  const first: InstallationNode = {
    id: 0,
    name: "Installation 1",
    topology: [],
    buildings: [],
    unassigned: [],
    group_addresses: [],
    group_ranges: [],
  };
  const second: InstallationNode = {
    id: 1,
    name: "Installation 2",
    topology: [],
    buildings: [],
    unassigned: [],
    group_addresses: [ga(201, "GA in second", "2/1/1")],
    group_ranges: [groupRange(301, "Range in second", "2/0/0", "2/7/255")],
  };
  return {
    schema_version: 11,
    errors: 0,
    warnings: 0,
    can_undo: false,
    can_redo: false,
    is_modified: false,
    group_address_style: "ThreeLevel",
    installations: [first, second],
  };
}

async function renderInspector(
  selection: Selection,
  tree: ProjectTree,
  deviceDetail: DeviceDetail | null = null,
  onApplied = vi.fn(),
) {
  host = document.createElement("div");
  document.body.appendChild(host);
  root = createRoot(host);
  await act(async () => {
    root!.render(
      <Inspector
        selection={selection}
        tree={tree}
        deviceDetail={deviceDetail}
        onApplied={onApplied}
        onDeleted={vi.fn()}
      />,
    );
  });
  return onApplied;
}

function setTextInputValue(input: HTMLInputElement, value: string) {
  const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!;
  setter.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

function deviceMoveTree(): ProjectTree {
  const device = {
    id: 42,
    name: "Device D",
    address: null,
    description: null,
    com_object_count: 0,
  };
  return {
    schema_version: 11,
    errors: 0,
    warnings: 0,
    can_undo: false,
    can_redo: false,
    is_modified: false,
    group_address_style: "ThreeLevel",
    installations: [{
      id: 1,
      name: "Installation",
      topology: [{
        id: 10,
        name: "Area A",
        address: 1,
        lines: [
          { id: 11, name: "Line A", address: 1, devices: [device] },
          { id: 12, name: "Line B", address: 2, devices: [] },
        ],
      }],
      buildings: [{ id: 501, name: "Room A", kind: "Room", children: [], devices: [] }],
      unassigned: [],
      group_addresses: [],
      group_ranges: [],
    }],
  };
}

function deviceDetail(): DeviceDetail {
  return {
    id: 42,
    name: "Device D",
    description: null,
    address: null,
    com_objects: [],
    product: { product_ref: null, program_ref: null, catalog: null, resolution: "NoReference" },
  };
}

function detailWithComObject(links: DeviceDetail["com_objects"][number]["links"] = []): DeviceDetail {
  return { ...deviceDetail(), com_objects: [{
    id: 7, number: 1, name: "Switch actuator output", dpt: "DPST-1-1", dpt_layer: null,
    description: null, description_layer: null, is_active: true,
    read: false, write: true, transmit: false, update: false,
    communication: true, read_on_init: false, links,
  }] };
}

describe("Inspector — collapsed delete-restriction message", () => {
  it("renders the 'Delete is only available for group addresses…' variant for a second-installation group address", async () => {
    const tree = twoInstallationTree();
    await renderInspector({ kind: "group_address", id: 201 }, tree);

    expect(host!.textContent).toContain(
      "Delete is only available for group addresses in the first installation.",
    );
    // The action clause is entity-specific text, not a raw discriminant —
    // no leftover template placeholder should ever reach the DOM.
    expect(host!.textContent).not.toContain("{action}");
    expect(host!.textContent).not.toContain("{entity}");
  });

  it("renders the 'Rename and Delete are only available for group ranges…' variant for a second-installation group range", async () => {
    const tree = twoInstallationTree();
    await renderInspector({ kind: "group_range", id: 301 }, tree);

    expect(host!.textContent).toContain(
      "Rename and Delete are only available for group ranges in the first installation.",
    );
  });
});

// KNOWN_LIMITATIONS.md §84 — a project's group address style, once chosen,
// used to be invisible again. This is display only: no button, dropdown, or
// route call lives in `ProjectInspector`, just `tree.group_address_style`
// read back onto the screen.
describe("Inspector — project node", () => {
  it("shows the project's current group address style", async () => {
    const tree = twoInstallationTree();
    await renderInspector({ kind: "project", id: 0 }, tree);

    expect(host!.textContent).toContain("ThreeLevel");
  });

  it("reflects a non-default style the same way", async () => {
    const tree = { ...twoInstallationTree(), group_address_style: "Free" };
    await renderInspector({ kind: "project", id: 0 }, tree);

    expect(host!.textContent).toContain("Free");
    expect(host!.textContent).not.toContain("ThreeLevel");
  });
});

describe("Inspector — structural move keyboard equivalent", () => {
  it("keeps both native selects wired to the same validated move commands", async () => {
    const nextTree = deviceMoveTree();
    apiMock.moveDeviceToLine.mockResolvedValueOnce(nextTree);
    apiMock.moveDeviceToBuildingPart.mockResolvedValueOnce(nextTree);
    const onApplied = vi.fn();
    await renderInspector(
      { kind: "device", id: 42 },
      deviceMoveTree(),
      deviceDetail(),
      onApplied,
    );
    const fields = Array.from(host!.querySelectorAll<HTMLLabelElement>("label.inspector-field"));
    const lineSelect = fields.find((field) => field.textContent?.startsWith("Line"))
      ?.querySelector("select");
    const buildingSelect = fields.find((field) => field.textContent?.startsWith("Building part"))
      ?.querySelector("select");
    expect(lineSelect).toBeTruthy();
    expect(buildingSelect).toBeTruthy();

    await act(async () => {
      lineSelect!.value = "12";
      lineSelect!.dispatchEvent(new Event("change", { bubbles: true }));
    });
    await act(async () => {
      buildingSelect!.value = "501";
      buildingSelect!.dispatchEvent(new Event("change", { bubbles: true }));
    });

    expect(apiMock.moveDeviceToLine).toHaveBeenCalledWith(42, 12);
    expect(apiMock.moveDeviceToBuildingPart).toHaveBeenCalledWith(42, 501);
    expect(onApplied).toHaveBeenCalledTimes(2);
    expect(onApplied).toHaveBeenCalledWith(nextTree);
  });
});

describe("Inspector — readable KNX communication flags (ISSUE-09)", () => {
  it.each([
    ["en", ["Read", "Write", "Transmit", "Update", "Communication", "Read on init"]],
    ["de", ["Lesen", "Schreiben", "Übertragen", "Aktualisieren", "Kommunikation", "Lesen bei Initialisierung"]],
  ])("shows the standard letters and the %s flag names without hiding them in hover tips", async (language, names) => {
    setSetting(UI_LANGUAGE_STORAGE_KEY, language);
    const nextTree = deviceMoveTree();
    apiMock.setComObjectFlag.mockResolvedValue(nextTree);
    await renderInspector({ kind: "device", id: 42 }, nextTree, detailWithComObject());
    const labels = Array.from(host!.querySelectorAll<HTMLElement>(".com-object-flags label"));
    expect(labels.map((label) => label.querySelector(".flag-code")?.textContent)).toEqual(["R", "W", "T", "U", "C", "I"]);
    expect(labels.map((label) => label.querySelector(".flag-name")?.textContent)).toEqual(names);
    await act(async () => labels[0].querySelector("input")!.click());
    expect(apiMock.setComObjectFlag).toHaveBeenCalledWith(7, "Read", true);
  });
});

describe("Inspector — atomic send-and-receive link action (ISSUE-09)", () => {
  it("offers Both as one request while keeping Send and Receive choices", async () => {
    const tree = deviceMoveTree();
    tree.installations[0].group_addresses = [ga(9, "Kitchen lights", "1/2/3")];
    apiMock.linkComObject.mockResolvedValue(tree);
    const onApplied = await renderInspector({ kind: "device", id: 42 }, tree, detailWithComObject());
    const row = host!.querySelector<HTMLElement>(".group-link-list .tree-new-row")!;
    const selects = row.querySelectorAll<HTMLSelectElement>("select");
    expect(Array.from(selects[1].options).map((option) => option.value)).toEqual(["Send", "Receive", "Both"]);
    await act(async () => {
      selects[0].value = "9";
      selects[0].dispatchEvent(new Event("change", { bubbles: true }));
      selects[1].value = "Both";
      selects[1].dispatchEvent(new Event("change", { bubbles: true }));
    });
    await act(async () => row.querySelector("button")!.click());
    expect(apiMock.linkComObject).toHaveBeenCalledTimes(1);
    expect(apiMock.linkComObject).toHaveBeenCalledWith(7, 9, "Both");
    expect(onApplied).toHaveBeenCalledWith(tree);
  });

  it("reports a refused paired link without refreshing or retrying a partial result", async () => {
    const tree = deviceMoveTree();
    tree.installations[0].group_addresses = [ga(9, "Kitchen lights", "1/2/3")];
    apiMock.linkComObject.mockRejectedValueOnce(new Error("Receive is already linked"));
    const onApplied = await renderInspector(
      { kind: "device", id: 42 },
      tree,
      detailWithComObject([{ ga_id: 9, address: "1/2/3", name: "Kitchen lights", direction: "Receive" }]),
    );
    const row = host!.querySelector<HTMLElement>(".group-link-list .tree-new-row")!;
    const selects = row.querySelectorAll<HTMLSelectElement>("select");
    await act(async () => {
      selects[0].value = "9";
      selects[0].dispatchEvent(new Event("change", { bubbles: true }));
      selects[1].value = "Both";
      selects[1].dispatchEvent(new Event("change", { bubbles: true }));
    });
    await act(async () => row.querySelector("button")!.click());
    expect(apiMock.linkComObject).toHaveBeenCalledTimes(1);
    expect(apiMock.linkComObject).toHaveBeenCalledWith(7, 9, "Both");
    expect(row.querySelector(".field-error")?.textContent).toContain("Receive is already linked");
    expect(host!.querySelectorAll(".group-link-row")).toHaveLength(1);
    expect(onApplied).not.toHaveBeenCalled();
  });

  it("offers one atomic unlink-both action beside separate directional unlink buttons", async () => {
    const tree = deviceMoveTree();
    tree.installations[0].group_addresses = [ga(9, "Kitchen lights", "1/2/3")];
    apiMock.unlinkComObject.mockResolvedValue(tree);
    const links = [
      { ga_id: 9, address: "1/2/3", name: "Kitchen lights", direction: "Send" },
      { ga_id: 9, address: "1/2/3", name: "Kitchen lights", direction: "Receive" },
    ];
    const onApplied = await renderInspector({ kind: "device", id: 42 }, tree, detailWithComObject(links));
    expect(host!.querySelectorAll(".group-link-row")).toHaveLength(2);
    const unlinkBoth = host!.querySelector<HTMLButtonElement>(".group-link-row .unlink-both")!;
    expect(unlinkBoth?.textContent).toBe("Unlink both");
    await act(async () => unlinkBoth.click());
    expect(apiMock.unlinkComObject).toHaveBeenCalledTimes(1);
    expect(apiMock.unlinkComObject).toHaveBeenCalledWith(7, 9, "Both");
    expect(onApplied).toHaveBeenCalledWith(tree);
    const single = host!.querySelector<HTMLButtonElement>(".group-link-row button:not(.unlink-both)")!;
    await act(async () => single.click());
    expect(apiMock.unlinkComObject).toHaveBeenLastCalledWith(7, 9, "Send");
  });
});

describe("Inspector — line-bound physical address (ISSUE-09)", () => {
  it("keeps area and line read-only while saving only an edited device octet as a full address", async () => {
    const tree = deviceMoveTree();
    const detail = { ...deviceDetail(), address: "1.1.12" };
    apiMock.setIndividualAddress.mockResolvedValue(tree);
    const onApplied = await renderInspector({ kind: "device", id: 42 }, tree, detail);
    const field = host!.querySelector<HTMLElement>(".individual-address-field")!;
    const prefix = field.querySelector<HTMLElement>(".address-prefix")!;
    expect(prefix.textContent).toBe("1.1.");
    const input = field.querySelector<HTMLInputElement>("input")!;
    expect(prefix.getAttribute("aria-hidden")).toBeNull();
    expect(input.getAttribute("aria-describedby")).toBe(prefix.id);
    expect(prefix.id).toBe("device-address-prefix-42");
    expect(input.value).toBe("12");
    await act(async () => setTextInputValue(input, "17"));
    await act(async () => {
      input.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
      await Promise.resolve();
    });
    expect(apiMock.setIndividualAddress).toHaveBeenCalledWith(42, "1.1.17");
    expect(onApplied).toHaveBeenCalledWith(tree);
  });

  it("rejects non-device numbers and coupler-only zero before contacting the API", async () => {
    const tree = deviceMoveTree();
    await renderInspector({ kind: "device", id: 42 }, tree, deviceDetail());
    const field = host!.querySelector<HTMLElement>(".individual-address-field")!;
    const input = field.querySelector<HTMLInputElement>("input")!;
    for (const bad of ["256", "0", "1.2", "-1"]) {
      await act(async () => setTextInputValue(input, bad));
      await act(async () => {
        input.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
        await Promise.resolve();
      });
      expect(field.querySelector(".field-error")?.textContent).toBeTruthy();
      expect(apiMock.setIndividualAddress).not.toHaveBeenCalled();
    }
  });

  it("preserves the full-address editor for unassigned devices, including clearing", async () => {
    const tree = deviceMoveTree();
    tree.installations[0].unassigned = tree.installations[0].topology[0].lines[0].devices.splice(0);
    apiMock.setIndividualAddress.mockResolvedValue(tree);
    const onApplied = await renderInspector({ kind: "device", id: 42 }, tree, { ...deviceDetail(), address: "2.3.7" });
    const field = host!.querySelector<HTMLElement>(".individual-address-field")!;
    expect(field.querySelector(".address-prefix")).toBeNull();
    const input = field.querySelector<HTMLInputElement>("input")!;
    expect(input.value).toBe("2.3.7");
    await act(async () => setTextInputValue(input, ""));
    await act(async () => {
      input.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
      await Promise.resolve();
    });
    expect(apiMock.setIndividualAddress).toHaveBeenCalledWith(42, null);
    expect(onApplied).toHaveBeenCalledWith(tree);
  });

  it("shows an imported mismatch without silently rewriting it on blur", async () => {
    const tree = deviceMoveTree();
    apiMock.setIndividualAddress.mockResolvedValue(tree);
    await renderInspector({ kind: "device", id: 42 }, tree, { ...deviceDetail(), address: "2.3.9" });
    const field = host!.querySelector<HTMLElement>(".individual-address-field")!;
    expect(field.textContent).toContain("2.3.9");
    const input = field.querySelector<HTMLInputElement>("input")!;
    await act(async () => input.dispatchEvent(new FocusEvent("focusout", { bubbles: true })));
    expect(apiMock.setIndividualAddress).not.toHaveBeenCalled();
    await act(async () => setTextInputValue(input, "17"));
    await act(async () => {
      input.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
      await Promise.resolve();
    });
    expect(apiMock.setIndividualAddress).toHaveBeenCalledWith(42, "1.1.17");
  });

  it("reports a duplicate address without applying or forgetting the original", async () => {
    const tree = deviceMoveTree();
    const onApplied = await renderInspector({ kind: "device", id: 42 }, tree, { ...deviceDetail(), address: "1.1.12" });
    apiMock.setIndividualAddress.mockRejectedValueOnce(new Error("individual address 1.1.17 already used"));
    const field = host!.querySelector<HTMLElement>(".individual-address-field")!;
    const input = field.querySelector<HTMLInputElement>("input")!;
    await act(async () => setTextInputValue(input, "17"));
    await act(async () => {
      input.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
      await Promise.resolve();
    });
    expect(apiMock.setIndividualAddress).toHaveBeenCalledWith(42, "1.1.17");
    expect(field.querySelector(".field-error")?.textContent).toContain("already used");
    expect(input.value).toBe("12");
    expect(onApplied).not.toHaveBeenCalled();
  });

  it("uses the line in the owning installation, not the first installation", async () => {
    const tree = twoInstallationTree();
    const area = deviceMoveTree().installations[0].topology[0];
    area.address = 2;
    area.lines[0].address = 3;
    tree.installations[1].topology = [area];
    apiMock.setIndividualAddress.mockResolvedValue(tree);
    await renderInspector({ kind: "device", id: 42 }, tree, { ...deviceDetail(), address: "2.3.9" });
    const field = host!.querySelector<HTMLElement>(".individual-address-field")!;
    expect(field.querySelector(".address-prefix")?.textContent).toBe("2.3.");
    const input = field.querySelector<HTMLInputElement>("input")!;
    await act(async () => setTextInputValue(input, "18"));
    await act(async () => {
      input.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
      await Promise.resolve();
    });
    expect(apiMock.setIndividualAddress).toHaveBeenCalledWith(42, "2.3.18");
  });

  it("disables the address editor when a device is both on a line and unassigned", async () => {
    const tree = deviceMoveTree();
    tree.installations[0].unassigned.push(tree.installations[0].topology[0].lines[0].devices[0]);
    await renderInspector({ kind: "device", id: 42 }, tree, deviceDetail());
    const field = host!.querySelector<HTMLElement>(".individual-address-field")!;
    expect(field.querySelector<HTMLInputElement>("input")!.disabled).toBe(true);
    expect(field.querySelector(".field-error")?.textContent).toContain("no unambiguous owning area and line");
    expect(apiMock.setIndividualAddress).not.toHaveBeenCalled();
  });

  it("disables the address editor for a repeated device reference in one line", async () => {
    const tree = deviceMoveTree();
    tree.installations[0].topology[0].lines[0].devices.push(tree.installations[0].topology[0].lines[0].devices[0]);
    await renderInspector({ kind: "device", id: 42 }, tree, deviceDetail());
    const field = host!.querySelector<HTMLElement>(".individual-address-field")!;
    expect(field.querySelector<HTMLInputElement>("input")!.disabled).toBe(true);
    expect(field.querySelector(".field-error")?.textContent).toContain("no unambiguous owning area and line");
    expect(apiMock.setIndividualAddress).not.toHaveBeenCalled();
  });

  it("refuses to guess when the tree lists a device on two lines", async () => {
    const tree = deviceMoveTree();
    tree.installations[0].topology[0].lines[1].devices.push(tree.installations[0].topology[0].lines[0].devices[0]);
    await renderInspector({ kind: "device", id: 42 }, tree, deviceDetail());
    const field = host!.querySelector<HTMLElement>(".individual-address-field")!;
    expect(field.querySelector<HTMLInputElement>("input")!.disabled).toBe(true);
    expect(field.querySelector(".field-error")?.textContent).toContain("no unambiguous owning area and line");
    expect(apiMock.setIndividualAddress).not.toHaveBeenCalled();
  });
});
