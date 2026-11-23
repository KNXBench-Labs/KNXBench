/** Visibility preferences recover empty sets without read side effects and sync consumers. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { COLUMN_VISIBILITY_KEY, useTableColumnVisibility } from "./useTableColumnVisibility";

const COLUMNS = ["product", "manufacturer", "number"];
let states: ReturnType<typeof useTableColumnVisibility>[];
Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
let host: HTMLDivElement; let root: ReturnType<typeof createRoot>;
function Probe({ index = 0, id = "test", schema = COLUMNS }: { index?: number; id?: string; schema?: string[] }) {
  states[index] = useTableColumnVisibility(id, schema); return null;
}

beforeEach(() => {
  window.localStorage.clear();
  states = [];
  host = document.createElement("div");
  document.body.appendChild(host);
  root = createRoot(host);
});
afterEach(() => { act(() => root.unmount()); host.remove(); vi.restoreAllMocks(); });

function render(...ids: string[]) {
  act(() => { root.render(<>{ids.map((id, index) => <Probe key={id} index={index} id={id} />)}</>); });
}

it("shows every declared column by default", () => {
  render("t1");
  expect(states[0].visible).toEqual({ product: true, manufacturer: true, number: true });
});

it("hides and re-shows one column and persists the choice", () => {
  render("t1");
  act(() => states[0].setVisible("manufacturer", false));
  expect(states[0].visible.manufacturer).toBe(false);
  expect(JSON.parse(window.localStorage.getItem(COLUMN_VISIBILITY_KEY + "t1") ?? "{}").manufacturer).toBe(false);
  act(() => states[0].setVisible("manufacturer", true));
  expect(states[0].visible.manufacturer).toBe(true);
});

it("refuses to hide the last visible column", () => {
  render("t1");
  act(() => { states[0].setVisible("product", false); states[0].setVisible("manufacturer", false); });
  act(() => states[0].setVisible("number", false));
  expect(states[0].visible).toEqual({ product: false, manufacturer: false, number: true });
});

it("drops unknown stored ids and defaults a newly declared column to visible", () => {
  window.localStorage.setItem(COLUMN_VISIBILITY_KEY + "t1", JSON.stringify({ ghost: false, product: false }));
  act(() => { root.render(<Probe index={0} id="t1" schema={[...COLUMNS, "section"]} />); });
  expect(states[0].visible).toEqual({ product: false, manufacturer: true, number: true, section: true });
});

it("reset restores all columns", () => {
  render("t1");
  act(() => states[0].setVisible("number", false));
  act(() => states[0].reset());
  expect(states[0].visible).toEqual({ product: true, manufacturer: true, number: true });
});

it("ignores malformed stored JSON", () => {
  window.localStorage.setItem(COLUMN_VISIBILITY_KEY + "t1", "{not json");
  render("t1");
  expect(states[0].visible).toEqual({ product: true, manufacturer: true, number: true });
});

it("synchronizes two mounted consumers of the same table", () => {
  render("t1", "t1");
  act(() => states[0].setVisible("number", false));
  expect(states[1].visible.number).toBe(false);
});

const ALL_HIDDEN = { product: false, manufacturer: false, number: false };
const ALL_VISIBLE = { product: true, manufacturer: true, number: true };
const KEY = COLUMN_VISIBILITY_KEY + "t1";

it("normalizes initial all-false preferences and rerenders without storage writes", () => {
  const raw = JSON.stringify({ ...ALL_HIDDEN, ghost: true });
  window.localStorage.setItem(KEY, raw);
  const write = vi.spyOn(window.localStorage, "setItem");
  const remove = vi.spyOn(window.localStorage, "removeItem");
  const clear = vi.spyOn(window.localStorage, "clear");
  render("t1"); render("t1");
  expect(states[0].visible).toEqual(ALL_VISIBLE);
  expect(window.localStorage.getItem(KEY)).toBe(raw);
  expect(write).not.toHaveBeenCalled(); expect(remove).not.toHaveBeenCalled(); expect(clear).not.toHaveBeenCalled();
});

it("refreshes cross-tab all-false, subset, removal and clear without writing back", () => {
  render("t1");
  const write = vi.spyOn(window.localStorage, "setItem");
  const refresh = (key: string | null) => act(() => window.dispatchEvent(new StorageEvent("storage", { key })));
  for (const raw of [ALL_HIDDEN, { ...ALL_HIDDEN, number: true }]) {
    window.localStorage.setItem(KEY, JSON.stringify(raw)); write.mockClear();
    refresh(KEY);
    expect(states[0].visible).toEqual(raw.number ? raw : ALL_VISIBLE);
    expect(write).not.toHaveBeenCalled();
    expect(window.localStorage.getItem(KEY)).toBe(JSON.stringify(raw));
  }
  window.localStorage.removeItem(KEY); refresh(KEY);
  expect(states[0].visible).toEqual(ALL_VISIBLE);
  window.localStorage.setItem(KEY, JSON.stringify({ ...ALL_HIDDEN, number: true })); write.mockClear();
  refresh(KEY); expect(states[0].visible.number).toBe(true); expect(states[0].visible.product).toBe(false);
  window.localStorage.clear(); refresh(null);
  expect(states[0].visible).toEqual(ALL_VISIBLE);
  expect(write).not.toHaveBeenCalled();
});

it("revalidates removed visible columns, new ids, empty schemas and table switches without writes", () => {
  window.localStorage.setItem(KEY, JSON.stringify({ ...ALL_HIDDEN, number: true }));
  window.localStorage.setItem(COLUMN_VISIBILITY_KEY + "t2", JSON.stringify(ALL_HIDDEN));
  const write = vi.spyOn(window.localStorage, "setItem");
  const show = (id: string, schema: string[]) => act(() => root.render(<Probe id={id} schema={schema} />));
  show("t1", COLUMNS); expect(states[0].visible).toEqual({ ...ALL_HIDDEN, number: true });
  show("t1", ["product", "manufacturer"]); expect(states[0].visible).toEqual({ product: true, manufacturer: true });
  show("t1", ["product", "section"]); expect(states[0].visible).toEqual({ product: false, section: true });
  show("t1", []); expect(states[0].visible).toEqual({});
  show("t2", COLUMNS); expect(states[0].visible).toEqual(ALL_VISIBLE);
  show("t1", COLUMNS); expect(states[0].visible).toEqual({ ...ALL_HIDDEN, number: true });
  expect(write).not.toHaveBeenCalled();
});

it("normalizes same-table local refreshes without writes and isolates other tables", () => {
  render("t1", "t2");
  const write = vi.spyOn(window.localStorage, "setItem");
  act(() => window.dispatchEvent(new CustomEvent("knxbench:table-column-visibility-changed", {
    detail: { tableId: "t1", raw: ALL_HIDDEN },
  })));
  expect(states[0].visible).toEqual(ALL_VISIBLE); expect(states[1].visible).toEqual(ALL_VISIBLE);
  expect(write).not.toHaveBeenCalled();
  act(() => states[0].setVisible("product", false));
  expect(states[0].visible).toEqual({ ...ALL_VISIBLE, product: false });
  expect(states[1].visible).toEqual(ALL_VISIBLE);
});

it("persists explicit toggle and reset starting from effective normalized preferences", () => {
  window.localStorage.setItem(KEY, JSON.stringify(ALL_HIDDEN));
  render("t1", "t1");
  act(() => states[0].setVisible("product", false));
  expect(JSON.parse(window.localStorage.getItem(KEY)!)).toEqual({ ...ALL_VISIBLE, product: false });
  expect(states[1].visible).toEqual({ ...ALL_VISIBLE, product: false });
  act(() => states[1].reset());
  expect(JSON.parse(window.localStorage.getItem(KEY)!)).toEqual(ALL_VISIBLE);
  expect(states[0].visible).toEqual(ALL_VISIBLE);
});
