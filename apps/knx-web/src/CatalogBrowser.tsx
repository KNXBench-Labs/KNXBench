import { useEffect, useRef, useState } from "react";
import * as api from "./api";
import type {
  CatalogInstallReport,
  CatalogItem,
  CatalogManufacturer,
  CreationDiagnostic,
} from "./api";
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
    setDiagnostics([]);
    setCreatedWithDiagnostics(false);
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
        api.catalogItems(filters.manufacturer || undefined, filters.search.trim() || undefined),
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
    <div className="search-overlay" onClick={onClose}>
      <div className="search-panel" onClick={(e) => e.stopPropagation()}>
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
          autoFocus
          value={search}
          onChange={(e) => changeSearch(e.target.value)}
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
        {selected && !createdWithDiagnostics && (
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
                <li key={`${diagnostic.kind}-${index}`}>
                  {formatDiagnostic(diagnostic)}
                </li>
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
      </div>
    </div>
  );
}

function formatDiagnostic(diagnostic: CreationDiagnostic): string {
  switch (diagnostic.kind) {
    case "programlessProduct":
      return "This product explicitly has no application program; it was created without communication objects.";
    case "ambiguousDpt":
      return `No DPT was inferred for ${diagnostic.refId}; alternatives: ${(diagnostic.alternatives ?? []).join(", ")}.`;
    case "comObjectRefMissing":
      return `Communication-object reference is missing from the installed program: ${diagnostic.refId}.`;
    case "dynamicOrModuleNotEvaluated":
      return `Dynamic and module activation was not evaluated for ${diagnostic.programId}; only static product data was seeded.`;
  }
}
