// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it, vi } from "vitest";
import type { ProjectTree } from "./bindings/ProjectTree";
const api = vi.hoisted(() => ({ deviceParameters: vi.fn().mockResolvedValue({ programId: null, sections: [], stale: [], diagnostics: [] }), setComObjectDpt: vi.fn() }));
vi.mock("./api", () => ({ ...api, errorMessage: String }));
import { DeviceWorkspace } from "./Inspector";
it("keeps communication editing and parameters reachable in the central device tabs", async () => {
  const host = document.createElement("div"); document.body.append(host); const root = createRoot(host);
  const tree: ProjectTree = { schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, installations: [] };
  await act(async () => root.render(<DeviceWorkspace detail={{ id: 9, name: "Example", address: null, description: null, com_objects: [] }} tree={tree} onApplied={() => {}} />));
  expect(host.textContent).toContain("Example");
  const tabs = [...host.querySelectorAll<HTMLButtonElement>('[role="tab"]')];
  expect(tabs).toHaveLength(2);
  await act(async () => tabs[1].click());
  expect(tabs[1].getAttribute("aria-selected")).toBe("true");
  expect(api.deviceParameters).toHaveBeenCalled();
  await act(async () => tabs[1].dispatchEvent(new KeyboardEvent("keydown", {key:"ArrowLeft", bubbles:true})));
  expect(tabs[0].getAttribute("aria-selected")).toBe("true");
  await act(async () => root.unmount()); host.remove();
});
