/** Tests that the effective-priority row shows every state and source exactly, never a default. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it } from "vitest";
import type { EffectivePriorityUnresolved } from "./bindings/EffectivePriorityUnresolved";
import type { EffectivePriorityView } from "./bindings/EffectivePriorityView";
import { EffectivePriorityRow } from "./EffectivePriority";
import { messages as de } from "./messages/de";
import { messages as en } from "./messages/en";
import { resetUiLanguageForTests } from "./uiLanguage";
import { settingsStorage } from "./settingsStore";

function setUiLanguage(language: string) { settingsStorage.setItem("uiLanguage", language); resetUiLanguageForTests(); }

let cleanup: (() => void) | null = null;
afterEach(() => { cleanup?.(); cleanup = null; setUiLanguage("en"); });

function render(priority: EffectivePriorityView): HTMLElement {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  act(() => root.render(<EffectivePriorityRow priority={priority} />));
  cleanup = () => { act(() => root.unmount()); host.remove(); };
  return host.querySelector<HTMLElement>(".com-object-effective-priority")!;
}

function next() { cleanup!(); cleanup = null; }

it("shows a resolved instance value verbatim with its source and the not-wire hint", () => {
  for (const value of ["Low", "High", "Alert"] as const) {
    const row = render({ status: "resolved", value, source: { kind: "instance" } });
    expect(row.dataset.status).toBe("resolved");
    expect(row.querySelector(".saved-priority-raw")?.textContent).toBe(value);
    expect(row.querySelector(".saved-priority-state")?.textContent).toBe(en["inspector.effectivePriority.resolved"]);
    expect(row.querySelector(".saved-priority-source")?.textContent).toBe(en["inspector.effectivePriority.source.instance"]);
    expect(row.textContent).toContain(en["inspector.effectivePriority.hint"]);
    next();
  }
});

it("names the product layer and the omission that let the product apply", () => {
  const row = render({ status: "resolved", value: "Low",
    source: { kind: "product", layer: "ProgramRef", omission: "InstanceElementOmitted" } });
  expect(row.querySelector(".saved-priority-source")?.textContent).toBe(
    "From the product object reference (ComObjectRef); the project file has no saved entry for the object");
  next();
  const other = render({ status: "resolved", value: "Low",
    source: { kind: "product", layer: "Program", omission: "AttributeOmitted" } });
  expect(other.querySelector(".saved-priority-source")?.textContent).toBe(
    "From the product object (ComObject); the saved object does not declare it");
});

it("names every unresolved reason distinctly and quotes a selected literal exactly", () => {
  const reasons: EffectivePriorityUnresolved[] = ["InstanceUnavailable", "InstanceEmpty", "InstanceUnknown",
    "ProductNotConsulted", "NoProgramReference", "NoProductDatabase", "ProgramNotInDatabase",
    "ProductObjectNotFound", "ProductLiteralAbsent", "ProductEmpty", "ProductUnknown"];
  const seen = new Set<string>();
  for (const reason of reasons) {
    const row = render({ status: "unresolved", reason, source: null, raw: null });
    const text = row.querySelector(".saved-priority-state")!.textContent!;
    expect(text).toBe(en[`inspector.effectivePriority.unresolved.${reason}`]);
    expect(row.querySelector(".saved-priority-raw")).toBeNull();
    expect(row.querySelector(".saved-priority-source")).toBeNull();
    seen.add(text);
    next();
  }
  expect(seen.size).toBe(reasons.length);
  for (const [raw, shown] of [["", '""'], ["  low ", '"  low "'], ["alert", '"alert"']] as const) {
    const row = render({ status: "unresolved", reason: "InstanceUnknown", source: { kind: "instance" }, raw });
    expect(row.querySelector(".saved-priority-raw")?.textContent).toBe(shown);
    expect(row.querySelector(".saved-priority-source")?.textContent).toBe(en["inspector.effectivePriority.source.instance"]);
    next();
  }
});

it("reports a newer server's state, reason, value or source as unknown instead of a default", () => {
  const unknown = en["inspector.effectivePriority.unrecognisedState"];
  for (const priority of [
    { status: "inherited", value: "Low" },
    { status: "unresolved", reason: "SomethingNew", source: null, raw: null },
    { status: "unresolved", reason: "constructor", source: null, raw: null },
    { status: "resolved", value: "Urgent", source: { kind: "instance" } },
  ]) {
    const row = render(priority as unknown as EffectivePriorityView);
    expect(row.querySelector(".saved-priority-state")?.textContent).toBe(unknown);
    expect(row.textContent).not.toContain(en["inspector.effectivePriority.resolved"] + "Low");
    next();
  }
  for (const source of [{ kind: "device" }, { kind: "product", layer: "Hardware", omission: "AttributeOmitted" },
    { kind: "product", layer: "Program", omission: "Later" }]) {
    const row = render({ status: "resolved", value: "Low", source } as unknown as EffectivePriorityView);
    expect(row.querySelector(".saved-priority-source")?.textContent).toBe(unknown);
    next();
  }
});

it("does not crash on a reason named like an object prototype member", () => {
  const priority = JSON.parse('{"status":"unresolved","reason":"__proto__","source":null,"raw":null}') as EffectivePriorityView;
  const row = render(priority);
  expect(row.querySelector(".saved-priority-state")?.textContent).toBe(en["inspector.effectivePriority.unrecognisedState"]);
});

it("speaks German, including the hint", () => {
  setUiLanguage("de");
  const row = render({ status: "unresolved", reason: "NoProductDatabase", source: null, raw: null });
  expect(row.querySelector(".saved-priority-label")?.textContent).toBe(de["inspector.effectivePriority.label"]);
  expect(row.querySelector(".saved-priority-state")?.textContent).toBe(de["inspector.effectivePriority.unresolved.NoProductDatabase"]);
  expect(row.textContent).toContain(de["inspector.effectivePriority.hint"]);
});
