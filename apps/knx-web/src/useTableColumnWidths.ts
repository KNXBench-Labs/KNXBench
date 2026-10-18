/** Browser-local validated column widths, independent of project and server settings. */
import { useEffect, useRef, useState } from "react";

export interface TableColumnWidth { id: string; width: number; min: number; max?: number; fitMin?: number }
export const COLUMN_WIDTH_KEY = "knxbench:table-column-widths:v1:";
const CHANGED = "knxbench:table-column-widths-changed";
export const columnMax = (column: TableColumnWidth) => column.max ?? 1200;
const clamp = (column: TableColumnWidth, width: number) => Math.round(Math.max(column.min, Math.min(columnMax(column), width)));

type Widths = Record<string, number>;
function read(tableId: string): Widths {
  try {
    const value: unknown = JSON.parse(window.localStorage.getItem(COLUMN_WIDTH_KEY + tableId) ?? "{}");
    return value !== null && typeof value === "object" && !Array.isArray(value) ? value as Widths : {};
  } catch { return {}; }
}
function validated(raw: Widths, columns: readonly TableColumnWidth[]): Widths {
  return Object.fromEntries(columns.map(column => {
    const value = Object.hasOwn(raw, column.id) ? raw[column.id] : undefined;
    return [column.id, typeof value === "number" && Number.isFinite(value) && value >= column.min && value <= columnMax(column)
      ? Math.round(value) : column.width];
  }));
}

export function useTableColumnWidths(tableId: string, columns: readonly TableColumnWidth[]) {
  const [stored, setStored] = useState(() => ({ tableId, widths: read(tableId) }));
  const latest = useRef(stored);
  // Derive against today's schema on every render: changing columns cannot
  // briefly apply an old positional width to a new column.
  const widths = validated(stored.tableId === tableId ? stored.widths : read(tableId), columns);
  useEffect(() => {
    const accept = (widths: Widths) => {
      latest.current = { tableId, widths };
      setStored(latest.current);
    };
    const refresh = () => accept(read(tableId));
    const changed = (event: Event) => {
      const detail = (event as CustomEvent<{ tableId: string; widths: Widths }>).detail;
      if (detail.tableId === tableId) accept(detail.widths);
    };
    const storage = (event: StorageEvent) => {
      if (event.key === null || event.key === COLUMN_WIDTH_KEY + tableId) refresh();
    };
    refresh();
    window.addEventListener(CHANGED, changed);
    window.addEventListener("storage", storage);
    return () => { window.removeEventListener(CHANGED, changed); window.removeEventListener("storage", storage); };
  }, [tableId]);
  function publish(next: Widths) {
    latest.current = { tableId, widths: next };
    setStored(latest.current);
    try { window.localStorage.setItem(COLUMN_WIDTH_KEY + tableId, JSON.stringify(next)); } catch { /* Session-only if storage is unavailable. */ }
    window.dispatchEvent(new CustomEvent(CHANGED, { detail: { tableId, widths: next } }));
  }
  function setWidth(id: string, width: number, persist = true) {
    const column = columns.find(column => column.id === id);
    if (!column || !Number.isFinite(width)) return;
    // The synchronous notification updates refs before the next setter runs,
    // including when persistence fails or React batches two instances' edits.
    const current = latest.current.tableId === tableId ? latest.current.widths : read(tableId);
    const retained = Object.fromEntries(Object.entries(current).filter(([, value]) =>
      typeof value === "number" && Number.isFinite(value) && value >= 24 && value <= 12000));
    const next = { ...retained, ...validated(current, columns), [id]: clamp(column, width) };
    if (persist) publish(next);
    else { latest.current = { tableId, widths: next }; setStored(latest.current); }
  }
  return { widths, setWidth, resetColumn: (id: string) => {
    const column = columns.find(column => column.id === id);
    if (column) setWidth(id, column.width);
  }, reset: () => publish(Object.fromEntries(columns.map(column => [column.id, column.width]))) };
}
