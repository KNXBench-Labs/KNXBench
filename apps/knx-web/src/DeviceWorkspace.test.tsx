// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";
import type { DeviceDetail } from "./bindings/DeviceDetail";
import type { DeviceProductCatalog } from "./bindings/DeviceProductCatalog";
import type { DeviceProductNode } from "./bindings/DeviceProductNode";
import type { ProjectTree } from "./bindings/ProjectTree";
const api = vi.hoisted(() => ({ deviceParameters: vi.fn().mockResolvedValue({ programId: null, sections: [], stale: [], diagnostics: [] }), setComObjectDpt: vi.fn() }));
vi.mock("./api", () => ({ ...api, errorMessage: String }));
import { DeviceWorkspace } from "./Inspector";

const tree: ProjectTree = { schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, installations: [] };

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

async function render(product: DeviceProductNode) {
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(<DeviceWorkspace detail={detail(product)} tree={tree} onApplied={() => {}} />));
  return { host, cleanup: async () => { await act(async () => root.unmount()); host.remove(); } };
}

it("keeps communication editing and parameters reachable in the central device tabs", async () => {
  const { host, cleanup } = await render(NO_REFERENCE);
  expect(host.textContent).toContain("Example");
  const tabs = [...host.querySelectorAll<HTMLButtonElement>('[role="tab"]')];
  expect(tabs).toHaveLength(2);
  await act(async () => tabs[1].click());
  expect(tabs[1].getAttribute("aria-selected")).toBe("true");
  expect(api.deviceParameters).toHaveBeenCalled();
  await act(async () => tabs[1].dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowLeft", bubbles: true })));
  expect(tabs[0].getAttribute("aria-selected")).toBe("true");
  await cleanup();
});

it("keeps the product identity readable from both tabs, above the tab strip", async () => {
  const { host, cleanup } = await render({ product_ref: "M-00FA_H-EX42-1_P-1", program_ref: "M-00FA_H-EX42-1_HP-1", catalog: catalog(), resolution: "Resolved" });
  const identity = host.querySelector<HTMLElement>(".device-identity")!;
  const tablist = host.querySelector('[role="tablist"]')!;
  // Position matters: the identity answers "what device is this", so it must
  // precede the tabs rather than hide inside one of them.
  expect(identity.compareDocumentPosition(tablist) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  const tabs = [...host.querySelectorAll<HTMLButtonElement>('[role="tab"]')];
  await act(async () => tabs[1].click());
  expect(host.querySelector(".device-identity")!.textContent).toContain("Example switch actuator");
  expect(host.querySelector('[role="tabpanel"]:not([hidden])')!.contains(identity)).toBe(false);
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
  expect(identity.textContent).not.toContain("Serial number");
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
