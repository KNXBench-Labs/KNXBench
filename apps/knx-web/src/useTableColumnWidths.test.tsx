/** Width preferences validate schemas and synchronize independent mounted consumers. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { COLUMN_WIDTH_KEY, useTableColumnWidths, type TableColumnWidth } from "./useTableColumnWidths";

const columns: TableColumnWidth[] = [{ id: "name", width: 180, min: 100, max: 400 }, { id: "address", width: 140, min: 80 }];
let states: ReturnType<typeof useTableColumnWidths>[];
Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
let host: HTMLDivElement; let root: ReturnType<typeof createRoot>;
function Probe({ index = 0, id = "test", schema = columns }: { index?: number; id?: string; schema?: TableColumnWidth[] }) {
  states[index] = useTableColumnWidths(id, schema); return null;
}
beforeEach(() => { window.localStorage.clear(); states = []; host = document.createElement("div"); root = createRoot(host); });
afterEach(async () => { await act(async () => root.unmount()); vi.restoreAllMocks(); window.localStorage.clear(); });
const render = async (schema = columns) => { await act(async () => root.render(<Probe schema={schema} />)); };
const stored = () => JSON.parse(window.localStorage.getItem(COLUMN_WIDTH_KEY + "test")!);

it("persists widths by stable ID and restores on remount", async () => {
  await render(); await act(async () => states[0].setWidth("name", 260));
  expect(stored()).toEqual({ name: 260, address: 140 });
  await act(async () => root.render(null)); await render();
  expect(states[0].widths).toEqual({ name: 260, address: 140 });
});
it("validates types, finite numbers, ranges, unknown columns and record shape", async () => {
  for (const raw of ['{"name":"250","address":79,"unknown":200}', '{"name":401,"address":1e999}', 'null', '[]', '42']) {
    window.localStorage.setItem(COLUMN_WIDTH_KEY + "test", raw);
    await act(async () => root.render(null)); await render();
    expect(states[0].widths).toEqual({ name: 180, address: 140 });
  }
  await act(async () => states[0].setWidth("name", -100)); expect(states[0].widths.name).toBe(100);
  await act(async () => states[0].setWidth("name", 500)); expect(states[0].widths.name).toBe(400);
  await act(async () => { states[0].setWidth("name", NaN); states[0].setWidth("unknown", 200); });
  expect(stored()).toEqual({ name: 400, address: 140 });
});
it("silently defaults corrupt JSON and overwrites it on the next change", async () => {
  window.localStorage.setItem(COLUMN_WIDTH_KEY + "test", "{broken"); await render();
  expect(states[0].widths.name).toBe(180);
  await act(async () => states[0].setWidth("address", 210)); expect(stored()).toEqual({ name: 180, address: 210 });
});
it("keeps IDs through reordered, added, removed and changed-bound columns", async () => {
  await render(); await act(async () => states[0].setWidth("name", 260));
  await render([columns[1], { id: "new", width: 300, min: 100 }, columns[0]]);
  expect(states[0].widths).toEqual({ address: 140, new: 300, name: 260 });
  await render([{ ...columns[0], min: 280, width: 300 }]);
  expect(states[0].widths).toEqual({ name: 300 });
  await act(async () => states[0].setWidth("name", 320)); expect(stored()).toEqual({ name: 320, address: 140 });
});
it("synchronizes multiple instances without overwriting another column or table", async () => {
  await act(async () => root.render(<><Probe /><Probe index={1} /><Probe index={2} id="other" /></>));
  await act(async () => { states[0].setWidth("name", 260); states[1].setWidth("address", 220); });
  expect(states[0].widths).toEqual({ name: 260, address: 220 });
  expect(states[1].widths).toEqual(states[0].widths);
  expect(states[2].widths).toEqual({ name: 180, address: 140 });
});
it("reset column preserves its neighbour; reset table restores every default", async () => {
  await render(); await act(async () => { states[0].setWidth("name", 260); states[0].setWidth("address", 220); });
  await act(async () => states[0].resetColumn("name")); expect(stored()).toEqual({ name: 180, address: 220 });
  await act(async () => states[0].reset()); expect(stored()).toEqual({ name: 180, address: 140 });
});
it("switches table identity and responds to cross-window updates and clear", async () => {
  await render(); await act(async () => states[0].setWidth("name", 260));
  await act(async () => root.render(<Probe id="other" />)); expect(states[0].widths.name).toBe(180);
  window.localStorage.setItem(COLUMN_WIDTH_KEY + "other", '{"name":310}');
  await act(async () => window.dispatchEvent(new StorageEvent("storage", { key: COLUMN_WIDTH_KEY + "other" })));
  expect(states[0].widths.name).toBe(310);
  window.localStorage.clear();
  await act(async () => window.dispatchEvent(new StorageEvent("storage", { key: null })));
  expect(states[0].widths.name).toBe(180);
});
it("keeps session resizing usable when storage is unavailable", async () => {
  const storage = window.localStorage;
  const deniedRead = vi.spyOn(storage, "getItem").mockImplementation(() => { throw new Error("blocked"); });
  const denied = vi.spyOn(storage, "setItem").mockImplementation(() => { throw new Error("blocked"); });
  await act(async () => root.render(<><Probe /><Probe index={1} /></>));
  await act(async () => states[0].setWidth("name", 260));
  expect(states[0].widths.name).toBe(260);
  await act(async () => states[1].setWidth("address", 220));
  expect(states[0].widths).toEqual({ name: 260, address: 220 });
  expect(states[1].widths).toEqual(states[0].widths);
  expect(deniedRead).toHaveBeenCalled();
  expect(denied).toHaveBeenCalledTimes(2);
  deniedRead.mockRestore(); denied.mockRestore();
});

it("keeps flat-view Channel while a grouped instance edits Name and switches back", async () => {
  const flat = [...columns, { id: "channel", width: 180, min: 80 }];
  await act(async () => root.render(<><Probe schema={flat} /><Probe index={1} schema={columns} /></>));
  await act(async () => states[0].setWidth("channel", 310));
  await act(async () => states[1].setWidth("name", 260));
  expect(stored().channel).toBe(310);
  expect(states[0].widths.channel).toBe(310);
  await render(columns); await render(flat);
  expect(states[0].widths.channel).toBe(310);
});
it("merges valid absent keys and prunes only unreasonable values or a table reset", async () => {
  window.localStorage.setItem(COLUMN_WIDTH_KEY + "test", JSON.stringify({ channel: 310, tiny: -5, huge: 1e12, text: "42" }));
  await render(); await act(async () => states[0].setWidth("name", 240));
  expect(stored()).toEqual({ channel: 310, name: 240, address: 140 });
  await act(async () => states[0].reset()); expect(stored()).toEqual({ name: 180, address: 140 });
});
