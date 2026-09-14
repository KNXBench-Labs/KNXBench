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

const tree: ProjectTree = { schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, group_address_style: "ThreeLevel", installations: [] };

const NO_REFERENCE: DeviceProductNode = { product_ref: null, program_ref: null, catalog: null, resolution: "NoReference" };

function detail(product: DeviceProductNode = NO_REFERENCE): DeviceDetail {
  return { id: 9, name: "Example", address: null, description: null, com_objects: [], product };
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
async function render(product: DeviceProductNode, selectProductTab = true) {
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(<DeviceWorkspace detail={detail(product)} tree={tree} onApplied={() => {}} />));
  const tabs = [...host.querySelectorAll<HTMLButtonElement>('[role="tab"]')];
  if (selectProductTab) await act(async () => tabs[2].click());
  return { host, tabs, cleanup: async () => { await act(async () => root.unmount()); host.remove(); } };
}

it("keeps communication editing and parameters reachable in the central device tabs", async () => {
  const { host, tabs, cleanup } = await render(NO_REFERENCE, false);
  expect(host.textContent).toContain("Example");
  expect(tabs.map((b) => b.textContent)).toEqual(["Communication objects", "Parameters", "Product data"]);
  await act(async () => tabs[1].click());
  expect(tabs[1].getAttribute("aria-selected")).toBe("true");
  expect(api.deviceParameters).toHaveBeenCalled();
  await act(async () => tabs[1].dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowLeft", bubbles: true })));
  expect(tabs[0].getAttribute("aria-selected")).toBe("true");
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
const serverTree: ProjectTree = { ...tree, errors: 5, warnings: 2, can_undo: true, can_redo: false };

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
  // Wrapping, in both directions, over three tabs rather than two.
  await press(0, "ArrowLeft"); expect(selected()).toBe(2);
  await press(2, "ArrowRight"); expect(selected()).toBe(0);
  await press(0, "End"); expect(selected()).toBe(2);
  await press(2, "Home"); expect(selected()).toBe(0);
  await cleanup();
});

it("puts the product identity in its own tab panel, alongside the other two", async () => {
  const { host, tabs, cleanup } = await render({ product_ref: "M-00FA_H-EX42-1_P-1", program_ref: "M-00FA_H-EX42-1_HP-1", catalog: catalog(), resolution: "Resolved" }, false);
  const identity = host.querySelector<HTMLElement>(".device-identity")!;
  const panels = [...host.querySelectorAll<HTMLElement>('[role="tabpanel"]')];
  expect(panels).toHaveLength(3);
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
  expect(panels.map((p) => p.tabIndex)).toEqual([0, 0, 0]);
  expect(panels[2].querySelectorAll('a[href], button, input, select, textarea, summary, [tabindex]')).toHaveLength(0);
  panels[2].focus();
  expect(document.activeElement).toBe(panels[2]);
  // The tablist itself still has exactly one stop, the selected tab.
  expect(tabs.map((b) => b.tabIndex)).toEqual([-1, -1, 0]);
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
