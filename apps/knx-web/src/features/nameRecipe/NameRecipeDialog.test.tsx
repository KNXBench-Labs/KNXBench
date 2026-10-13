/** Goal B7: the naming-recipe dialog binds, retires and confirms exactly the shown preview. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import NameRecipeDialog from "./NameRecipeDialog";
import { renameTree } from "../../rename.fixture";
import type { ProjectTree } from "../../bindings/ProjectTree";
import { settingsStorage, resetSettingsForTests } from "../../settingsStore";
import { resetUiLanguageForTests, saveUiLanguage } from "../../uiLanguage";
import { messages as en } from "../../messages/en";
import { messages as de } from "../../messages/de";

const client = vi.hoisted(() => ({ previewNameRecipe: vi.fn(), applyNameRecipe: vi.fn(), cancelNameRecipe: vi.fn() }));
vi.mock("./client", async (actual) => ({ ...(await actual<typeof import("./client")>()), ...client }));

let host: HTMLDivElement;
let root: Root;
const applied = vi.fn();
const closed = vi.fn();
const stale = vi.fn();
const PLAN = { token: "shown-preview-token", changed: 2, noOp: false, expiresInSeconds: 300, rows: [
  { id: 2, before: "Original", after: "EG Original", changed: true },
  { id: 1, before: "Original", after: "EG Original", changed: true },
] };
const refusal = (status: number, kind: string, message: string) => Object.assign(new Error(message), { status, kind });

beforeEach(() => {
  vi.clearAllMocks();
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  resetSettingsForTests(); resetUiLanguageForTests(); saveUiLanguage(settingsStorage, "en");
  client.previewNameRecipe.mockResolvedValue(PLAN);
  client.applyNameRecipe.mockResolvedValue({ changed: true, nativeGeneration: null, snapshotRevision: 2, persistence: "session-only" });
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
});
afterEach(() => { act(() => root.unmount()); host.remove(); vi.unstubAllGlobals(); resetSettingsForTests(); resetUiLanguageForTests(); });

async function render(tree: ProjectTree = renameTree(), ids: number[] = [2, 1]) {
  await act(async () => root.render(<NameRecipeDialog tree={tree} ids={ids} onApplied={applied} onClose={closed} onStale={stale} />));
}
const panel = () => document.querySelector(".name-recipe-panel") as HTMLElement;
const confirmButton = () => panel().querySelector<HTMLButtonElement>("[data-recipe-confirm]")!;
const previewButton = () => panel().querySelector<HTMLButtonElement>(".name-recipe-form button[type=submit]")!;
async function type(selector: string, text: string) {
  const input = panel().querySelector<HTMLInputElement>(selector)!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!.call(input, text);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
}
async function preview() { await act(async () => previewButton().click()); }

describe("NameRecipeDialog", () => {
  it("previews the chosen ids in the chosen order and confirms only the shown token", async () => {
    await render();
    expect(previewButton().disabled, "nothing to add yet").toBe(true);
    expect(confirmButton().disabled).toBe(true);
    await type("[data-recipe-prefix]", "EG ");
    await preview();
    expect(client.previewNameRecipe).toHaveBeenCalledWith(
      { serverIncarnation: "rename", projectIncarnation: 0, snapshotRevision: 1 }, [2, 1], "EG ", "");
    const rows = [...panel().querySelectorAll("[data-recipe-row]")].map((row) => row.textContent);
    expect(rows).toEqual(["1/1/1OriginalEG Original", "1/1/1OriginalEG Original"]);
    expect(panel().textContent).toContain("2 of 2 names change");
    expect(confirmButton().textContent).toBe("Rename 2 names");
    expect(client.applyNameRecipe).not.toHaveBeenCalled();
    await act(async () => confirmButton().click());
    expect(client.applyNameRecipe).toHaveBeenCalledExactlyOnceWith("shown-preview-token");
    expect(applied).toHaveBeenCalledOnce();
    expect(closed).toHaveBeenCalledOnce();
  });

  it("retires the preview, here and on the server, as soon as the text changes", async () => {
    await render();
    await type("[data-recipe-suffix]", " (alt)");
    await preview();
    await type("[data-recipe-suffix]", " (neu)");
    expect(panel().querySelector(".name-recipe-preview")).toBeNull();
    expect(confirmButton().disabled).toBe(true);
    expect(client.cancelNameRecipe).toHaveBeenCalledWith("shown-preview-token");
  });

  it("retires the preview when the project moves on, and says so", async () => {
    await render();
    await type("[data-recipe-prefix]", "EG ");
    await preview();
    await render({ ...renameTree(), snapshot_revision: 2 });
    expect(panel().querySelector(".name-recipe-preview")).toBeNull();
    expect(confirmButton().disabled).toBe(true);
    expect(panel().querySelector("[role=alert]")?.textContent).toBe(en["nameRecipe.stale"]);
    expect(client.cancelNameRecipe).toHaveBeenCalledWith("shown-preview-token");
  });

  it("a stale confirmation renames nothing and asks for a new preview", async () => {
    client.applyNameRecipe.mockRejectedValue(refusal(409, "nameRecipeStale", "The project changed since this preview; preview again"));
    await render();
    await type("[data-recipe-prefix]", "EG ");
    await preview();
    await act(async () => confirmButton().click());
    expect(applied).not.toHaveBeenCalled();
    expect(closed).not.toHaveBeenCalled();
    expect(panel().querySelector(".name-recipe-preview")).toBeNull();
    expect(confirmButton().disabled).toBe(true);
    expect(panel().querySelector("[role=alert]")?.textContent).toBe(en["nameRecipe.stale"]);
    expect(stale, "the caller re-reads the project so the next preview can bind").toHaveBeenCalledOnce();
  });

  it("a stale preview also asks the caller for the current project", async () => {
    client.previewNameRecipe.mockRejectedValue(refusal(409, "nameRecipeStale", "preview again"));
    await render();
    await type("[data-recipe-prefix]", "EG ");
    await preview();
    expect(stale).toHaveBeenCalledOnce();
    expect(panel().querySelector("[role=alert]")?.textContent).toBe(en["nameRecipe.stale"]);
    // Any other refusal is not a reason to re-read anything.
    stale.mockClear();
    client.previewNameRecipe.mockRejectedValue(refusal(409, "projectBusy", "busy"));
    await preview();
    expect(stale).not.toHaveBeenCalled();
  });

  it.each(["en", "de"] as const)("names a refusal in %s and keeps the server's words verbatim", async (language) => {
    saveUiLanguage(settingsStorage, language); resetUiLanguageForTests();
    const raw = "The new name of group address 2 is not allowed: ControlCharacter('\\u{0}')";
    client.previewNameRecipe.mockRejectedValue(refusal(422, "nameRecipeInvalidName", raw));
    await render();
    await type("[data-recipe-prefix]", "\u0000");
    await preview();
    const alert = panel().querySelector("[role=alert]")!;
    const catalogue = language === "de" ? de : en;
    expect(alert.querySelector("strong")?.textContent).toBe(catalogue["nameRecipe.refusal.invalidName"]);
    expect(alert.querySelector("p")?.textContent).toBe(raw);
    expect(alert.querySelector("small")?.textContent).toBe(catalogue["toast.error.messageIsEnglish"]);
  });

  it("an unknown refusal falls back to the general lead, never to a raw key", async () => {
    client.previewNameRecipe.mockRejectedValue(refusal(422, "somethingNew", "raw words"));
    await render();
    await type("[data-recipe-prefix]", "X");
    await preview();
    expect(panel().querySelector("[role=alert] strong")?.textContent).toBe(en["nameRecipe.failed"]);
  });

  it("a preview that changes nothing cannot be confirmed", async () => {
    client.previewNameRecipe.mockResolvedValue({ ...PLAN, changed: 0, noOp: true,
      rows: PLAN.rows.map((row) => ({ ...row, after: row.before, changed: false })) });
    await render();
    await type("[data-recipe-prefix]", "EG ");
    await preview();
    expect(panel().textContent).toContain(en["nameRecipe.nothingChanges"]);
    expect(panel().querySelectorAll(".name-recipe-unchanged")).toHaveLength(2);
    expect(confirmButton().disabled).toBe(true);
    expect(confirmButton().textContent).toBe(en["nameRecipe.confirmIdle"]);
  });

  it("refuses more than the service allows before asking it", async () => {
    await render(renameTree(), Array.from({ length: 257 }, (_, i) => i + 1));
    await type("[data-recipe-prefix]", "EG ");
    expect(previewButton().disabled).toBe(true);
    expect(panel().textContent).toContain("Select at most 256 group addresses at a time.");
    await type("[data-recipe-prefix]", "x".repeat(4097));
    expect(panel().textContent).toContain(en["nameRecipe.tooLong"]);
  });

  it("closing with a shown preview cancels it on the server", async () => {
    await render();
    await type("[data-recipe-prefix]", "EG ");
    await preview();
    act(() => root.unmount());
    expect(client.cancelNameRecipe).toHaveBeenCalledWith("shown-preview-token");
    root = createRoot(host);
  });

  it("counts with real plurals in both languages", () => {
    for (const catalogue of [en, de]) {
      for (const key of ["nameRecipe.title", "nameRecipe.changes", "nameRecipe.confirm"]) {
        expect(catalogue[`${key}.one` as keyof typeof catalogue], key).toBeTruthy();
        expect(catalogue[`${key}.other` as keyof typeof catalogue], key).toBeTruthy();
      }
    }
    expect(de["nameRecipe.title.one"].replace("{count}", "1")).toBe("1 Gruppenadresse umbenennen");
  });
});
