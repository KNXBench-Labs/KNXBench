/** Accessible header resizing isolates gestures from sorting, row selection and drag/drop. */
// @vitest-environment happy-dom
import { act, StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { beforeEach, afterEach, expect, it, vi } from "vitest";
import ResizableTable from "./ResizableTable";
import { resetSettingsForTests } from "./settingsStore";
import { resetUiLanguageForTests } from "./uiLanguage";
const columns = [{ id: "name", width: 180, min: 100, max: 400 }, { id: "address", width: 140, min: 80 }];
let host: HTMLDivElement; let root: ReturnType<typeof createRoot>;
const sort = vi.fn(); const select = vi.fn(); const drag = vi.fn();
beforeEach(async () => {
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
  window.localStorage.clear(); sort.mockClear(); select.mockClear(); drag.mockClear();
  await act(async () => root.render(<StrictMode><ResizableTable tableId="component" columns={columns}>
    <thead><tr>{columns.map(column => <th key={column.id} onClick={sort} draggable onDragStart={drag}><button onClick={sort}>{column.id}</button></th>)}</tr></thead>
    <tbody><tr onClick={select}><td>Alpha</td><td>1.1.1</td></tr></tbody>
  </ResizableTable></StrictMode>));
});
afterEach(async () => { await act(async () => root.unmount()); host.remove(); resetSettingsForTests(); resetUiLanguageForTests(); window.localStorage.clear(); });
const handle = () => host.querySelector<HTMLElement>('[role="separator"]')!;
const width = () => Number(handle().getAttribute("aria-valuenow"));
async function key(key: string, shiftKey = false) { await act(async () => handle().dispatchEvent(new KeyboardEvent("keydown", { key, shiftKey, bubbles: true, cancelable: true }))); }
async function pointer(type: string, x: number, pointerId = 1) { await act(async () => handle().dispatchEvent(new PointerEvent(type, { clientX: x, pointerId, button: 0, bubbles: true, cancelable: true }))); }

it("exposes named vertical separators with current, minimum and maximum values", () => {
  expect(host.querySelectorAll('[role="separator"]')).toHaveLength(2);
  expect(handle().tabIndex).toBe(0); expect(handle().getAttribute("aria-orientation")).toBe("vertical");
  expect(document.getElementById(handle().getAttribute("aria-labelledby")!)?.textContent).toBe("name");
  expect(handle().getAttribute("aria-valuemin")).toBe("100"); expect(handle().getAttribute("aria-valuemax")).toBe("400");
  expect(width()).toBe(180);
});
it("supports arrows, shift steps, bounds and column reset without sorting", async () => {
  handle().focus(); await key("ArrowRight"); expect(width()).toBe(188);
  await key("ArrowRight", true); expect(width()).toBe(228);
  await key("ArrowLeft"); expect(width()).toBe(220);
  await key("Home"); await key("ArrowLeft"); expect(width()).toBe(100);
  await key("End"); await key("ArrowRight"); expect(width()).toBe(400);
  await key("Enter"); expect(width()).toBe(180);
  expect(document.activeElement).toBe(handle()); expect(sort).not.toHaveBeenCalled(); expect(select).not.toHaveBeenCalled();
});
it("captures a pointer, clamps width, and never sorts, drags or selects a row", async () => {
  const capture = vi.fn(); const release = vi.fn(); handle().setPointerCapture = capture; handle().releasePointerCapture = release;
  await pointer("pointerdown", 200); expect(capture).toHaveBeenCalledWith(1);
  await pointer("pointermove", 250, 2); expect(width()).toBe(180);
  await pointer("pointermove", 250); expect(width()).toBe(230);
  expect(host.querySelector("col")?.style.width).toBe("230px");
  await pointer("pointermove", -1000); expect(width()).toBe(100);
  await pointer("pointerup", -1000); expect(release).toHaveBeenCalledWith(1);
  await pointer("pointermove", 500); expect(width()).toBe(100);
  await act(async () => { handle().click(); handle().dispatchEvent(new Event("dragstart", { bubbles: true, cancelable: true })); });
  expect(sort).not.toHaveBeenCalled(); expect(drag).not.toHaveBeenCalled(); expect(select).not.toHaveBeenCalled();
});
it("cancellation or capture loss ends the drag", async () => {
  handle().setPointerCapture = vi.fn();
  await pointer("pointerdown", 100); await pointer("pointermove", 120); expect(width()).toBe(200);
  await pointer("pointercancel", 120); await pointer("pointermove", 160); expect(width()).toBe(200);
  await pointer("pointerdown", 100); await pointer("lostpointercapture", 100); await pointer("pointermove", 160); expect(width()).toBe(200);
});
it("double-click resets a column and the table action resets all columns", async () => {
  await key("ArrowRight", true); expect(width()).toBe(220);
  await act(async () => handle().dispatchEvent(new MouseEvent("dblclick", { bubbles: true, cancelable: true })));
  expect(width()).toBe(180); await key("ArrowRight");
  await act(async () => host.querySelector<HTMLButtonElement>(".table-column-reset")!.click());
  expect(width()).toBe(180); expect(sort).not.toHaveBeenCalled();
  await act(async () => host.querySelector<HTMLButtonElement>("th button")!.click()); expect(sort).toHaveBeenCalled();
});
