/** Plug-in data dialog: device pairs, bound preview, token-only apply, refusals. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
beforeEach(() => Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true }));
import type { ProjectTree } from "./bindings/ProjectTree";
import type { AdoptionCandidate, AdoptionListing, AdoptionPreviewResponse } from "./pluginData";
const api = vi.hoisted(() => ({
  listPluginDataCandidates: vi.fn(), previewPluginData: vi.fn(), applyPluginData: vi.fn(), cancelPluginData: vi.fn(), currentProject: vi.fn(),
}));
const picker = vi.hoisted(() => ({ pickOpenPath: vi.fn() }));
vi.mock("./api", () => api);
vi.mock("./filePicker", () => picker);
import PluginDataDialog, { PluginDataButton } from "./PluginDataDialog";
import { resetSettingsForTests, settingsStorage } from "./settingsStore";
import { resetUiLanguageForTests } from "./uiLanguage";

let root: ReturnType<typeof createRoot> | undefined;
let host: HTMLDivElement | undefined;
const update = vi.fn();
const close = vi.fn();
const history = vi.fn();
const tree: ProjectTree = { schema_version: 11, errors: 0, warnings: 0, can_undo: true, can_redo: false, is_modified: false,
  server_incarnation: "test-server", project_incarnation: 4, snapshot_revision: 7, group_address_style: "ThreeLevel",
  installations: [{ id: 0, name: "Destination", topology: [], buildings: [], unassigned: [], group_addresses: [], group_ranges: [] }] };
const device = (id: number, name: string, address: string | null) => ({ id, name, etsId: `P-0001-0_DI-${id}`, address });
const adoptable: AdoptionCandidate = { sourceDevice: device(1, "Sensor (edited)", "1.1.2"), targetDevice: device(10, "Sensor", "1.1.1"), refusal: null, refusalDetail: null, replacements: 2 };
const refused: AdoptionCandidate = { sourceDevice: device(2, "Dimmer", "1.1.3"), targetDevice: device(10, "Sensor", "1.1.1"),
  refusal: "links differ", refusalDetail: { code: "groupLinksDiffer", message: "links differ" }, replacements: 0 };
const elsewhere: AdoptionCandidate = { ...adoptable, sourceDevice: device(3, "Other", "1.1.9"), targetDevice: device(30, "Other", "1.1.9") };
const listing: AdoptionListing = { sourceHash: "a".repeat(64), persistence: "native", candidates: [adoptable, refused, elsewhere] };
const shown: AdoptionPreviewResponse = { confirmationToken: "reviewed-token", preview: {
  sourceHash: listing.sourceHash, request: { sourceDevice: 1, targetDevice: 10 }, sourceDevice: adoptable.sourceDevice, targetDevice: adoptable.targetDevice,
  unchangedPaths: ["P-0001/ExtraData/393257.rbg"],
  replacements: [
    { channel: "BinaryData", path: "P-0001/BinaryData/b113.dat", bytesBefore: 7, bytesAfter: 38, sha256Before: "1".repeat(64), sha256After: "2".repeat(64) },
    { channel: "ExtraData", path: "P-0001/ExtraData/393256.rbg", bytesBefore: 8, bytesAfter: 39, sha256Before: "3".repeat(64), sha256After: "4".repeat(64) },
  ],
  notes: ["Bytes are copied verbatim and never decoded."] } };
const http = (status: number, body: Record<string, unknown>) => Object.assign(new Error(String(body.error)), { status, body });

afterEach(async () => {
  if (root) await act(() => root!.unmount());
  host?.remove(); root = undefined; host = undefined;
  vi.clearAllMocks(); resetSettingsForTests(); resetUiLanguageForTests();
});
function button(label: string) { return [...document.querySelectorAll("button")].find(b => b.textContent === label); }
async function click(label: string) { const target = button(label); expect(target, label).toBeDefined(); await act(async () => target!.click()); }
const text = () => document.body.textContent ?? "";
async function render(value: ProjectTree = tree, deviceId: number | null = 10) {
  if (!host) { host = document.createElement("div"); document.body.appendChild(host); root = createRoot(host); }
  await act(() => root!.render(<PluginDataDialog tree={value} deviceId={deviceId} onTreeUpdate={update} onClose={close} onOpenHistory={history} />));
}
async function choose(value: AdoptionListing = listing, label = "Choose source project…") {
  picker.pickOpenPath.mockResolvedValue("/data/edited.knxproj");
  api.listPluginDataCandidates.mockResolvedValue(value);
  await click(label);
}
async function pick(key: string) {
  await act(async () => document.querySelector<HTMLInputElement>(`[data-plugin-pair="${key}"]`)!.click());
}
async function previewed() {
  api.previewPluginData.mockResolvedValue(shown); api.cancelPluginData.mockResolvedValue({ cancelled: true });
  await render(); await choose(); await pick("1:10"); await click("Preview");
}

it("lists only the device's pairs, says why one cannot adopt, and can show the rest", async () => {
  await render(); await choose();
  expect(api.listPluginDataCandidates).toHaveBeenCalledWith("/data/edited.knxproj", undefined);
  expect(text()).toContain("Source: edited.knxproj");
  expect(text()).toContain("Can adopt: 2 entries differ");
  expect(text()).toContain("Cannot adopt: The devices link their communication objects to different group addresses.");
  expect(document.querySelector('[data-plugin-pair="2:10"]')).toBeNull();
  expect(document.querySelector('[data-plugin-pair="3:30"]')).toBeNull();
  expect(text()).toContain("1 pair for another device is hidden.");
  await act(async () => document.querySelector<HTMLInputElement>("[data-plugin-show-all]")!.click());
  expect(document.querySelector('[data-plugin-pair="3:30"]')).not.toBeNull();
  expect(button("Preview")!.disabled).toBe(true);
  await pick("1:10");
  expect(button("Preview")!.disabled).toBe(false);
});

it("previews under the shown binding, names every entry and digest, and applies with the token only", async () => {
  await previewed();
  expect(api.previewPluginData).toHaveBeenCalledWith({ path: "/data/edited.knxproj", sourceHash: listing.sourceHash,
    request: { sourceDevice: 1, targetDevice: 10 }, serverIncarnation: "test-server", projectIncarnation: 4, snapshotRevision: 7 });
  expect(text()).toContain("Preview — nothing is written yet");
  for (const r of shown.preview.replacements) { expect(text()).toContain(r.path); expect(text()).toContain(r.sha256Before); expect(text()).toContain(r.sha256After); }
  expect(text()).toContain("7 → 38 bytes");
  expect(text()).toContain("1 entry already holds the same bytes and stays untouched");
  expect(text()).toContain("The replaced bytes stay in the project as superseded entries.");
  expect(text()).toContain("Undo does not reverse this.");
  expect(text()).toContain("different individual addresses (1.1.2 and 1.1.1)");
  expect(button("Adopt plug-in data")!.disabled).toBe(true);
  await act(async () => document.querySelector<HTMLInputElement>("[data-plugin-consent]")!.click());
  const applied = { preview: shown.preview, project: { ...tree, snapshot_revision: 8 }, rollbackVersion: { id: 5, label: "Before plug-in data adoption" } };
  api.applyPluginData.mockResolvedValue(applied);
  await click("Adopt plug-in data");
  expect(api.applyPluginData).toHaveBeenCalledWith("reviewed-token");
  expect(update).toHaveBeenCalledWith(applied.project);
  expect(text()).toContain("Plug-in data adopted: 2 entries replaced. Undo does not reverse this. To go back, restore the version “Before plug-in data adoption” in Project history.");
  expect([...document.querySelectorAll("button")].some(b => /undo/i.test(b.textContent ?? ""))).toBe(false);
  await click("Open project history");
  expect(history).toHaveBeenCalledTimes(1);
  await click("Close");
  expect(api.cancelPluginData).not.toHaveBeenCalled();
});

it("refuses to preview a project that has no .knxdb file yet", async () => {
  await render(); await choose({ ...listing, persistence: "session" });
  expect(text()).toContain("Save the project as .knxdb first.");
  await pick("1:10");
  expect(button("Preview")!.disabled).toBe(true);
});

it("a stale apply keeps the choice, writes nothing, and offers a reload instead of a resend", async () => {
  await previewed();
  await act(async () => document.querySelector<HTMLInputElement>("[data-plugin-consent]")!.click());
  api.applyPluginData.mockRejectedValue(http(409, { kind: "pluginDataStale", error: "stale" }));
  await click("Adopt plug-in data");
  expect(text()).toContain("Nothing was adopted. Reload the project and preview again; your choice is kept.");
  expect(document.querySelector<HTMLInputElement>('[data-plugin-pair="1:10"]')!.checked).toBe(true);
  expect(document.querySelector("[data-plugin-consent]")).toBeNull();
  expect(api.applyPluginData).toHaveBeenCalledTimes(1);
  api.currentProject.mockResolvedValue({ ...tree, snapshot_revision: 9 });
  await click("Reload project");
  expect(update).toHaveBeenCalledWith({ ...tree, snapshot_revision: 9 });
  expect(api.applyPluginData).toHaveBeenCalledTimes(1);
});

it("words a refusal at preview time in the UI language", async () => {
  settingsStorage.setItem("uiLanguage", "de"); resetUiLanguageForTests();
  api.previewPluginData.mockRejectedValue(http(422, { kind: "pluginDataRefused", error: "already identical", refusal: { code: "alreadyIdentical", message: "already identical" } }));
  await render(); await choose(listing, "Quellprojekt wählen…"); await pick("1:10"); await click("Vorschau");
  expect(document.querySelector('[role="alert"]')!.textContent).toBe("Dieses Projekt hat schon genau diese Plug-in-Bytes; es gibt nichts zu übernehmen.");
  expect(button("Plug-in-Daten übernehmen")).toBeUndefined();
});

it("closing or a change elsewhere drops the shown preview on the server", async () => {
  await previewed();
  await click("Close");
  expect(api.cancelPluginData).toHaveBeenCalledWith("reviewed-token");
  expect(close).toHaveBeenCalledTimes(1);

  await act(() => root!.unmount()); root = undefined; host?.remove(); host = undefined; vi.clearAllMocks();
  await previewed();
  await render({ ...tree, snapshot_revision: 8 });
  expect(api.cancelPluginData).toHaveBeenCalledWith("reviewed-token");
  expect(text()).not.toContain("Preview — nothing is written yet");
});

it("a lost acknowledgement is reported as uncertain and observed, never replayed", async () => {
  await previewed();
  await act(async () => document.querySelector<HTMLInputElement>("[data-plugin-consent]")!.click());
  api.applyPluginData.mockRejectedValue(new TypeError("Failed to fetch"));
  api.currentProject.mockResolvedValue(tree);
  await click("Adopt plug-in data");
  expect(text()).toContain("a lost response does not prove that nothing was written");
  expect(api.currentProject).toHaveBeenCalledTimes(1);
  expect(api.applyPluginData).toHaveBeenCalledTimes(1);
});

it("the device panel button opens the dialog for that device", async () => {
  host = document.createElement("div"); document.body.appendChild(host); root = createRoot(host);
  await act(() => root!.render(<PluginDataButton tree={tree} deviceId={10} onTreeUpdate={update} />));
  await click("Adopt plug-in data…");
  expect(document.querySelector('[role="dialog"]')).not.toBeNull();
  await choose();
  expect(document.querySelector('[data-plugin-pair="3:30"]')).toBeNull();
});

it("moves focus to each step's result so the keyboard never falls out of the dialog", async () => {
  await render(); await choose();
  expect(document.activeElement).toBe(document.querySelector("[data-plugin-pairs]"));
  api.previewPluginData.mockResolvedValue(shown); await pick("1:10"); await click("Preview");
  expect(document.activeElement).toBe(document.querySelector("#plugin-data-preview-title"));
  await act(async () => document.querySelector<HTMLInputElement>("[data-plugin-consent]")!.click());
  api.applyPluginData.mockRejectedValue(http(409, { kind: "pluginDataStale", error: "stale" }));
  await click("Adopt plug-in data");
  expect(document.activeElement).toBe(document.querySelector('[role="alert"]'));
});

it("a change elsewhere while the source is being listed discards that listing instead of staying busy", async () => {
  let finish!: (value: AdoptionListing) => void;
  picker.pickOpenPath.mockResolvedValue("/data/edited.knxproj");
  api.listPluginDataCandidates.mockReturnValue(new Promise<AdoptionListing>(resolve => { finish = resolve; }));
  await render(); await click("Choose source project…");
  expect(text()).toContain("Working offline…");
  await render({ ...tree, snapshot_revision: 8 });
  expect(text()).not.toContain("Working offline…");
  expect(button("Choose source project…")!.disabled).toBe(false);
  await act(async () => finish(listing));
  expect(document.querySelector('[data-plugin-pair="1:10"]')).toBeNull();
});
