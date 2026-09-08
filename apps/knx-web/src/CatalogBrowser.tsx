import { useEffect, useRef, useState } from "react";
import * as api from "./api";
import type { CatalogItem, CatalogManufacturer } from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";

// T2 (GAP_ANALYSIS_ETS.md) — the device-from-catalog browser. Feeds T1's
// `Command::CreateDevice` (backend-only since 2026-09-08). Reuses
// `.search-overlay`/`.search-panel`, the modal shape already shared by
// `Search.tsx` and `CommandPalette.tsx`. Click-only selection — no
// arrow-key nav, a deliberate scope cut (unlike Search.tsx) since this
// modal's own text field already needs Enter for "create", not "navigate".
export default function CatalogBrowser(props: {
  lineId: number | null;
  onCreated: (tree: ProjectTree) => void;
  onClose: () => void;
}) {
  const { lineId, onCreated, onClose } = props;
  const [manufacturers, setManufacturers] = useState<CatalogManufacturer[]>([]);
  const [manufacturer, setManufacturer] = useState("");
  const [search, setSearch] = useState("");
  const [items, setItems] = useState<CatalogItem[]>([]);
  const [itemsLoaded, setItemsLoaded] = useState(false);
  const [selected, setSelected] = useState<CatalogItem | null>(null);
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);
  // Guards against a slower, earlier request's response landing after a
  // faster, later one's — the same stale-reply hazard `App.tsx`'s
  // `selectedDeviceIdRef` guards for device selection, applied here to a
  // real server round trip instead of a synchronous re-render.
  const requestIdRef = useRef(0);

  useEffect(() => {
    api.catalogManufacturers().then(setManufacturers).catch((e) => setError(api.errorMessage(e)));
  }, []);

  // Debounced so a fast typist doesn't fire one request per keystroke —
  // `catalogItems` is a real round trip (server-side LIKE query), unlike
  // Search.tsx's synchronous client-side index match.
  useEffect(() => {
    const handle = setTimeout(() => {
      const requestId = ++requestIdRef.current;
      api
        .catalogItems(manufacturer || undefined, search.trim() || undefined)
        .then((rows) => {
          if (requestId !== requestIdRef.current) return;
          setItems(rows);
          setItemsLoaded(true);
        })
        .catch((e) => {
          if (requestId !== requestIdRef.current) return;
          setError(api.errorMessage(e));
        });
    }, 200);
    return () => clearTimeout(handle);
  }, [manufacturer, search]);

  function pick(item: CatalogItem) {
    setSelected(item);
    setName(item.name ?? "");
    setError(null);
  }

  async function create() {
    if (!selected || name.trim() === "") return;
    setError(null);
    try {
      const tree = await api.createDevice(lineId, selected.id, name.trim());
      onCreated(tree);
      onClose();
    } catch (e) {
      setError(api.errorMessage(e));
    }
  }

  return (
    <div className="search-overlay" onClick={onClose}>
      <div className="search-panel" onClick={(e) => e.stopPropagation()}>
        <select value={manufacturer} onChange={(e) => setManufacturer(e.target.value)}>
          <option value="">All manufacturers</option>
          {manufacturers.map((m) => (
            <option key={m.id} value={m.id}>
              {m.name ?? m.id}
            </option>
          ))}
        </select>
        <input
          autoFocus
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape") onClose();
          }}
          placeholder="Search catalog items…"
        />
        {itemsLoaded && items.length === 0 && <p className="search-empty">No matches.</p>}
        <ul className="search-results">
          {items.map((item) => (
            <li
              key={item.id}
              className={selected?.id === item.id ? "search-result selected" : "search-result"}
              onClick={() => pick(item)}
            >
              {item.name ?? item.id}
              {item.number ? ` (${item.number})` : ""}
              {item.visibleDescription ? ` — ${item.visibleDescription}` : ""}
            </li>
          ))}
        </ul>
        {selected && (
          <div className="catalog-create-row">
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") void create();
                if (e.key === "Escape") onClose();
              }}
              placeholder="Device name"
            />
            <button onClick={create} disabled={name.trim() === ""}>
              Create
            </button>
          </div>
        )}
        {error && <span className="field-error">{error}</span>}
      </div>
    </div>
  );
}
