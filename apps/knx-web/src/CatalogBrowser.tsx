import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import * as api from "./api";
import type {
  CatalogInstallReport,
  CatalogItem,
  CatalogManufacturer,
  CreationDiagnostic,
} from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import Overlay from "./Overlay";
import { useProductLanguage } from "./productLanguage";

// T2 (GAP_ANALYSIS_ETS.md) — the device-from-catalog browser. Feeds T1's
// `Command::CreateDevice` (backend-only since 2026-09-08). Built on the
// shared `Overlay` shell (T31). The catalog search input is a combobox
// over the results list: ArrowUp/ArrowDown move a highlight and Enter
// picks the highlighted item, pre-filling the name field exactly as
// clicking the row does — it does not create the device, since creation
// stays behind the name field's own Enter/Create.
export default function CatalogBrowser(props: {
  lineId: number | null;
  onCreated: (tree: ProjectTree) => void;
  onClose: () => void;
}) {
  const { lineId, onCreated, onClose } = props;
  const [language] = useProductLanguage();
  const [manufacturers, setManufacturers] = useState<CatalogManufacturer[]>([]);
  const [manufacturer, setManufacturer] = useState("");
  const [search, setSearch] = useState("");
  const [items, setItems] = useState<CatalogItem[]>([]);
  const [itemsLoaded, setItemsLoaded] = useState(false);
  const [highlight, setHighlight] = useState(0);
  const [selected, setSelected] = useState<CatalogItem | null>(null);
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [installReport, setInstallReport] = useState<CatalogInstallReport | null>(null);
  const [diagnostics, setDiagnostics] = useState<CreationDiagnostic[]>([]);
  const [installing, setInstalling] = useState(false);
  const [creating, setCreating] = useState(false);
  const [createdWithDiagnostics, setCreatedWithDiagnostics] = useState(false);
  // Guards against a slower, earlier request's response landing after a
  // faster, later one's — the same stale-reply hazard `App.tsx`'s
  // `selectedDeviceIdRef` guards for device selection, applied here to a
  // real server round trip instead of a synchronous re-render.
  const requestIdRef = useRef(0);
  const filtersRef = useRef({ manufacturer: "", search: "" });
  const createInFlightRef = useRef(false);
  const searchRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    api.catalogManufacturers().then(setManufacturers).catch((e) => setError(api.errorMessage(e)));
  }, []);

  useEffect(() => {
    setHighlight(0);
  }, [items]);

  // Debounced so a fast typist doesn't fire one request per keystroke —
  // `catalogItems` is a real round trip (server-side LIKE query), unlike
  // Search.tsx's synchronous client-side index match.
  useEffect(() => {
    const handle = setTimeout(() => {
      const requestId = ++requestIdRef.current;
      api
        .catalogItems(manufacturer || undefined, search.trim() || undefined, language)
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
  }, [manufacturer, search, language]);

  function pick(item: CatalogItem) {
    setSelected(item);
    setName(item.name ?? "");
    setError(null);
    setDiagnostics([]);
    setCreatedWithDiagnostics(false);
  }

  function handleSearchKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setHighlight((h) => Math.min(h + 1, items.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setHighlight((h) => Math.max(h - 1, 0));
    } else if (e.key === "Enter") {
      const item = items[highlight];
      if (item) pick(item);
    }
  }

  function changeManufacturer(value: string) {
    filtersRef.current = { ...filtersRef.current, manufacturer: value };
    requestIdRef.current += 1;
    setManufacturer(value);
  }

  function changeSearch(value: string) {
    filtersRef.current = { ...filtersRef.current, search: value };
    requestIdRef.current += 1;
    setSearch(value);
  }

  async function install(file: File) {
    setError(null);
    setInstallReport(null);
    setInstalling(true);
    try {
      const report = await api.installProductPackage(file);
      setInstallReport(report);
      const filters = filtersRef.current;
      const requestId = ++requestIdRef.current;
      const [manufacturers, items] = await Promise.all([
        api.catalogManufacturers(),
        api.catalogItems(
          filters.manufacturer || undefined,
          filters.search.trim() || undefined,
          language,
        ),
      ]);
      if (requestId !== requestIdRef.current) return;
      setManufacturers(manufacturers);
      setItems(items);
      setItemsLoaded(true);
    } catch (e) {
      setError(api.errorMessage(e));
    } finally {
      setInstalling(false);
    }
  }

  async function create() {
    if (!selected || name.trim() === "" || createInFlightRef.current || createdWithDiagnostics) return;
    createInFlightRef.current = true;
    setCreating(true);
    setError(null);
    try {
      const response = await api.createDevice(lineId, selected.id, name.trim());
      onCreated(response.tree);
      setDiagnostics(response.diagnostics);
      if (response.diagnostics.length === 0) {
        onClose();
      } else {
        setCreatedWithDiagnostics(true);
      }
    } catch (e) {
      setError(api.errorMessage(e));
    } finally {
      createInFlightRef.current = false;
      setCreating(false);
    }
  }

  return (
    <Overlay label="Device catalog" onClose={onClose} initialFocusRef={searchRef}>
      <label className="catalog-install">
        {installing ? "Installing product database…" : "Install product database"}
        <input
          type="file"
          accept=".knxprod,.vd2,application/zip"
          disabled={installing}
          onChange={(e) => {
            const file = e.target.files?.[0];
            if (file) void install(file);
            e.currentTarget.value = "";
          }}
        />
      </label>
      {installReport && (
        <p className="catalog-report">
          {installReport.skipped ? "Already installed" : "Installed"}: scheme {installReport.scheme}, {installReport.members.length} members, {installReport.unknown} unknown, {installReport.conflicts} conflicts.
        </p>
      )}
      <select value={manufacturer} onChange={(e) => changeManufacturer(e.target.value)}>
        <option value="">All manufacturers</option>
        {manufacturers.map((m) => (
          <option key={m.id} value={m.id}>
            {m.name ?? m.id}
          </option>
        ))}
      </select>
      <input
        ref={searchRef}
        value={search}
        onChange={(e) => changeSearch(e.target.value)}
        onKeyDown={handleSearchKeyDown}
        placeholder="Search catalog items…"
        role="combobox"
        aria-haspopup="listbox"
        aria-expanded={items.length > 0}
        aria-controls="catalog-results"
        aria-activedescendant={items[highlight] ? `catalog-option-${highlight}` : undefined}
      />
      {itemsLoaded && items.length === 0 && <p className="search-empty">No matches.</p>}
      <ul className="search-results" id="catalog-results" role="listbox">
        {items.map((item, i) => (
          <li
            key={item.id}
            id={`catalog-option-${i}`}
            role="option"
            aria-selected={selected?.id === item.id}
            className={selected?.id === item.id ? "search-result selected" : "search-result"}
            onClick={() => pick(item)}
          >
            {item.name ?? item.id}
            {item.number ? ` (${item.number})` : ""}
            {item.visibleDescription ? ` — ${item.visibleDescription}` : ""}
          </li>
        ))}
      </ul>
      {selected && !createdWithDiagnostics && (
        <div className="catalog-create-row">
          <input
            value={name}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") void create();
            }}
            placeholder="Device name"
          />
          <button onClick={create} disabled={name.trim() === "" || creating}>
            {creating ? "Creating…" : "Create"}
          </button>
        </div>
      )}
      {diagnostics.length > 0 && (
        <section className="catalog-diagnostics" aria-live="polite">
          <h3>Creation diagnostics</h3>
          <ul>
            {diagnostics.map((diagnostic, index) => (
              <li key={`${diagnostic.kind}-${index}`}>{diagnostic.detail}</li>
            ))}
          </ul>
        </section>
      )}
      {createdWithDiagnostics && (
        <div className="catalog-create-row">
          <span>Device created with diagnostics.</span>
          <button onClick={onClose}>Done</button>
        </div>
      )}
      {error && <span className="field-error">{error}</span>}
    </Overlay>
  );
}
