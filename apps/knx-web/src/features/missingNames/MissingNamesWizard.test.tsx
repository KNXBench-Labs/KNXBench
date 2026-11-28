/** WP-51.3a: the missing-name wizard scans, caps, previews and applies exactly the shown plan. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import MissingNamesWizard from "./MissingNamesWizard";
import { proposal, target } from "./missingNames.test-fixtures";
import type { MissingNameProposal, MissingNameScan } from "./client";
import { renameTree } from "../../rename.fixture";
import type { ProjectTree } from "../../bindings/ProjectTree";
import { settingsStorage, resetSettingsForTests } from "../../settingsStore";
import { resetUiLanguageForTests, saveUiLanguage } from "../../uiLanguage";
import { messages as en } from "../../messages/en";
import { messages as de } from "../../messages/de";

const client = vi.hoisted(() => ({ scanMissingNames: vi.fn(), reevaluateMissingNames: vi.fn(), previewMissingNames: vi.fn(),
  applyMissingNames: vi.fn(), cancelMissingNames: vi.fn() }));
vi.mock("./client", async (actual) => ({ ...(await actual<typeof import("./client")>()), ...client }));
const api = vi.hoisted(() => ({ currentProject: vi.fn() }));
vi.mock("../../api", async (actual) => ({ ...(await actual<typeof import("../../api")>()), ...api }));

let host: HTMLDivElement;
let root: Root;
const updated = vi.fn();
const closed = vi.fn();
const stale = vi.fn();
const BINDING = { serverIncarnation: "srv", projectIncarnation: 3, snapshotRevision: 7 };
const TREE = { ...renameTree(), server_incarnation: "srv", project_incarnation: 3, snapshot_revision: 7 } as ProjectTree;
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((settle) => { resolve = settle; });
  return { promise, resolve };
}
const refusal = (status: number, kind: string, message: string) => Object.assign(new Error(message), { status, kind });

function scan(items: readonly MissingNameProposal[], options: Partial<{ complete: boolean; offset: number; hasMore: boolean;
  persistence: MissingNameScan["persistence"] }> = {}): MissingNameScan {
  const complete = options.complete ?? true;
  return { binding: BINDING, persistence: options.persistence ?? "native-working-history", items, discovery: {
    completeness: complete ? "complete" : "incomplete", offset: options.offset ?? 0, limit: 100, has_more_in_scan: options.hasMore ?? false,
    counts: { entities_scanned: 40, candidates_found_in_scan: items.length, total_candidates: complete ? items.length : null,
      total_by_kind: complete ? {} : null } } };
}
const ITEMS = [proposal("area", 1, "Area 1"), proposal("group_address", 10, "Light"), proposal("device", 5, null)];
const PREVIEW = { token: "shown-token", expiresInSeconds: 300, persistence: "native-working-history" as const, changed: 1,
  rows: [{ target: target("group_address", 10), kind: "group_address" as const, before: "", after: "Light" }], warnings: [] };

beforeEach(() => {
  vi.clearAllMocks();
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  resetSettingsForTests(); resetUiLanguageForTests(); saveUiLanguage(settingsStorage, "en");
  client.scanMissingNames.mockResolvedValue(scan(ITEMS));
  client.previewMissingNames.mockResolvedValue(PREVIEW);
  client.applyMissingNames.mockResolvedValue({ changed: true, nativeGeneration: 4, snapshotRevision: 8, persistence: "native-working-history" });
  api.currentProject.mockResolvedValue({ ...TREE, snapshot_revision: 8 });
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
});
afterEach(() => { act(() => root.unmount()); host.remove(); vi.unstubAllGlobals(); resetSettingsForTests(); resetUiLanguageForTests(); });

async function render(tree: ProjectTree = TREE) {
  await act(async () => root.render(<MissingNamesWizard tree={tree} onTreeUpdate={updated} onStale={stale} onClose={closed} />));
}
const panel = () => document.querySelector(".missing-names-panel") as HTMLElement;
const button = (selector: string) => panel().querySelector<HTMLButtonElement>(selector)!;
const entryBox = (kind: string, index = 0) =>
  panel().querySelectorAll<HTMLInputElement>(`[data-missing-group="${kind}"] li input[type=checkbox]`)[index]!;
const nameInput = (kind: string, index = 0) =>
  panel().querySelectorAll<HTMLInputElement>(`[data-missing-group="${kind}"] .missing-names-name`)[index]!;
async function click(element: HTMLElement) { await act(async () => element.click()); }
async function type(input: HTMLInputElement, text: string) {
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, text);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

describe("MissingNamesWizard", () => {
  it("is a named dialog that groups entries with context, ID and provenance", async () => {
    await render();
    const dialog = panel().closest("[role=dialog]") ?? panel();
    expect(dialog.getAttribute("aria-labelledby")).toBe("missing-names-title");
    expect(panel().textContent).toContain("3 unnamed objects");
    expect(panel().querySelector("[data-incomplete]")).toBeNull();
    expect([...panel().querySelectorAll("[data-missing-group]")].map((group) => group.getAttribute("data-missing-group")))
      .toEqual(["area", "device", "group_address"].filter((kind) => panel().querySelector(`[data-missing-group="${kind}"]`)));
    expect(nameInput("group_address").value).toBe("Light");
    expect(nameInput("device").value).toBe("");
    expect(nameInput("device").placeholder).toBe("Type a name");
    expect(panel().textContent).toContain("No proposal: type a name");
    expect(panel().textContent).toContain("Proposed from the project");
  });

  it("labels an incomplete scan without a full total and pages through it", async () => {
    client.scanMissingNames.mockResolvedValueOnce(scan(ITEMS, { complete: false, hasMore: true }));
    await render();
    expect(panel().querySelector("[data-incomplete]")?.textContent).toBe("incomplete");
    expect(panel().textContent).toContain("3 unnamed objects found so far (40 objects checked)");
    client.scanMissingNames.mockResolvedValueOnce(scan([proposal("line", 2, "Line 2")], { complete: false, offset: 3 }));
    await click([...panel().querySelectorAll<HTMLButtonElement>(".missing-names-pages button")].at(-1)!);
    expect(client.scanMissingNames).toHaveBeenLastCalledWith({ offset: 3 });
    expect(nameInput("line").value).toBe("Line 2");
  });

  it("previews only the selected entries, then applies the shown token and re-reads the project", async () => {
    await render();
    expect(button("[data-missing-preview]").disabled).toBe(true);
    await click(entryBox("group_address"));
    await type(nameInput("group_address"), "Light kitchen");
    await click(button("[data-missing-preview]"));
    expect(client.previewMissingNames).toHaveBeenCalledWith(BINDING, [{ target: target("group_address", 10), name: "Light kitchen" }]);
    expect(panel().querySelector("[data-preview-row]")).not.toBeNull();
    expect(client.applyMissingNames).not.toHaveBeenCalled();
    await click(button("[data-missing-confirm]"));
    expect(client.applyMissingNames).toHaveBeenCalledWith("shown-token");
    expect(updated).toHaveBeenCalledWith(expect.objectContaining({ snapshot_revision: 8 }));
    expect(panel().textContent).toContain("1 object was named.");
  });

  it("caps the selection at 256 and never submits 257", async () => {
    client.scanMissingNames.mockResolvedValueOnce(scan(Array.from({ length: 257 }, (_, i) => proposal("group_address", i + 1, `GA ${i}`))));
    await render();
    await click(panel().querySelector<HTMLInputElement>('[data-missing-group="group_address"] h4 input')!);
    expect(panel().querySelector("[data-too-many]")?.textContent).toContain("Clear 1 to continue");
    expect(button("[data-missing-preview]").disabled).toBe(true);
    await click(button("[data-missing-preview]"));
    expect(client.previewMissingNames).not.toHaveBeenCalled();
    await click(entryBox("group_address", 0));
    expect(panel().querySelector("[data-too-many]")).toBeNull();
    await click(button("[data-missing-preview]"));
    expect(client.previewMissingNames.mock.calls[0]![1]).toHaveLength(256);
  });

  it("warns about duplicate names without blocking the preview", async () => {
    client.scanMissingNames.mockResolvedValueOnce(scan([proposal("group_address", 1, "Light"), proposal("group_address", 2, "Light")]));
    await render();
    await click(entryBox("group_address", 0)); await click(entryBox("group_address", 1));
    expect(panel().querySelectorAll("[data-duplicate]").length).toBeGreaterThan(0);
    expect(nameInput("group_address", 1).value).toBe("Light");
    expect(button("[data-missing-preview]").disabled).toBe(false);
  });

  it("re-evaluates dependents after a parent name changes", async () => {
    client.reevaluateMissingNames.mockResolvedValue({ invalidated_targets: [target("group_address", 10)],
      proposals: [proposal("group_address", 10, "Area 9 Light")] });
    await render();
    expect(panel().querySelector("[data-missing-reevaluate]")).toBeNull();
    await type(nameInput("area"), "Area 9");
    await click(button("[data-missing-reevaluate]"));
    expect(client.reevaluateMissingNames.mock.calls[0]![0]).toEqual(BINDING);
    expect(client.reevaluateMissingNames.mock.calls[0]![1]).toEqual([target("area", 1)]);
    expect(nameInput("group_address").value).toBe("Area 9 Light");
    expect(panel().querySelector("[data-recheck=updated]")).not.toBeNull();
    expect(panel().querySelector("[data-missing-reevaluate]")).toBeNull();
  });

  it("blocks the preview after a parent edit until Re-evaluate has run", async () => {
    client.reevaluateMissingNames.mockResolvedValue({ invalidated_targets: [], proposals: [] });
    await render();
    await click(entryBox("group_address"));
    expect(button("[data-missing-preview]").disabled).toBe(false);
    await type(nameInput("area"), "Area 9");
    expect(button("[data-missing-preview]").disabled).toBe(true);
    expect(panel().textContent).toContain(en["missingNames.reevaluateHint"]);
    await click(button("[data-missing-preview]"));
    expect(client.previewMissingNames).not.toHaveBeenCalled();
    await click(button("[data-missing-reevaluate]"));
    expect(button("[data-missing-preview]").disabled).toBe(false);
  });

  it("re-proposes a dependent child on a page loaded only after Re-evaluate", async () => {
    client.scanMissingNames.mockResolvedValueOnce(scan([proposal("area", 1, "Area 1")], { hasMore: true }));
    client.reevaluateMissingNames.mockResolvedValue({ invalidated_targets: [target("group_address", 20)],
      proposals: [proposal("group_address", 20, "Area 9 Light")] });
    await render();
    await type(nameInput("area"), "Area 9");
    await click(button("[data-missing-reevaluate]"));
    client.scanMissingNames.mockResolvedValueOnce(scan([proposal("group_address", 20, "Light")], { offset: 1 }));
    await click([...panel().querySelectorAll<HTMLButtonElement>(".missing-names-pages button")].at(-1)!);
    expect(nameInput("group_address").value).toBe("Area 9 Light");
    expect(panel().querySelector("[data-recheck=updated]")).not.toBeNull();
    await click(entryBox("group_address"));
    await click(button("[data-missing-preview]"));
    expect(client.previewMissingNames).toHaveBeenCalledWith(BINDING, [{ target: target("group_address", 20), name: "Area 9 Light" }]);
  });

  it("shows the provenance and gaps of the re-evaluated proposal", async () => {
    client.reevaluateMissingNames.mockResolvedValue({ invalidated_targets: [target("group_address", 10)],
      proposals: [proposal("group_address", 10, null, { gaps: [{ gap: "unselected_parent", parent: target("area", 1) }] })] });
    await render();
    const entry = () => panel().querySelector('[data-missing-group="group_address"] li')!;
    expect(entry().textContent).toContain("Proposed from the project");
    expect(entry().querySelector(".missing-names-gaps")).toBeNull();
    await type(nameInput("area"), "Area 9");
    await click(button("[data-missing-reevaluate]"));
    expect(entry().textContent).toContain("No proposal: type a name");
    expect(entry().textContent).not.toContain("Proposed from the project");
    expect(entry().querySelector(".missing-names-gaps")?.textContent).toContain("which is not selected");
  });

  it("cancels the shown preview when Apply answers stale", async () => {
    client.applyMissingNames.mockRejectedValueOnce(refusal(409, "missingNameStale", "stale"));
    await render();
    await click(entryBox("group_address"));
    await click(button("[data-missing-preview]"));
    expect(client.cancelMissingNames).not.toHaveBeenCalled();
    await click(button("[data-missing-confirm]"));
    expect(client.cancelMissingNames).toHaveBeenCalledTimes(1);
    expect(client.cancelMissingNames).toHaveBeenCalledWith("shown-token");
    expect(stale).toHaveBeenCalled();
    expect(panel().querySelector("[data-preview-row]")).toBeNull();
    expect(updated).not.toHaveBeenCalled();
  });

  it("reports a stale preview, tells the caller and applies nothing", async () => {
    client.previewMissingNames.mockRejectedValueOnce(refusal(409, "missingNameStale", "stale"));
    await render();
    await click(entryBox("group_address"));
    await click(button("[data-missing-preview]"));
    expect(stale).toHaveBeenCalled();
    expect(panel().querySelector("[role=alert]")?.textContent).toContain("The project changed");
    expect(button("[data-missing-preview]").disabled).toBe(true);
  });

  it("shows a refusal and cancels the shown preview", async () => {
    client.applyMissingNames.mockRejectedValueOnce(refusal(409, "missingNameTargetNotBlank", "not blank"));
    await render();
    await click(entryBox("group_address"));
    await click(button("[data-missing-preview]"));
    await click(button("[data-missing-confirm]"));
    expect(panel().querySelector("[role=alert]")?.textContent).toContain("got a name in the meantime");
    expect(panel().querySelector("[data-preview-row]")).toBeNull();
    expect(client.cancelMissingNames).toHaveBeenCalledWith("shown-token");
    expect(updated).not.toHaveBeenCalled();
  });

  it("says when names only last for the session", async () => {
    client.scanMissingNames.mockResolvedValueOnce(scan(ITEMS, { persistence: "session-only" }));
    await render();
    expect(panel().querySelector("[data-session-only]")).not.toBeNull();
  });

  it("closes on Escape without applying and cancels the shown preview", async () => {
    await render();
    await click(entryBox("group_address"));
    await click(button("[data-missing-preview]"));
    await act(async () => { panel().dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })); });
    expect(closed).toHaveBeenCalled();
    expect(client.applyMissingNames).not.toHaveBeenCalled();
    act(() => root.unmount());
    root = createRoot(host);
    expect(client.cancelMissingNames).toHaveBeenCalledWith("shown-token");
  });

  it("discards and cancels a preview that answers after the project changed", async () => {
    const pending = deferred<typeof PREVIEW>();
    client.previewMissingNames.mockReturnValueOnce(pending.promise);
    await render();
    await click(entryBox("group_address"));
    await click(button("[data-missing-preview]"));
    await render({ ...TREE, snapshot_revision: 8 });
    await act(async () => pending.resolve(PREVIEW));
    expect(panel().querySelector("[data-preview-row]")).toBeNull();
    expect(button("[data-missing-confirm]").disabled).toBe(true);
    expect(client.cancelMissingNames).toHaveBeenCalledWith("shown-token");
    expect(panel().querySelector("[role=alert]")?.textContent).toContain("The project changed");
    expect(client.applyMissingNames).not.toHaveBeenCalled();
  });

  it("cancels a preview that answers after the dialog closed", async () => {
    const pending = deferred<typeof PREVIEW>();
    client.previewMissingNames.mockReturnValueOnce(pending.promise);
    await render();
    await click(entryBox("group_address"));
    await click(button("[data-missing-preview]"));
    act(() => root.unmount());
    root = createRoot(host);
    expect(client.cancelMissingNames).not.toHaveBeenCalled();
    await act(async () => pending.resolve(PREVIEW));
    expect(client.cancelMissingNames).toHaveBeenCalledWith("shown-token");
    expect(client.applyMissingNames).not.toHaveBeenCalled();
  });

  it("retires the preview when a name is edited", async () => {
    await render();
    await click(entryBox("group_address"));
    await click(button("[data-missing-preview]"));
    await type(nameInput("group_address"), "Other");
    expect(client.cancelMissingNames).toHaveBeenCalledWith("shown-token");
    expect(button("[data-missing-confirm]").disabled).toBe(true);
  });

  it("has every key in German", () => {
    const keys = Object.keys(en).filter((key) => key.startsWith("missingNames."));
    expect(keys.length).toBeGreaterThan(50);
    expect(keys.filter((key) => !(key in de))).toEqual([]);
  });
});
