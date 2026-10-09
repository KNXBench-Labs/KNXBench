/** Tests for the device workspace tabs, their keyboard navigation, and the product identity. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";
import type { DeviceDetail } from "./bindings/DeviceDetail";
import type { DeviceProductCatalog } from "./bindings/DeviceProductCatalog";
import type { DeviceProductNode } from "./bindings/DeviceProductNode";
import type { ProductResolution } from "./bindings/ProductResolution";
import type { ProjectTree } from "./bindings/ProjectTree";
import { messages as germanMessages } from "./messages/de";
import { messages as englishMessages } from "./messages/en";
const api = vi.hoisted(() => ({ deviceParameters: vi.fn().mockResolvedValue({ programId: null, sections: [], stale: [], diagnostics: [] }), setComObjectDpt: vi.fn(), setParameterValue: vi.fn() }));
vi.mock("./api", () => ({ ...api, errorMessage: String }));
import { DeviceWorkspace } from "./Inspector";

const tree: ProjectTree = { schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false, group_address_style: "ThreeLevel", installations: [] };

const NO_REFERENCE: DeviceProductNode = { product_ref: null, program_ref: null, catalog: null, resolution: "NoReference" };

function detail(product: DeviceProductNode = NO_REFERENCE, comObjects: DeviceDetail["com_objects"] = [], id = 9): DeviceDetail {
  return { id, name: "Example", address: null, description: null, com_objects: comObjects, product };
}

// Fictional refs and catalogue values throughout: this file is public, and a
// real ProductRefId would name a real product in a real installation.
function catalog(overrides: Partial<DeviceProductCatalog> = {}): DeviceProductCatalog {
  return {
    manufacturer_id: "M-00FA", manufacturer_name: "Example Manufacturing",
    product_text: "Example switch actuator", order_number: "EX-4210",
    hardware_name: "EX-HW-42", hardware_version: "1", hardware_serial_number: null,
    catalog_item_name: "Example switch actuator, 4-fold", catalog_item_number: "EX-4210-4",
    application_program_id: "M-00FA_A-1234-2-0000", application_name: "Switching 4f",
    application_number: "4660", application_version: "2", mask_version: "MV-0701",
    ...overrides,
  };
}

// Every identity test below works from the selected "Product data" tab, i.e.
// from what a user actually has on screen — a `hidden` panel still has
// `textContent`, so asserting against it would pass even if the tab were
// unreachable.
async function render(product: DeviceProductNode, selectProductTab = true, comObjects: DeviceDetail["com_objects"] = []) {
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(<DeviceWorkspace detail={detail(product, comObjects)} tree={tree} onApplied={() => {}} />));
  const tabs = [...host.querySelectorAll<HTMLButtonElement>('[role="tab"]')];
  if (selectProductTab) await act(async () => tabs[2].click());
  return {
    host, tabs,
    rerender: async (next: DeviceDetail) => { await act(async () => root.render(<DeviceWorkspace detail={next} tree={tree} onApplied={() => {}} />)); },
    cleanup: async () => { await act(async () => root.unmount()); host.remove(); },
  };
}

it("keeps communication editing and parameters reachable in the central device tabs", async () => {
  const { host, tabs, cleanup } = await render(NO_REFERENCE, false);
  expect(host.textContent).toContain("Example");
  expect(tabs.map((b) => b.textContent)).toEqual(["Communication objects", "Parameters", "Product data", "Diagnostics", "Manufacturer fields"]);
  await act(async () => tabs[1].click());
  expect(tabs[1].getAttribute("aria-selected")).toBe("true");
  expect(api.deviceParameters).toHaveBeenCalled();
  await act(async () => tabs[1].dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowLeft", bubbles: true })));
  expect(tabs[0].getAttribute("aria-selected")).toBe("true");
  await cleanup();
});

it("separates grouped diagnostics and restricted fields without refetching on tab switches", async () => {
  api.deviceParameters.mockClear();
  const field = (etsId: string, access: string | null) => ({
    etsId, name: etsId, text: null, nameLanguage: null, textLanguage: null,
    kind: "Number", value: "5", valueSource: "Stored", editable: access === null,
    writeEtsId: access === null ? etsId : null, access, min: null, max: null,
    enumOptions: [], displayOrder: null,
  });
  api.deviceParameters.mockResolvedValueOnce({
    programId: "EXAMPLE", sourceLanguage: null, tree: null,
    sections: [{ scope: null, fields: [field("Editable", null), field("Hidden", "None"), field("Read-only", "Read")] }],
    stale: [{ etsId: "Legacy", raw: "99" }],
    diagnostics: Array.from({ length: 6 }, (_, i) => ({
      scope: null, kind: "noBranchMatched", severity: "info",
      message: "A choice did not match any of its options.", detail: `Branch ${i}`,
    })),
  });
  const { host, tabs, cleanup } = await render(NO_REFERENCE, false);
  await act(async () => tabs[1].click());
  const visible = () => host.querySelector<HTMLElement>('[role="tabpanel"]:not([hidden])')!;
  expect(visible().textContent).toContain("Editable");
  expect(visible().textContent).not.toContain("A choice");
  expect(visible().textContent).not.toContain("Hidden");
  expect(visible().textContent).not.toContain("Legacy");
  await act(async () => tabs[3].click());
  expect(visible().querySelectorAll("li[data-severity]")).toHaveLength(1);
  expect(visible().textContent).toContain("6 occurrences");
  expect(visible().textContent).toContain("current controlling value");
  expect(visible().textContent).toContain("Legacy");
  await act(async () => tabs[4].click());
  expect([...visible().querySelectorAll(".parameter-field")].map((el) => el.getAttribute("data-ets-id")))
    .toEqual(["Hidden", "Read-only"]);
  expect(visible().textContent).toContain("Access None");
  expect(visible().textContent).toContain("Access Read");
  expect([...visible().querySelectorAll<HTMLInputElement>("input")].every((input) => input.disabled)).toBe(true);
  expect(api.deviceParameters).toHaveBeenCalledTimes(1);
  expect(api.setParameterValue).not.toHaveBeenCalled();
  await act(async () => tabs[4].dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true })));
  expect(tabs[0].getAttribute("aria-selected")).toBe("true");
  await cleanup();
});

type Issue08Object = DeviceDetail["com_objects"][number] & {
  activation: "Active" | "Inactive" | "Undetermined" | "NotEvaluated";
  channel: { key: string; kind: "Channel" | "ChannelIndependentBlock"; text: string | null; name: string | null; number: string | null; order: number } | null;
  is_active: boolean; program_dpt: string | null; dpt_text: string | null; function_text: string | null;
};

function object(id: number, overrides: Partial<Issue08Object> = {}): Issue08Object {
  return {
    id, number: id, name: `Object ${id}`, description: null, description_layer: null, dpt: null, dpt_layer: null,
    read: false, write: false, communication: true, transmit: false, update: false, read_on_init: false,
    links: [], activation: "NotEvaluated", channel: null, is_active: true,
    program_dpt: null, dpt_text: null, function_text: null, ...overrides,
  };
}

it("groups by opaque channel key in program order, collapsed by default and stable on refresh", async () => {
  const alpha = { key: "opaque-a", kind: "Channel" as const, text: "Inputs", name: "Raw A", number: "07", order: 2 };
  const another = { ...alpha, key: "opaque-a2", order: 3 };
  const beta = { ...alpha, key: "opaque-b", text: "Outputs", order: 8 };
  const objects = [
    object(4, { activation: "Active", channel: beta }),
    object(1, { activation: "Active", channel: alpha }),
    object(5, { activation: "NotEvaluated" }),
    object(2, { activation: "Active", channel: beta }),
    object(3, { activation: "Active", channel: another }),
  ];
  const { host, rerender, cleanup } = await render(NO_REFERENCE, false, objects);
  const groups = [...host.querySelectorAll<HTMLTableRowElement>("tr.com-object-channel")];
  expect(groups).toHaveLength(4);
  expect(groups.map((g) => g.querySelector("button")?.textContent)).toEqual([
    expect.stringContaining("Inputs"), expect.stringContaining("Inputs"),
    expect.stringContaining("Outputs"), expect.stringContaining("Without evaluated channel"),
  ]);
  expect(groups.map(g => host.querySelectorAll(`tr[data-object-id][data-channel-key="${g.dataset.channelKey}"]`).length)).toEqual([1, 1, 2, 1]);
  expect(groups.every((g) => g.querySelector("button")!.getAttribute("aria-expanded") === "false")).toBe(true);
  expect(groups[0].querySelector(".com-object-channel-name")?.textContent).toContain("Raw A");
  expect(groups[0].querySelector(".com-object-channel-number")?.textContent).toContain("07");
  await act(async () => { groups[0].querySelector("button")!.click(); });
  expect(groups[0].querySelector("button")!.getAttribute("aria-expanded")).toBe("true");
  await rerender(detail(NO_REFERENCE, [...objects], 9));
  expect(host.querySelectorAll<HTMLTableRowElement>("tr.com-object-channel")[0].querySelector("button")!.getAttribute("aria-expanded")).toBe("true");
  await rerender(detail(NO_REFERENCE, objects, 10));
  expect([...host.querySelectorAll<HTMLTableRowElement>("tr.com-object-channel")].every((g) => g.querySelector("button")!.getAttribute("aria-expanded") === "false")).toBe(true);
  await cleanup();
});

it("shows source channel name and opaque textual number beside translated text", async () => {
  const channel = { key: "opaque-owned", kind: "Channel" as const, text: "Lighting", name: "Raw manufacturer name", number: "A/05", order: 0 };
  const { host, cleanup } = await render(NO_REFERENCE, false, [object(1, { activation: "Active", channel })]);
  const summary = host.querySelector(".com-object-channel-summary")!;
  expect(summary.querySelector("strong")?.textContent).toBe("Lighting");
  expect(summary.querySelector(".com-object-channel-name")?.textContent).toContain("Raw manufacturer name");
  expect(summary.querySelector(".com-object-channel-number")?.textContent).toContain("A/05");
  expect(summary.textContent).not.toContain("opaque-owned");
  await cleanup();
});

it("uses the verbatim channel name without Text, and keeps numbers visible even without a name", async () => {
  const named = { key: "opaque-name", kind: "Channel" as const, text: null, name: "  Unchanged name  ", number: "07", order: 0 };
  const numbered = { key: "opaque-number", kind: "Channel" as const, text: null, name: null, number: "0", order: 1 };
  const empty = { key: "opaque-empty", kind: "Channel" as const, text: "", name: "", number: "", order: 2 };
  const independent = { key: "opaque-independent", kind: "ChannelIndependentBlock" as const, text: null, name: null, number: null, order: 3 };
  const { host, cleanup } = await render(NO_REFERENCE, false, [
    object(1, { activation: "Active", channel: named }),
    object(2, { activation: "Active", channel: numbered }),
    object(3, { activation: "Active", channel: empty }),
    object(4, { activation: "Active", channel: independent }),
    object(5, { activation: "NotEvaluated" }),
  ]);
  const groups = [...host.querySelectorAll(".com-object-channel-summary")];
  expect(groups[0].querySelector("strong")?.textContent).toBe("  Unchanged name  ");
  expect(groups[0].querySelector(".com-object-channel-number")?.textContent).toContain("07");
  expect(groups[1].querySelector("strong")?.textContent).toBe("Untitled channel");
  expect(groups[1].querySelector(".com-object-channel-number")?.textContent).toContain("0");
  expect(groups[2].querySelector("strong")?.textContent).toBe("Untitled channel");
  expect(groups[2].querySelector(".com-object-channel-number, .com-object-channel-name")).toBeNull();
  expect(groups[3].textContent).toContain("Channel-independent objects");
  expect(groups[4].textContent).toContain("Without evaluated channel");
  expect(groups.slice(3).every((group) => !group.querySelector(".com-object-channel-number, .com-object-channel-name"))).toBe(true);
  await cleanup();
});

it("keeps channel-independent blocks distinct from objects with no evaluated owner", async () => {
  const independent = { key: "opaque-independent", kind: "ChannelIndependentBlock" as const, text: null, name: null, number: null, order: 7 };
  const objects = [
    object(2, { activation: "NotEvaluated" }),
    object(1, { activation: "Active", channel: independent }),
    object(3, { activation: "Active", channel: null }),
    // A malformed or future server must not place an inactive object under
    // an owner that only an active evaluation can establish.
    object(4, { activation: "Inactive", channel: independent }),
    // An older or malformed wire response may omit channel even while
    // activation is Active; that must stay visible, not crash the editor.
    object(5, { activation: "Active", channel: undefined as unknown as null }),
  ];
  const { host, cleanup } = await render(NO_REFERENCE, false, objects);
  const groups = [...host.querySelectorAll<HTMLTableRowElement>("tr.com-object-channel")];
  expect(groups).toHaveLength(2);
  expect(groups[0].querySelector("button")?.textContent).toContain("Channel-independent objects");
  expect(groups[1].querySelector("button")?.textContent).toContain("Without evaluated channel");
  expect(groups.map(g => host.querySelectorAll(`tr[data-object-id][data-channel-key="${g.dataset.channelKey}"]`).length)).toEqual([1, 4]);
  expect(host.querySelector('tr[data-activation="Active"][data-channel-key="unassigned"]')).not.toBeNull();
  expect(host.querySelector('tr[data-activation="Inactive"][data-channel-key="unassigned"]')).not.toBeNull();
  await cleanup();
});

it("keeps inactive, undetermined and unevaluated objects inspectable but visibly distinct", async () => {
  const channel = { key: "opaque-1", kind: "Channel" as const, text: null, name: "Raw name", number: "4", order: 0 };
  const objects = [
    object(1, { activation: "Active", is_active: false, channel }),
    object(2, { activation: "Inactive", is_active: false }),
    object(3, { activation: "Undetermined" }),
    object(4, { activation: "NotEvaluated" }),
  ];
  const { host, cleanup } = await render(NO_REFERENCE, false, objects);
  const groups = [...host.querySelectorAll<HTMLTableRowElement>("tr.com-object-channel")];
  expect(groups[0].querySelector("button > strong")?.textContent).toBe("Raw name");
  expect(groups[0].querySelector(".com-object-channel-number")?.textContent).toContain("4");
  await act(async () => { for (const group of groups) group.querySelector("button")!.click(); });
  expect(host.querySelector('[data-activation="Inactive"]')?.textContent).toContain("Inactive");
  expect(host.querySelector('[data-activation="Undetermined"]')?.textContent).toContain("Undetermined");
  expect(host.querySelector('[data-activation="NotEvaluated"]')?.textContent).toContain("Not evaluated");
  expect(host.querySelector('[data-activation="Active"]')?.textContent).toContain("Active");
  expect(host.textContent).toContain("Stored inactive");
  await cleanup();
});

it("does not discard an activation state added by a newer server", async () => {
  const { host, cleanup } = await render(NO_REFERENCE, false, [
    object(6, { activation: "FutureActivation" as "NotEvaluated", channel: null }),
  ]);
  const group = host.querySelector<HTMLTableRowElement>("tr.com-object-channel")!;
  expect(group.querySelector("button")?.textContent).toContain("Without evaluated channel");
  await act(async () => group.querySelector("button")!.click());
  const row = host.querySelector('tr[data-activation="FutureActivation"]');
  expect(row?.textContent).toContain("Unknown activation state");
  await cleanup();
});

it("shows DPT provenance, canonical ids and function text without editing the program default", async () => {
  const channel = { key: "opaque-c", kind: "Channel" as const, text: "Lighting", name: null, number: null, order: 0 };
  const objects = [
    object(1, { activation: "Active", channel, dpt: "DPST-1-1", dpt_text: "Switch state", function_text: "Toggle relay" }),
    object(2, { activation: "Active", channel, program_dpt: "DPST-9-1", dpt_text: "Temperature" }),
    object(3, { activation: "Active", channel, dpt: "DPST-999-999" }),
    object(4, { activation: "Active", channel }),
  ];
  const { host, cleanup } = await render(NO_REFERENCE, false, objects);
  await act(async () => { host.querySelector<HTMLButtonElement>(".com-object-channel-summary")!.click(); });
  const rows = [...host.querySelectorAll<HTMLTableRowElement>("tr[data-object-id]")];
  expect(rows[0].textContent).toContain("Toggle relay");
  expect(rows[0].textContent).toContain("DPST-1-1");
  expect(rows[0].textContent).toContain("Switch state");
  expect(rows[1].textContent).toContain("Program default");
  expect(rows[1].textContent).toContain("DPST-9-1");
  expect(rows[1].textContent).toContain("Temperature");
  expect(rows[2].textContent).toContain("DPST-999-999");
  expect(rows[3].textContent).toContain("—");
  await act(async () => { rows[1].querySelector("summary")!.click(); });
  expect(host.querySelector<HTMLTableRowElement>('[data-editor-id="2"]')!.querySelector<HTMLInputElement>('input[placeholder="DPST-9-1"]')?.value).toBe("");
  const typed: DeviceDetail["com_objects"][number] = objects[0];
  expect(typed.activation).toBe("Active"); // TS must see the generated contract, not only this fixture.
  expect(typed.program_dpt).toBeNull();
  const defaultTyped: DeviceDetail["com_objects"][number] = objects[1];
  expect(defaultTyped.program_dpt).toBe("DPST-9-1");
  await cleanup();
});

// T3: closes the goal.md §6 item 6 parked finding. Before the fix,
// `ParameterPanel` had no way to tell its caller a write had landed at
// all, so `onApplied` was never called for this edit path — asserting
// merely that `api.setParameterValue` was called would have passed on the
// broken code too, since that call was never the missing half.
//
// T3 fix round 1, item 6: `onApplied` used to receive a hand-built
// `{...tree, can_undo: true, can_redo: false}` overlay of the local
// `tree` prop, because the write response carried no tree of its own.
// `serverTree` below differs from `tree` in fields an overlay could never
// touch (`errors`/`warnings`), so a passing assertion against it proves
// the server's own tree is what gets published, not a caller-side guess.
const serverTree: ProjectTree = { ...tree, errors: 5, warnings: 2, can_undo: true, can_redo: false, is_modified: true };

function committableField() {
  return {
    etsId: "P1", name: "Field A", text: null, kind: "Number", value: "5",
    valueSource: "Stored", editable: true, min: null, max: null, enumOptions: [],
    displayOrder: null, access: null, writeEtsId: "P1",
  };
}

function setNumberInputValue(input: HTMLInputElement, value: string) {
  const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!;
  setter.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

it("publishes a committed parameter edit to onApplied, not just to api.setParameterValue (T3)", async () => {
  api.deviceParameters.mockResolvedValueOnce({
    programId: "PROG-1",
    sections: [{ scope: null, fields: [committableField()] }],
    stale: [],
    diagnostics: [],
  });
  api.setParameterValue.mockResolvedValueOnce({
    programId: "PROG-1",
    sections: [{ scope: null, fields: [{ ...committableField(), value: "6" }] }],
    stale: [],
    diagnostics: [],
    tree: serverTree,
  });
  const onApplied = vi.fn();
  const host2 = document.createElement("div");
  document.body.append(host2);
  const root2 = createRoot(host2);
  await act(async () => {
    root2.render(<DeviceWorkspace detail={detail(NO_REFERENCE)} tree={tree} onApplied={onApplied} />);
  });
  const tabs2 = [...host2.querySelectorAll<HTMLButtonElement>('[role="tab"]')];
  await act(async () => tabs2[1].click());

  const input = host2.querySelector<HTMLInputElement>('input[type="number"]')!;
  await act(async () => {
    setNumberInputValue(input, "6");
    input.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
    await Promise.resolve();
  });

  // The call alone is not the fix — the hole was that `onApplied` never
  // ran, so `App.tsx`'s tree-publish effect never fired either.
  expect(api.setParameterValue).toHaveBeenCalledWith(9, "P1", "6", null);
  expect(onApplied).toHaveBeenCalledTimes(1);
  // `serverTree` differs from the local `tree` prop in fields no overlay
  // of `tree` could ever produce (`errors`/`warnings`) — this passes only
  // because `ParameterPanel` now forwards the write response's own `tree`
  // verbatim, not a `{...tree, can_undo, can_redo}` reconstruction of it.
  expect(onApplied).toHaveBeenCalledWith(serverTree);

  await act(async () => root2.unmount());
  host2.remove();
});

it("moves both ways with the arrow keys and reaches the last tab with End", async () => {
  // The two-tab version toggled with `1 - tab`, which moved on any arrow key
  // in the only direction it knew. With three tabs that is visible: these
  // assertions fail unless left and right are genuinely opposite.
  const { tabs, cleanup } = await render(NO_REFERENCE, false);
  const press = async (from: number, key: string) =>
    act(async () => tabs[from].dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true })));
  const selected = () => tabs.findIndex((b) => b.getAttribute("aria-selected") === "true");

  await press(0, "ArrowRight"); expect(selected()).toBe(1);
  await press(1, "ArrowRight"); expect(selected()).toBe(2);
  await press(2, "ArrowLeft"); expect(selected()).toBe(1);
  await press(1, "ArrowLeft"); expect(selected()).toBe(0);
  // Wrapping and End include both inspection tabs.
  await press(0, "ArrowLeft"); expect(selected()).toBe(4);
  await press(4, "ArrowRight"); expect(selected()).toBe(0);
  await press(0, "End"); expect(selected()).toBe(4);
  await press(2, "Home"); expect(selected()).toBe(0);
  await cleanup();
});

it("puts the product identity in its own tab panel, alongside the other two", async () => {
  const { host, tabs, cleanup } = await render({ product_ref: "M-00FA_H-EX42-1_P-1", program_ref: "M-00FA_H-EX42-1_HP-1", catalog: catalog(), resolution: "Resolved" }, false);
  const identity = host.querySelector<HTMLElement>(".device-identity")!;
  const panels = [...host.querySelectorAll<HTMLElement>('[role="tabpanel"]')];
  expect(panels).toHaveLength(5);
  expect(panels[2].contains(identity)).toBe(true);
  expect(panels[2].hidden).toBe(true);
  expect(tabs[2].getAttribute("aria-controls")).toBe(panels[2].id);

  await act(async () => tabs[2].click());
  expect(panels[2].hidden).toBe(false);
  expect(host.querySelector('[role="tabpanel"]:not([hidden])')).toBe(panels[2]);
  expect(identity.textContent).toContain("Example switch actuator");

  // Leaving the tab hides the identity but keeps the parameter panel mounted:
  // its fetch is keyed to the mount, so a tab switch must not refetch.
  const calls = api.deviceParameters.mock.calls.length;
  await act(async () => tabs[1].click());
  expect(panels[2].hidden).toBe(true);
  expect(api.deviceParameters.mock.calls.length).toBe(calls);
  await cleanup();
});

it("shows a resolved device's catalogue entry, with refs verbatim in monospace", async () => {
  const { host, cleanup } = await render({ product_ref: "M-00FA_H-EX42-1_P-1", program_ref: "M-00FA_H-EX42-1_HP-1", catalog: catalog(), resolution: "Resolved" });
  const identity = host.querySelector<HTMLElement>(".device-identity")!;
  const more = identity.querySelector<HTMLDetailsElement>("details.identity-more")!;
  // What a user sees without opening anything: the verdict and who made it.
  const headline = (node: Element) => [...node.children].filter((c) => c !== more).map((c) => c.textContent).join(" ");
  expect(headline(identity)).toContain("From the product database");
  expect(headline(identity)).toContain("Example Manufacturing");
  expect(headline(identity)).toContain("Example switch actuator");
  expect(headline(identity)).toContain("EX-4210");
  // Both refs are readable exactly as the project states them, in the mono face.
  const mono = [...identity.querySelectorAll(".mono")].map((n) => n.textContent);
  expect(mono).toContain("M-00FA_H-EX42-1_P-1");
  expect(mono).toContain("M-00FA_H-EX42-1_HP-1");
  // The long tail is one disclosure away, not gone: the summary names it, and
  // the hardware and program details live inside it.
  expect(more.open).toBe(false);
  expect(more.querySelector("summary")!.textContent).toBe("More product data");
  expect(more.textContent).toContain("EX-HW-42");
  expect(more.textContent).toContain("Switching 4f");
  expect(more.textContent).toContain("MV-0701");
  // One null field (the serial number) is omitted, and the omission is stated
  // rather than left for the user to notice.
  expect(more.textContent).toContain("1 further field is omitted");
  expect(identity.textContent).not.toContain("Hardware serial number");
  await cleanup();
});

it("names the missing serial and missing application data honestly on a partly installed catalogue", async () => {
  const partial = catalog({ application_name: null, application_number: null, application_version: null, application_program_id: null, mask_version: null });
  const { host, cleanup } = await render({ product_ref: "M-00FA_H-EX42-1_P-1", program_ref: null, catalog: partial, resolution: "Resolved" });
  const more = host.querySelector<HTMLElement>("details.identity-more")!;
  const identity = host.querySelector<HTMLElement>(".device-identity")!;
  expect(more.textContent).toContain("The product database holds no values here.");
  expect(more.textContent).toContain("6 further fields are omitted");
  // A ref the project never stated says so in prose — not blank, not "unknown",
  // and not in the monospace face reserved for real ref strings.
  const missing = [...identity.querySelectorAll("dd")].find((d) => d.textContent === "not stated in the project")!;
  expect(missing).toBeDefined();
  expect(missing.className).not.toContain("mono");
  expect(identity.textContent?.toLowerCase()).not.toContain("unknown");
  await cleanup();
});

it("distinguishes no product database from a database that lacks the product", async () => {
  const refs = { product_ref: "M-00FA_H-EX42-1_P-1", program_ref: "M-00FA_H-EX42-1_HP-1", catalog: null };
  const noDb = await render({ ...refs, resolution: "NoDatabase" });
  expect(noDb.host.textContent).toContain("No product database");
  expect(noDb.host.textContent).toContain("No product database is loaded here, so what the project states above cannot be matched to a product");
  // No catalogue is invented to fill the space.
  expect(noDb.host.querySelector("details.identity-more")).toBeNull();
  await noDb.cleanup();

  const notInDb = await render({ ...refs, resolution: "NotInDatabase" });
  expect(notInDb.host.textContent).toContain("Not in the product database");
  expect(notInDb.host.textContent).toContain("A product database is loaded and does not contain what the project states above");
  // The refs stay visible: they are what the user would go looking for.
  expect([...notInDb.host.querySelectorAll(".device-identity .mono")].map((n) => n.textContent))
    .toContain("M-00FA_H-EX42-1_P-1");
  expect(notInDb.host.querySelector("details.identity-more")).toBeNull();
  await notInDb.cleanup();
});

it("says a device states no product reference at all, without an empty ref list", async () => {
  const { host, cleanup } = await render(NO_REFERENCE);
  const identity = host.querySelector<HTMLElement>(".device-identity")!;
  expect(identity.textContent).toContain("No product reference");
  expect(identity.textContent).toContain("states neither a product reference nor an application program reference");
  // Nothing to print verbatim, so no ref rows at all — not two "not stated" ones.
  expect(identity.querySelector(".identity-row")).toBeNull();
  expect(identity.querySelector("details.identity-more")).toBeNull();
  await cleanup();
});

it("admits a match that carried no details rather than rendering a blank resolved device", async () => {
  // The server never sends this, but `DeviceProductNode` permits it, and a
  // silent empty panel would read as "resolved, and this device has nothing".
  const { host, cleanup } = await render({ product_ref: "M-00FA_H-EX42-1_P-1", program_ref: null, catalog: null, resolution: "Resolved" });
  expect(host.textContent).toContain("reported a match but returned no details");
  expect(host.querySelector("details.identity-more")).toBeNull();
  await cleanup();
});

it("names the hardware serial as the hardware's, never as this unit's", async () => {
  // `hardware_serial_number` comes from the manufacturer package and describes
  // the hardware type. The serial of the unit on the wall cannot be read at
  // all (KNOWN_LIMITATIONS.md §73), so the label must not claim to be it.
  const { host, cleanup } = await render({ product_ref: "M-00FA_H-EX42-1_P-1", program_ref: "M-00FA_H-EX42-1_HP-1", catalog: catalog({ hardware_serial_number: "EXHW-SER-1" }), resolution: "Resolved" });
  const labels = [...host.querySelectorAll(".device-identity dt")].map((n) => n.textContent);
  expect(labels).toContain("Hardware serial number");
  expect(labels).not.toContain("Serial number");
  await cleanup();
});

it("says so instead of guessing when the server reports a state this build does not know", async () => {
  // A frontend older than its server. Borrowing `NoReference`'s wording here
  // would print "No product reference" directly above two references the user
  // can read — the one failure mode this panel exists to prevent.
  const unknown = { product_ref: "M-00FA_H-EX42-1_P-1", program_ref: "M-00FA_H-EX42-1_HP-1", catalog: null, resolution: "SomethingNewerThanThisBuild" as unknown as ProductResolution };
  const { host, cleanup } = await render(unknown);
  const identity = host.querySelector<HTMLElement>(".device-identity")!;
  expect(identity.textContent).toContain("State not recognised");
  expect(identity.textContent).toContain("does not recognise");
  expect(identity.textContent).not.toContain("No product reference");
  // The refs are still printed, because they are still facts.
  expect([...identity.querySelectorAll(".mono")].map((n) => n.textContent))
    .toEqual(["M-00FA_H-EX42-1_P-1", "M-00FA_H-EX42-1_HP-1"]);
  expect(identity.querySelector(".resolution-badge")!.getAttribute("data-resolution")).toBe("SomethingNewerThanThisBuild");
  await cleanup();
});

it("counts omitted catalogue fields without assuming two ref rows are present", async () => {
  // `NoReference` empties the ref rows; a catalogue arriving anyway (the
  // server does not pair them, but the type allows it) used to make the
  // omission count two too high and drop three fields from the headline.
  const { host, cleanup } = await render({ product_ref: null, program_ref: null, catalog: catalog({ hardware_serial_number: "EXHW-SER-1" }), resolution: "NoReference" });
  const identity = host.querySelector<HTMLElement>(".device-identity")!;
  expect(identity.textContent).toContain("Example Manufacturing");
  expect(identity.textContent).toContain("EX-4210");
  // All fourteen catalogue fields have values, so nothing is omitted and no
  // count is printed at all.
  expect(identity.textContent).not.toContain("further field");
  await cleanup();
});

it("gives every tab panel its own tab stop, so a panel with nothing focusable stays reachable", async () => {
  // Under `NoReference` the product panel is a heading, a badge and one
  // sentence — no link, no control, nothing focusable. Without a tab stop of
  // its own, a keyboard user leaving the tablist would land past the whole
  // panel and never reach what the tab was for. Applied to all three panels
  // because which of them ends up empty is a property of the project, not of
  // this component.
  const { host, tabs, cleanup } = await render(NO_REFERENCE);
  const panels = [...host.querySelectorAll<HTMLElement>('[role="tabpanel"]')];
  expect(panels.map((p) => p.tabIndex)).toEqual([0, 0, 0, 0, 0]);
  expect(panels[2].querySelectorAll('a[href], button, input, select, textarea, summary, [tabindex]')).toHaveLength(0);
  panels[2].focus();
  expect(document.activeElement).toBe(panels[2]);
  // The tablist itself still has exactly one stop, the selected tab.
  expect(tabs.map((b) => b.tabIndex)).toEqual([-1, -1, 0, -1, -1]);
  await cleanup();
});

it("never tells a German reader the state is simply unknown, which the English also avoids", () => {
  // The English guard lives in the rendered assertions above; its German twin
  // has to be a catalogue assertion, because `DeviceWorkspace` renders English
  // in these tests and seeding the UI language would test the language store
  // rather than the wording. Scope is the whole `deviceIdentity.*` namespace,
  // not the one key that once read "Zustand unbekannt", so the claim cannot
  // creep back in through a neighbour. The server knows the state perfectly
  // well; only this build does not.
  const identityKeys = Object.entries(germanMessages).filter(([key]) => key.startsWith("deviceIdentity."));
  // Parity with English rather than a hardcoded count: a new key should not
  // fail this test for a reason that has nothing to do with the wording.
  expect(identityKeys.map(([key]) => key).sort()).toEqual(
    Object.keys(englishMessages)
      .filter((key) => key.startsWith("deviceIdentity."))
      .sort(),
  );
  expect(identityKeys.length).toBeGreaterThan(0);
  for (const [key, value] of identityKeys) {
    expect(value.toLowerCase(), key).not.toContain("unbekannt");
  }
  expect(germanMessages["deviceIdentity.resolution.unrecognised"]).toBe("Zustand nicht erkannt");
});

// MODEL-01 / ADR-0070: a link never connects two installations, so a device
// is only offered the group addresses of the installation that places it.
it("offers a later-installation device only that installation's group addresses to link", async () => {
  const ga = (id: number, name: string, address: string) => ({ id, name, address, range: null, dpts: [], links: [] });
  const device = { id: 9, name: "Example", address: null, description: null, com_object_count: 1 };
  const installation = (id: number, name: string) => ({ id, name, topology: [], buildings: [], unassigned: [],
    group_addresses: [], group_ranges: [] });
  const twoInstallations: ProjectTree = { ...tree, installations: [
    { ...installation(1, "First"), group_addresses: [ga(70, "First light", "1/1/1")] },
    { ...installation(2, "Second"), group_addresses: [ga(80, "Second light", "2/1/1")],
      topology: [{ id: 5, name: "Area", address: 2, lines: [{ id: 6, name: "Line", address: 1, devices: [device] }] }] },
  ] };
  const com = {
    id: 7, number: 1, name: "Switch", dpt: "DPST-1-1", dpt_layer: null, description: null, description_layer: null,
    is_active: true, activation: "NotEvaluated", channel: null, program_dpt: null, dpt_text: null, function_text: null,
    read: false, write: true, transmit: false, update: false, communication: true, read_on_init: false, links: [],
  } as unknown as DeviceDetail["com_objects"][number];
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(<DeviceWorkspace detail={detail(NO_REFERENCE, [com])} tree={twoInstallations}
    onApplied={() => {}} />));
  await act(async () => host.querySelector<HTMLElement>(".com-object-detail summary")!.click());
  const values = Array.from(host.querySelectorAll<HTMLOptionElement>(".group-link-list select option"), (option) => option.value);
  expect(values).toContain("80");
  expect(values).not.toContain("70");
  await act(async () => root.unmount()); host.remove();
});
