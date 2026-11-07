/** Tests the saved-folder currency notice: changed or unknown shown, unchanged silent. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it } from "vitest";
import type { ProjectFolderCurrency } from "./bindings/ProjectFolderCurrency";
import type { ProjectFolderCurrencyUnknown } from "./bindings/ProjectFolderCurrencyUnknown";
import { FolderCurrencyNotice } from "./FolderCurrency";
import { messages as de } from "./messages/de";
import { messages as en } from "./messages/en";
import { resetUiLanguageForTests } from "./uiLanguage";
import { settingsStorage } from "./settingsStore";

function setUiLanguage(language: string) { settingsStorage.setItem("uiLanguage", language); resetUiLanguageForTests(); }

let cleanup: (() => void) | null = null;
afterEach(() => { cleanup?.(); cleanup = null; setUiLanguage("en"); });

function render(currency: ProjectFolderCurrency | undefined): HTMLElement | null {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<FolderCurrencyNotice currency={currency} />));
  cleanup = () => { act(() => root.unmount()); host.remove(); };
  return host.querySelector<HTMLElement>(".com-folder-currency");
}
function next() { cleanup!(); cleanup = null; }

it("says nothing for an unchanged device or when the server made no claim", () => {
  expect(render({ status: "unchanged" })).toBeNull(); next();
  expect(render(undefined)).toBeNull();
});

it("warns that a changed device's saved folders may no longer match", () => {
  const note = render({ status: "changedSinceImport" })!;
  expect(note.textContent).toBe(en["comTable.folderCurrency.changed"]);
  expect(note.getAttribute("role")).toBe("note");
});

it("names every unknown reason distinctly", () => {
  const reasons: ProjectFolderCurrencyUnknown[] = ["NoRetainedSource", "AmbiguousSource", "SourceIntegrity", "SourceLimit",
    "UnsupportedSchema", "Unreadable", "DeviceNotInSource", "AmbiguousDevice"];
  const seen = new Set<string>();
  for (const reason of reasons) {
    const text = render({ status: "unknown", reason })!.textContent!;
    expect(text).toContain(en[`comTable.folderCurrency.unknown.${reason}`]);
    seen.add(text); next();
  }
  expect(seen.size).toBe(reasons.length);
});

it("reports a newer server's state or reason as unknown, never as current", () => {
  for (const currency of [{ status: "fresh" }, { status: "unknown", reason: "Later" }, { status: "unknown", reason: "constructor" },
    JSON.parse('{"status":"unknown","reason":"__proto__"}')]) {
    const note = render(currency as ProjectFolderCurrency)!;
    expect(note.textContent).toContain(en["comTable.folderCurrency.unrecognised"]);
    next();
  }
});

it("speaks German", () => {
  setUiLanguage("de");
  expect(render({ status: "changedSinceImport" })!.textContent).toBe(de["comTable.folderCurrency.changed"]);
});
