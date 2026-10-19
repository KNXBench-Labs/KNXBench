/** Tests for the workbench's keyboard pane resizing and its width clamping. */
// @vitest-environment happy-dom
import { readFileSync } from "node:fs";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect, it } from "vitest";
import ResizablePane from "./ResizablePane";

it("keeps the fixed desktop shell inside the viewport when zoom enlarges the root", () => {
  const css = readFileSync("src/styles.css", "utf8");
  expect(css).toMatch(/\.workbench\s*\{[^}]*height:\s*calc\(100dvh\s*\/\s*var\(--app-ui-scale,\s*1\)\)/);
});

it("keeps diagram device content visible on hover inside clipped topology cards", () => {
  const css = readFileSync("src/styles.css", "utf8");
  expect(css).toMatch(/\.diagram-device:hover:not\(:disabled\)\s*\{[^}]*transform:\s*none/);
  expect(css).toMatch(/\.diagram-device small\s*\{[^}]*display:\s*block/);
  expect(css).toMatch(/\.device-object-count\s*\{[^}]*color:\s*var\(--knx-muted\)/);
});

it("commits the clamped pointer width once on release and keeps keyboard resizing", async () => {
  const host = document.createElement("div"); document.body.append(host);
  const root = createRoot(host);
  const commits: number[] = [];
  await act(async () => root.render(<ResizablePane label="Navigation" side="left" initialWidth={250} min={200} max={400} onWidthCommit={(width) => commits.push(width)}>Tree</ResizablePane>));
  const separator = host.querySelector<HTMLElement>('[role="separator"]')!;
  await act(async () => separator.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, button: 0, clientX: 100 })));
  await act(async () => separator.dispatchEvent(new PointerEvent("pointermove", { bubbles: true, clientX: 800 })));
  expect(separator.getAttribute("aria-valuenow")).toBe("400");
  expect(commits).toEqual([]);
  await act(async () => separator.dispatchEvent(new PointerEvent("pointerup", { bubbles: true, clientX: 800 })));
  expect(commits).toEqual([400]);
  await act(async () => separator.dispatchEvent(new KeyboardEvent("keydown", { key: "Home", bubbles: true })));
  expect(commits).toEqual([400, 200]);
  await act(async () => root.unmount()); host.remove();
});

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

// UISET follow-up: the pointer delta is visual pixels under application zoom,
// the pane width a layout length; keyboard steps stay layout pixels.
it.each([[0.8, "left", 325], [1.5, "left", 290], [0.8, "right", 175], [1.5, "right", 210]] as const)(
  "at scale %s converts a %s pane drag to layout pixels before committing it", async (scale, side, expected) => {
    document.documentElement.style.setProperty("--app-ui-scale", String(scale));
    const host = document.createElement("div"); document.body.append(host);
    const root = createRoot(host);
    const commits: number[] = [];
    try {
      await act(async () => root.render(<ResizablePane label="Pane" side={side} initialWidth={250} min={100} max={400} onWidthCommit={(width) => commits.push(width)}>Tree</ResizablePane>));
      const separator = host.querySelector<HTMLElement>('[role="separator"]')!;
      await act(async () => separator.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, button: 0, clientX: 500 })));
      await act(async () => separator.dispatchEvent(new PointerEvent("pointermove", { bubbles: true, clientX: 560 })));
      expect(separator.getAttribute("aria-valuenow")).toBe(String(expected));
      expect(host.querySelector<HTMLElement>("aside")!.style.width).toBe(`${expected}px`);
      await act(async () => separator.dispatchEvent(new PointerEvent("pointerup", { bubbles: true, clientX: 560 })));
      expect(commits).toEqual([expected]);
      await act(async () => separator.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true })));
      expect(commits).toEqual([expected, expected + (side === "left" ? 16 : -16)]);
    } finally {
      await act(async () => root.unmount()); host.remove();
      document.documentElement.style.removeProperty("--app-ui-scale");
    }
  });
