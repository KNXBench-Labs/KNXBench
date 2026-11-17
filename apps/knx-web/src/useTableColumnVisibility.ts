/** Browser-local visible-column sets for the native data tables; one hook per table. */
import { useEffect, useRef, useState } from "react";

export const COLUMN_VISIBILITY_KEY = "knxbench:table-column-visibility:v1:";
const CHANGED = "knxbench:table-column-visibility-changed";

type Visibility = Record<string, boolean>;

function read(tableId: string): Visibility {
  try {
    const value: unknown = JSON.parse(window.localStorage.getItem(COLUMN_VISIBILITY_KEY + tableId) ?? "{}");
    return value !== null && typeof value === "object" && !Array.isArray(value) ? value as Visibility : {};
  } catch { return {}; }
}

/** Unknown ids drop out; every declared id gets an explicit boolean. */
function validated(raw: Visibility, columns: readonly string[]): Visibility {
  return Object.fromEntries(columns.map(id => [id, raw[id] !== false]));
}

/**
 * The columns a table shows, persisted per table id in localStorage. All
 * declared columns are visible until the user hides one, so a new column
 * added by a later release appears by default. At least one column stays
 * visible: hiding the last one is refused and keeps the previous set.
 */
export function useTableColumnVisibility(tableId: string, columns: readonly string[]) {
  const [stored, setStored] = useState(() => ({ tableId, raw: read(tableId) }));
  const latest = useRef(stored);
  const visible = validated(stored.tableId === tableId ? stored.raw : read(tableId), columns);
  useEffect(() => {
    const accept = (raw: Visibility) => { latest.current = { tableId, raw }; setStored(latest.current); };
    const refresh = () => accept(read(tableId));
    const changed = (event: Event) => {
      const detail = (event as CustomEvent<{ tableId: string; raw: Visibility }>).detail;
      if (detail.tableId === tableId) accept(detail.raw);
    };
    const storage = (event: StorageEvent) => {
      if (event.key === null || event.key === COLUMN_VISIBILITY_KEY + tableId) refresh();
    };
    refresh();
    window.addEventListener(CHANGED, changed);
    window.addEventListener("storage", storage);
    return () => { window.removeEventListener(CHANGED, changed); window.removeEventListener("storage", storage); };
  }, [tableId]);
  function publish(next: Visibility) {
    latest.current = { tableId, raw: next };
    setStored(latest.current);
    try { window.localStorage.setItem(COLUMN_VISIBILITY_KEY + tableId, JSON.stringify(next)); } catch { /* Session-only if storage is unavailable. */ }
    window.dispatchEvent(new CustomEvent(CHANGED, { detail: { tableId, raw: next } }));
  }
  const setVisible = (id: string, next: boolean) => {
    if (!columns.includes(id)) return;
    const merged = validated(latest.current.tableId === tableId ? latest.current.raw : read(tableId), columns);
    if (next === false && columns.filter(column => merged[column]).length <= 1) return; // keep at least one column
    publish({ ...merged, [id]: next });
  };
  const reset = () => publish(Object.fromEntries(columns.map(id => [id, true])));
  return { visible, setVisible, reset };
}
