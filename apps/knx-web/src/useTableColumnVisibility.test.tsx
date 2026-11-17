/** Width preferences validate schemas and synchronize independent mounted consumers. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, expect, it } from "vitest";
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
afterEach(() => { act(() => root.unmount()); host.remove(); });

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
