/** Tests for the workbench's keyboard pane resizing and its width clamping. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it } from "vitest";
import ResizablePane from "./ResizablePane";

it("resizes a pane with the keyboard and clamps its width", async () => {
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host);
  await act(async () => root.render(<ResizablePane label="Navigation" side="left" initialWidth={250} min={200} max={400}>Tree</ResizablePane>));
  const separator = host.querySelector('[role="separator"]')!;
  expect(separator.getAttribute("aria-valuenow")).toBe("250");
  await act(async () => separator.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true })));
  expect(separator.getAttribute("aria-valuenow")).toBe("266");
  await act(async () => separator.dispatchEvent(new KeyboardEvent("keydown", { key: "End", bubbles: true })));
  expect(separator.getAttribute("aria-valuenow")).toBe("400");
  await act(async () => separator.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true })));
  expect(separator.getAttribute("aria-valuenow")).toBe("400");
  await act(async () => root.unmount()); host.remove();
});
