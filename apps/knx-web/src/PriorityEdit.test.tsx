/** Tests the priority edit select: exact tokens, clearing with null and the shown source. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { ComObjectNode } from "./bindings/ComObjectNode";
import type { EffectivePriorityView } from "./bindings/EffectivePriorityView";
import type { ProjectTree } from "./bindings/ProjectTree";
import { messages as de } from "./messages/de";
import { messages as en } from "./messages/en";
import { resetUiLanguageForTests } from "./uiLanguage";
import { settingsStorage } from "./settingsStore";

const tree = { schema_version: 18 } as unknown as ProjectTree;
const api = vi.hoisted(() => ({ setComObjectPriority: vi.fn() }));
vi.mock("./api", () => ({ ...api, errorMessage: (e: unknown) => String(e) }));
import { EffectivePriorityRow, PriorityEditField, priorityEdit } from "./EffectivePriority";

function setUiLanguage(language: string) { settingsStorage.setItem("uiLanguage", language); resetUiLanguageForTests(); }

function object(effective_priority: EffectivePriorityView): ComObjectNode {
  return { id: 7, effective_priority } as unknown as ComObjectNode;
}

let cleanup: (() => void) | null = null;
beforeEach(() => { api.setComObjectPriority.mockReset(); api.setComObjectPriority.mockResolvedValue(tree); });
afterEach(() => { cleanup?.(); cleanup = null; setUiLanguage("en"); });

function render(priority: EffectivePriorityView) {
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  const applied = vi.fn();
  act(() => root.render(<>
    <EffectivePriorityRow priority={priority} />
    <PriorityEditField com={object(priority)} onApplied={applied} />
  </>));
  cleanup = () => { act(() => root.unmount()); host.remove(); };
  const select = host.querySelector<HTMLSelectElement>("select")!;
  const choose = async (value: string) => { await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, "value")!.set!.call(select, value);
    select.dispatchEvent(new Event("change", { bubbles: true }));
  }); };
  return { host, select, choose, applied };
}

it("shows a user edit as the effective value with its own source and preselects it", () => {
  const v = render({ status: "resolved", value: "Alert", source: { kind: "userEdit" } });
  expect(v.host.querySelector(".saved-priority-raw")?.textContent).toBe("Alert");
  expect(v.host.querySelector(".saved-priority-source")?.textContent).toBe(en["inspector.effectivePriority.source.userEdit"]);
  expect(v.select.value).toBe("Alert");
  expect([...v.select.options].map((o) => o.value)).toEqual(["", "Low", "High", "Alert"]);
  expect(v.host.textContent).toContain(en["inspector.priorityEdit.hint"]);
});

it("never presents a captured or product value as the user's edit", () => {
  for (const view of [
    { status: "resolved", value: "High", source: { kind: "instance" } },
    { status: "resolved", value: "Low", source: { kind: "product", layer: "Program", omission: "AttributeOmitted" } },
    { status: "unresolved", reason: "InstanceEmpty", source: { kind: "instance" }, raw: "" },
    { status: "resolved", value: "Urgent", source: { kind: "userEdit" } },
  ] as EffectivePriorityView[]) {
    expect(priorityEdit(view)).toBeNull();
  }
  const v = render({ status: "resolved", value: "High", source: { kind: "instance" } });
  expect(v.select.value).toBe("");
});

it("sends the exact token, clears with null and hands the tree on", async () => {
  const v = render({ status: "resolved", value: "Low", source: { kind: "userEdit" } });
  await v.choose("High");
  expect(api.setComObjectPriority).toHaveBeenLastCalledWith(7, "High");
  await v.choose("");
  expect(api.setComObjectPriority).toHaveBeenLastCalledWith(7, null);
  expect(v.applied).toHaveBeenCalledTimes(2);
  expect(v.applied).toHaveBeenCalledWith(tree);
});

it("shows a refusal from the server", async () => {
  api.setComObjectPriority.mockRejectedValue("communication object instance 7 already has this priority edit");
  const v = render({ status: "resolved", value: "Low", source: { kind: "userEdit" } });
  await v.choose("Alert");
  expect(v.host.querySelector(".field-error")?.textContent).toBe("communication object instance 7 already has this priority edit");
  expect(v.applied).not.toHaveBeenCalled();
});

it("speaks German", () => {
  setUiLanguage("de");
  const v = render({ status: "resolved", value: "Low", source: { kind: "userEdit" } });
  expect(v.host.querySelector("label")?.textContent).toBe(de["inspector.priorityEdit.label"]);
  expect(v.select.options[0].textContent).toBe(de["inspector.priorityEdit.none"]);
  expect(v.host.textContent).toContain(de["inspector.priorityEdit.hint"]);
  expect(v.host.querySelector(".saved-priority-source")?.textContent).toBe(de["inspector.effectivePriority.source.userEdit"]);
});
