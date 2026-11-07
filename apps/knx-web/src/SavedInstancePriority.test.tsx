/** Tests that the saved-priority row shows every projected state exactly and never a default. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it } from "vitest";
import type { CapturedSavedInstanceSource } from "./bindings/CapturedSavedInstanceSource";
import type { SavedInstancePriorityView } from "./bindings/SavedInstancePriorityView";
import type { SavedPriorityUnavailableView } from "./bindings/SavedPriorityUnavailableView";
import { SavedInstancePriorityRow } from "./SavedInstancePriority";
import { messages as de } from "./messages/de";
import { messages as en } from "./messages/en";
import { resetUiLanguageForTests } from "./uiLanguage";
import { settingsStorage } from "./settingsStore";

function setUiLanguage(language: string) { settingsStorage.setItem("uiLanguage", language); resetUiLanguageForTests(); }

const source = {
  kind: "CapturedSavedInstance", source_schema: 23,
  device_source: { path: "P-0001/0.xml", ets_id: "P-0001-0_DI-1" },
  object_source: { path: "P-0001/0.xml", ets_id: "O-7_R-9" },
  owner_xpath: "/KNX/Project/Installations/Installation[1]", topology_sha256: "a".repeat(64),
} satisfies CapturedSavedInstanceSource;

let cleanup: (() => void) | null = null;
afterEach(() => { cleanup?.(); cleanup = null; setUiLanguage("en"); });

function render(priority: SavedInstancePriorityView): HTMLElement {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<SavedInstancePriorityRow priority={priority} />));
  cleanup = () => { act(() => root.unmount()); host.remove(); };
  return host.querySelector<HTMLElement>(".com-object-saved-priority")!;
}

it("shows a known declaration verbatim, with its captured source and the not-effective hint", () => {
  const row = render({ status: "declared", raw: "High", token_status: "Known", source });
  expect(row.dataset.status).toBe("declared");
  expect(row.querySelector(".saved-priority-raw")?.textContent).toBe("High");
  expect(row.querySelector(".saved-priority-state")?.textContent).toBe(en["inspector.savedPriority.declared.known"]);
  expect(row.querySelector(".saved-priority-source")?.textContent).toBe("Captured from project schema 23, object O-7_R-9");
  expect(row.textContent).toContain(en["inspector.savedPriority.hint"]);
});

it("keeps empty, whitespace, case-different and unknown literals exact and quoted", () => {
  for (const [raw, status, shown] of [
    ["", "Empty", '""'],
    ["  low ", "Unknown", '"  low "'],
    ["alert", "Unknown", '"alert"'],
    ["Höchste", "Unknown", '"Höchste"'],
  ] as const) {
    const row = render({ status: "declared", raw, token_status: status, source });
    expect(row.querySelector(".saved-priority-raw")?.textContent).toBe(shown);
    expect(row.querySelector(".saved-priority-state")?.textContent).toBe(
      en[status === "Empty" ? "inspector.savedPriority.declared.empty" : "inspector.savedPriority.declared.unknown"]);
    cleanup!(); cleanup = null;
  }
});

it("names both absence causes without inventing a value", () => {
  for (const omission of ["AttributeOmitted", "InstanceElementOmitted"] as const) {
    const row = render({ status: "absent", omission, source });
    expect(row.querySelector(".saved-priority-raw")).toBeNull();
    expect(row.querySelector(".saved-priority-state")?.textContent).toBe(en[`inspector.savedPriority.absent.${omission}`]);
    expect(row.querySelector(".saved-priority-source")?.textContent).toContain("O-7_R-9");
    cleanup!(); cleanup = null;
  }
});

it("names every unavailable reason, including the projection-only binding mismatch", () => {
  const reasons: SavedPriorityUnavailableView[] = ["NotCaptured", "UnsupportedSourceSchema", "ModuleScope",
    "AmbiguousOwner", "AmbiguousDeclaration", "SourceIntegrity", "ResourceLimit", "SourceBindingMismatch"];
  const seen = new Set<string>();
  for (const reason of reasons) {
    const row = render({ status: "unavailable", reason });
    const text = row.querySelector(".saved-priority-state")!.textContent!;
    expect(text).toBe(en[`inspector.savedPriority.unavailable.${reason}`]);
    expect(row.querySelector(".saved-priority-raw")).toBeNull();
    expect(row.querySelector(".saved-priority-source")).toBeNull();
    seen.add(text);
    cleanup!(); cleanup = null;
  }
  expect(seen.size).toBe(reasons.length);
});

it("reports a newer server's state or reason as unknown instead of a default", () => {
  for (const priority of [
    { status: "inherited", raw: "Low" },
    { status: "unavailable", reason: "SomethingNew" },
    { status: "unavailable", reason: "constructor" },
    { status: "unavailable", reason: "toString" },
    { status: "declared", raw: "Low", token_status: "Later", source },
    { status: "absent", omission: "Later", source },
  ]) {
    const row = render(priority as unknown as SavedInstancePriorityView);
    expect(row.querySelector(".saved-priority-state")?.textContent).toBe(en["inspector.savedPriority.unrecognisedState"]);
    // A declared literal stays visible, quoted, even when its token status is new.
    expect(row.querySelector(".saved-priority-raw")?.textContent ?? null).toBe(priority.status === "declared" ? '"Low"' : null);
    expect(row.textContent).not.toContain(en["inspector.savedPriority.declared.known"] + "Low");
    cleanup!(); cleanup = null;
  }
});

it("speaks German, including the hint", () => {
  setUiLanguage("de");
  const row = render({ status: "unavailable", reason: "NotCaptured" });
  expect(row.querySelector(".saved-priority-label")?.textContent).toBe(de["inspector.savedPriority.label"]);
  expect(row.querySelector(".saved-priority-state")?.textContent).toBe(de["inspector.savedPriority.unavailable.NotCaptured"]);
  expect(row.textContent).toContain(de["inspector.savedPriority.hint"]);
});

it("does not crash on a reason named like an object prototype member", () => {
  const priority = JSON.parse('{"status":"unavailable","reason":"__proto__"}') as SavedInstancePriorityView;
  const row = render(priority);
  expect(row.querySelector(".saved-priority-state")?.textContent).toBe(en["inspector.savedPriority.unrecognisedState"]);
});
