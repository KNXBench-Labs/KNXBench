/** Pins one native selection button per catalogue row across visible-column preferences. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import CatalogResultsTable from "./CatalogResultsTable";
import type { CatalogItem } from "./api";
import { CATALOG_TABLE_COLUMNS } from "./tableColumns";
import { COLUMN_VISIBILITY_KEY } from "./useTableColumnVisibility";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
const key = COLUMN_VISIBILITY_KEY + "catalog-results";
const item: CatalogItem = { id: "fixture-1", manufacturerId: "M-1", name: "Alpha Relay", number: "ORD-1",
  visibleDescription: null, productRefId: "P-1", hardware2programRefId: "HP-1",
  nameLanguage: null, visibleDescriptionLanguage: null, sourceLanguage: null };
afterEach(() => window.localStorage.removeItem(key));

it.each(CATALOG_TABLE_COLUMNS.map(column => column.id))(
  "keeps exactly one selection button when %s is the first visible column", async first => {
    const raw = JSON.stringify(Object.fromEntries(CATALOG_TABLE_COLUMNS.map(column => [column.id,
      CATALOG_TABLE_COLUMNS.findIndex(c => c.id === column.id) >= CATALOG_TABLE_COLUMNS.findIndex(c => c.id === first)])));
    window.localStorage.setItem(key, raw);
    const host = document.createElement("div"); document.body.appendChild(host);
    const root = createRoot(host);
    const onPick = vi.fn();
    const render = (disabled: boolean, selectedId: string | null) => act(async () => {
      root.render(<CatalogResultsTable rows={[item]} sort={{ column: "product", descending: false }}
        onSort={vi.fn()} manufacturerNames={new Map([["M-1", "Fictional maker"]])} language={null}
        highlightIndex={0} selectedId={selectedId} disabled={disabled} active={true} onPick={onPick} />);
    });
    try {
      await render(false, null);
      const row = host.querySelector("tbody tr")!;
      const button = row.querySelector<HTMLButtonElement>("button")!;
      expect(row.querySelectorAll("button")).toHaveLength(1);
      expect(button.closest("td")).toBe(row.querySelector("td"));
      expect(button.type).toBe("button");
      expect(button.getAttribute("aria-label") ?? button.textContent).toContain("Alpha Relay");
      expect(button.textContent).toBe(first === "product" ? "Alpha Relay" : first === "manufacturer" ? "Fictional maker" : first === "number" ? "ORD-1" : "—");
      expect(button.getAttribute("aria-pressed")).toBe("false");
      await act(async () => button.click());
      expect(onPick).toHaveBeenCalledExactlyOnceWith(item);
      onPick.mockClear();
      await act(async () => (row.lastElementChild as HTMLElement).click());
      expect(onPick).toHaveBeenCalledExactlyOnceWith(item);
      await render(true, item.id);
      expect(button.disabled).toBe(true);
      expect(button.getAttribute("aria-pressed")).toBe("true");
      onPick.mockClear();
      await act(async () => { button.click(); (row as HTMLElement).click(); });
      expect(onPick).not.toHaveBeenCalled();
      expect(window.localStorage.getItem(key)).toBe(raw);
    } finally {
      await act(async () => root.unmount()); host.remove();
    }
  },
);

it("uses the catalogue id when the hidden product has no usable name", async () => {
  window.localStorage.setItem(key, JSON.stringify({ product: false }));
  const host = document.createElement("div"); const root = createRoot(host);
  try {
    await act(async () => root.render(<CatalogResultsTable rows={[{ ...item, name: "  " }]}
      sort={{ column: "product", descending: false }} onSort={vi.fn()} manufacturerNames={new Map()}
      language={null} highlightIndex={0} selectedId={null} disabled={false} active={false} onPick={vi.fn()} />));
    expect(host.querySelector("tbody button")!.getAttribute("aria-label")).toBe("fixture-1 — M-1");
  } finally { await act(async () => root.unmount()); }
});
