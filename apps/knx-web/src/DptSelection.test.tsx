/** Localized DPT selection and field provenance without imported-data rewrites. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import type { ComObjectNode } from "./bindings/ComObjectNode";
import type { DeviceDetail } from "./bindings/DeviceDetail";
import type { ProjectTree } from "./bindings/ProjectTree";
import { resetUiLanguageForTests } from "./uiLanguage";
import { settingsStorage } from "./settingsStore";
function setUiLanguage(language: string) { settingsStorage.setItem("uiLanguage", language); resetUiLanguageForTests(); }
const api = vi.hoisted(() => ({ deviceParameters: vi.fn().mockResolvedValue({ programId: null, sections: [], stale: [], diagnostics: [] }), setComObjectDpt: vi.fn() }));
vi.mock("./api", () => ({ ...api, errorMessage: String }));
import { DeviceWorkspace } from "./Inspector";
const tree: ProjectTree = { schema_version: 12, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false, group_address_style: "ThreeLevel", installations: [] };
const object = (overrides: Partial<ComObjectNode> = {}): ComObjectNode => ({ id: 1, number: 1, name: "Example", description: null, description_layer: null, dpt: "DPST-9-1", dpt_layer: "Program", program_dpt: null, dpt_text: null, function_text: null, is_active: true, activation: "NotEvaluated", channel: null, read: false, write: false, communication: true, transmit: false, update: false, read_on_init: false, links: [], ...overrides });
const detail = (com: ComObjectNode): DeviceDetail => ({ id: 9, name: "Example device", address: null, description: null, com_objects: [com], product: { product_ref: null, program_ref: null, catalog: null, resolution: "NoReference" } });
afterEach(() => { setUiLanguage("en"); vi.clearAllMocks(); });
async function render(com: ComObjectNode) {
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host); const onApplied = vi.fn();
  await act(async () => root.render(<DeviceWorkspace detail={detail(com)} tree={tree} onApplied={onApplied} />));
  await act(async () => host.querySelector<HTMLButtonElement>(".com-object-channel-summary")!.click());
  await act(async () => host.querySelector<HTMLElement>(".com-object-detail > summary")!.click());
  return { host, onApplied, cleanup: async () => { await act(async () => root.unmount()); host.remove(); } };
}
for (const [language, temperature, scene, source] of [["en", "Temperature", "Scene number", "Source: application program"], ["de", "Temperatur", "Szenennummer", "Quelle: Applikationsprogramm"]] as const) {
  it(`offers numbered ${language} names without a free-text DPT input or database`, async () => {
    setUiLanguage(language); const v = await render(object());
    try {
      const field = v.host.querySelector<HTMLElement>(".dpt-field")!;
      expect(field, "DPT has its own field-local context").not.toBeNull();
      expect(field.querySelector("input")).toBeNull();
      expect(v.host.querySelector(".com-object-effective-dpt")?.textContent).not.toContain("DPST-");
      expect(field.querySelector<HTMLDetailsElement>(".dpt-reference")?.open).toBe(false);
      const select = field.querySelector<HTMLSelectElement>("select")!;
      expect(select).not.toBeNull(); expect(select.value).toBe("DPST-9-1");
      expect(select.querySelector('[value="DPST-9-1"]')?.textContent).toBe(`${temperature} (9.001)`);
      expect(select.querySelector('[value="DPST-17-1"]')?.textContent).toBe(`${scene} (17.001)`);
      expect(field.querySelector(".provenance-badge")?.textContent).toBe(source);
      expect(field.querySelector(".provenance-badge")?.getAttribute("title")).toBeTruthy();
      expect(v.host.querySelector(".com-object-effective-dpt")?.textContent).toContain(`${temperature} (9.001)`);
      expect(api.setComObjectDpt).not.toHaveBeenCalled();
    } finally { await v.cleanup(); }
  });
}
it("preserves unknown imported identifiers until an explicit dropdown change", async () => {
  const v = await render(object({ dpt: "vendor:opaque", dpt_layer: "Instance" }));
  try {
    const select = v.host.querySelector<HTMLSelectElement>(".dpt-field select")!;
    expect(select).not.toBeNull(); expect(select.value).toBe("vendor:opaque");
    expect(select.selectedOptions[0].textContent).toContain("vendor:opaque");
    await act(async () => select.dispatchEvent(new FocusEvent("focusout", { bubbles: true })));
    expect(api.setComObjectDpt).not.toHaveBeenCalled();
    api.setComObjectDpt.mockResolvedValue(tree);
    await act(async () => { select.value = "DPST-17-1"; select.dispatchEvent(new Event("change", { bubbles: true })); });
    await act(async () => select.dispatchEvent(new FocusEvent("focusout", { bubbles: true })));
    expect(api.setComObjectDpt).toHaveBeenCalledExactlyOnceWith(1, "DPST-17-1");
  } finally { await v.cleanup(); }
});
it("keeps a program default display-only and offers explicit reset to that default", async () => {
  const v = await render(object({ dpt: null, dpt_layer: null, program_dpt: "DPST-9-1" }));
  try {
    const select = v.host.querySelector<HTMLSelectElement>(".dpt-field select")!;
    expect(select).not.toBeNull(); expect(select.value).toBe("");
    expect(select.selectedOptions[0].textContent).toContain("Temperature (9.001)");
    await act(async () => select.dispatchEvent(new FocusEvent("focusout", { bubbles: true })));
    expect(api.setComObjectDpt).not.toHaveBeenCalled();
  } finally { await v.cleanup(); }
});
it("binds description provenance to the description rather than a stray flex item", async () => {
  const v = await render(object({ description: "Original", description_layer: "UserEdit" }));
  try {
    const description = [...v.host.querySelectorAll(".inspector-field")].find(field => field.querySelector<HTMLInputElement>("input")?.value === "Original")!;
    expect(description.querySelector(".provenance-badge")?.textContent).toBe("Source: project edit");
    expect(v.host.querySelectorAll(".com-object-edit-fields > .provenance-badge")).toHaveLength(0);
  } finally { await v.cleanup(); }
});

// UI-02 (owner request 7): "Source: device instance" is routine noise in the
// field headings. The layer stays in the data and in the opened technical reference.
for (const [language, instance] of [["en", "Source: device instance"], ["de", "Quelle: Geräteinstanz"]] as const) {
  it(`omits the routine ${language} device-instance source from field headings and keeps it on record`, async () => {
    setUiLanguage(language);
    const com = object({ dpt: "DPST-9-1", dpt_layer: "Instance", description: "Stated", description_layer: "Instance" });
    const v = await render(com);
    try {
      expect(v.host.querySelectorAll(".inspector-field-heading .provenance-badge")).toHaveLength(0);
      expect(v.host.querySelector(".inspector-field-heading")?.textContent).not.toContain(instance);
      const reference = v.host.querySelector<HTMLDetailsElement>(".dpt-field .dpt-reference")!;
      expect(reference.open).toBe(false);
      const badge = reference.querySelector(".provenance-badge");
      expect(badge?.textContent).toBe(instance);
      expect(badge?.getAttribute("data-source-layer")).toBe("Instance");
      // Presentation only: the object keeps both layers and nothing is written.
      expect(com.dpt_layer).toBe("Instance");
      expect(com.description_layer).toBe("Instance");
      expect(api.setComObjectDpt).not.toHaveBeenCalled();
    } finally { await v.cleanup(); }
  });
}
// Review UI02-003: each field's Instance layer is reachable on its own; hiding the
// heading badge must never leave a layer without a deliberately opened reference.
const descriptionField = (host: HTMLElement) =>
  [...host.querySelectorAll<HTMLElement>(".inspector-field")].find(field => field.querySelector<HTMLInputElement>("input")?.value === "Stated")!;
it("names a description-only Instance layer in the description's technical reference", async () => {
  const v = await render(object({ dpt_layer: "Program", description: "Stated", description_layer: "Instance" }));
  try {
    const description = descriptionField(v.host);
    expect(description.querySelector(".inspector-field-heading .provenance-badge")).toBeNull();
    const reference = description.querySelector<HTMLDetailsElement>(".value-source-reference")!;
    expect(reference, "description has its own technical reference").not.toBeNull();
    expect(reference.open).toBe(false);
    expect(reference.querySelector("summary")?.textContent).toBe("Technical reference");
    expect(reference.querySelector(".provenance-badge")?.getAttribute("data-source-layer")).toBe("Instance");
    expect(reference.querySelector(".provenance-badge")?.textContent).toBe("Source: device instance");
    // The DPT keeps its non-routine layer in the heading, and its reference does not repeat it.
    expect(v.host.querySelector(".dpt-field .inspector-field-heading .provenance-badge")?.textContent).toBe("Source: application program");
    expect(v.host.querySelectorAll(".dpt-field .dpt-reference .provenance-badge")).toHaveLength(0);
  } finally { await v.cleanup(); }
});
it("names a DPT-only Instance layer in the DPT reference and keeps the description heading badge", async () => {
  const v = await render(object({ dpt_layer: "Instance", description: "Stated", description_layer: "UserEdit" }));
  try {
    expect(v.host.querySelector(".dpt-field .inspector-field-heading .provenance-badge")).toBeNull();
    expect(v.host.querySelector(".dpt-field .dpt-reference .provenance-badge")?.getAttribute("data-source-layer")).toBe("Instance");
    const description = descriptionField(v.host);
    expect(description.querySelector(".inspector-field-heading .provenance-badge")?.textContent).toBe("Source: project edit");
    expect(description.querySelector(".value-source-reference")).toBeNull();
  } finally { await v.cleanup(); }
});
it("names the description reference in German", async () => {
  setUiLanguage("de");
  const v = await render(object({ description: "Stated", description_layer: "Instance" }));
  try {
    const reference = descriptionField(v.host).querySelector<HTMLDetailsElement>(".value-source-reference")!;
    expect(reference.querySelector("summary")?.textContent).toBe("Technische Referenz");
    expect(reference.querySelector(".provenance-badge")?.textContent).toBe("Quelle: Geräteinstanz");
  } finally { await v.cleanup(); }
});
it("keeps every non-routine source visible in the field heading", async () => {
  for (const [layer, text] of [["ProgramRef", "Source: application reference"], ["Inferred", "Source: inferred value"], ["UserEdit", "Source: project edit"]] as const) {
    const v = await render(object({ dpt_layer: layer }));
    try {
      expect(v.host.querySelector(".dpt-field .inspector-field-heading .provenance-badge")?.textContent).toBe(text);
      // Shown once: the technical reference does not repeat a heading badge.
      expect(v.host.querySelectorAll(".dpt-field .dpt-reference .provenance-badge")).toHaveLength(0);
    } finally { await v.cleanup(); }
  }
});

it("preserves an unrecognized provenance identifier even when it names a prototype property", async () => {
  const v = await render(object({ dpt_layer: "constructor" }));
  try { expect(v.host.querySelector(".dpt-field .provenance-badge")?.textContent).toBe("Source: constructor"); }
  finally { await v.cleanup(); }
});

it("clears an explicit project DPT only after choosing the reset option", async () => {
  const v = await render(object());
  try {
    const select = v.host.querySelector<HTMLSelectElement>(".dpt-field select")!;
    api.setComObjectDpt.mockResolvedValue(tree);
    await act(async () => { select.value = ""; select.dispatchEvent(new Event("change", { bubbles: true })); });
    expect(api.setComObjectDpt).not.toHaveBeenCalled();
    await act(async () => select.dispatchEvent(new FocusEvent("focusout", { bubbles: true })));
    expect(api.setComObjectDpt).toHaveBeenCalledExactlyOnceWith(1, null);
  } finally { await v.cleanup(); }
});
