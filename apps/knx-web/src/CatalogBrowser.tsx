/** Workbench view for browsing the product catalogue, installing packages, and creating devices. */
import { useEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import * as api from "./api";
import type {
  CatalogItem,
  CatalogManufacturer,
  CreationDiagnostic,
  CreatedCatalogDevice,
} from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";

import { useProductLanguage } from "./productLanguage";
import { useTranslate } from "./i18n";
import { useUiLanguage } from "./uiLanguage";
import { describeCreationDiagnostic, newRequestId } from "./CatalogInstallReport";
import CatalogUpdatePanel from "./CatalogUpdatePanel";
import ProductInstallControl from "./ProductInstallControl";
import CatalogResultsTable from "./CatalogResultsTable";
import { DEFAULT_CATALOG_SORT, displayText, productLabel, sortCatalogItems } from "./catalogTableSort";
import { useOptionalCatalogueJobs } from "./CatalogueJobsContext";
import CatalogCleanupDialog, { type CatalogCleanupResult } from "./CatalogCleanupDialog";

// T2 (GAP_ANALYSIS_ETS.md) — the device-from-catalog workspace. Feeds T1's
// `Command::CreateDevice` (backend-only since 2026-09-08). The search field's arrow keys
// move a highlight through the results table's rows
// in their visible (sorted) order and Enter picks the highlighted item,
// pre-filling the name field exactly as selecting the row does — it does not
// create the device, since creation stays behind the name field's own Enter/Create.
// The table keeps native table semantics, so the highlight is announced through a polite
// status region instead of an aria-activedescendant that would point outside the input.
type CreateArgs = [
  lineId: number | null, catalogItemId: string, name: string, quantity: number, requestId: string,
  options: api.CatalogCreateOptions,
];

export default function CatalogBrowser(props: {
  lineId: number | null;
  active?: boolean;
  onCreated: (tree: ProjectTree) => void;
  onClose: () => void;
  /** The `server_incarnation` of the project tree on screen. A retry is only
   * safe while the same server process still holds the request (ADR-0069). */
  serverIncarnation?: string;
  /** ADR-0093: continue with the add-device wizard for the selected item. */
  onWizard?: (item: CatalogItem) => void;
}) {
  const { lineId, active = true, onCreated, onClose, serverIncarnation, onWizard } = props;
  const t = useTranslate();
  const jobs = useOptionalCatalogueJobs();
  const catalogueRevision = jobs?.state.catalogueRevision ?? 0n;
  const [language] = useProductLanguage();
  const [manufacturers, setManufacturers] = useState<CatalogManufacturer[]>([]);
  const [manufacturer, setManufacturer] = useState("");
  const [search, setSearch] = useState("");
  const [items, setItems] = useState<CatalogItem[]>([]);
  const [itemsLoaded, setItemsLoaded] = useState(false);
  const [cleanupOpen, setCleanupOpen] = useState(false);
  // Kept here, not in the dialog: the result outlives a dialog that closes.
  const [cleanupResult, setCleanupResult] = useState<CatalogCleanupResult | null>(null);
  const [uiLanguage] = useUiLanguage();
  const [sort, setSort] = useState(DEFAULT_CATALOG_SORT);
  // The highlight is a catalogue id, so re-sorting keeps it on the same product.
  const [highlightId, setHighlightId] = useState<string | null>(null);
  // Spoken text for the last keyboard move only; re-sorts and clicks leave it alone.
  const [announcement, setAnnouncement] = useState("");
  const [selected, setSelected] = useState<CatalogItem | null>(null);
  const [name, setName] = useState("");
  const [quantity, setQuantity] = useState("1");
  const [error, setError] = useState<string | null>(null);
  const [catalogueReadError, setCatalogueReadError] = useState<string | null>(null);
  const [diagnostics, setDiagnostics] = useState<CreationDiagnostic[]>([]);
  const [createdItems, setCreatedItems] = useState<CreatedCatalogDevice[]>([]);
  const [creating, setCreating] = useState(false);
  const [createdWithDiagnostics, setCreatedWithDiagnostics] = useState(false);
  const [batchOutcomeUnconfirmed, setBatchOutcomeUnconfirmed] = useState(false);
  // MODEL-04: both opt-in; allocation needs a target line to take addresses from.
  const [allocateAddresses, setAllocateAddresses] = useState(false);
  const [uniqueNames, setUniqueNames] = useState(false);
  // DATA-03: the exact request whose outcome is unknown, kept for a resend
  // with the same requestId; `retry` says whether that resend is offered.
  const pendingRef = useRef<{ args: CreateArgs; incarnation: string } | null>(null);
  const [retry, setRetry] = useState<"none" | "available" | "restarted">("none");
  // Guards against a slower, earlier request's response landing after a
  // faster, later one's — the same stale-reply hazard `App.tsx`'s
  // `selectedDeviceIdRef` guards for device selection, applied here to a
  // real server round trip instead of a synchronous re-render.
  const requestIdRef = useRef(0);
  const filtersRef = useRef({ manufacturer: "", search: "" });
  const createInFlightRef = useRef(false);
  const searchRef = useRef<HTMLInputElement>(null);
  const manufacturerNames = useMemo(() => new Map(manufacturers.map((m) => [m.id, displayText(m.name) || m.id])), [manufacturers]);
  // The returned rows are everything the filters matched (the server applies no cap or
  // paging); sorting orders that set in place on a copy and never re-queries.
  const visibleItems = useMemo(() => sortCatalogItems(items, sort, uiLanguage, manufacturerNames),
    [items, sort, uiLanguage, manufacturerNames]);
  const highlight = Math.max(0, visibleItems.findIndex((row) => row.id === highlightId));
  const quantityNumber = Number(quantity);
  const validQuantity = /^\d+$/.test(quantity) && quantityNumber >= 1 && quantityNumber <= 32;

  useEffect(() => {
    if (active) searchRef.current?.focus();
  }, [active]);

  useEffect(() => {
    let current = true;
    api.catalogManufacturers().then(rows => { if (current) setManufacturers(rows); }).catch((e) => { if (current) setCatalogueReadError(api.errorMessage(e)); });
    return () => { current = false; };
  }, [catalogueRevision]);

  // The highlight survives a refresh or filter while its item is still listed.
  useEffect(() => {
    setHighlightId((id) => (id !== null && items.some((row) => row.id === id) ? id : null));
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
          setCatalogueReadError(api.errorMessage(e));
        });
    }, 200);
    return () => { clearTimeout(handle); requestIdRef.current++; };
  }, [manufacturer, search, language, catalogueRevision]);

  function pick(item: CatalogItem) {
    if (batchOutcomeUnconfirmed) return; // Inspect/reload the project before another request.
    setSelected(item);
    setName(item.name ?? "");
    setQuantity("1");
    setError(null);
    setDiagnostics([]);
    setCreatedItems([]);
    setCreatedWithDiagnostics(false);
  }

  function handleSearchKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const index = e.key === "ArrowDown" ? Math.min(highlight + 1, visibleItems.length - 1) : Math.max(highlight - 1, 0);
      const row = visibleItems[index];
      setHighlightId(row?.id ?? null);
      if (row) setAnnouncement(t("catalogTable.highlighted", { name: productLabel(row), index: index + 1, count: visibleItems.length }));
    } else if (e.key === "Enter") {
      const item = visibleItems[highlight];
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

  /** A package or legacy database was installed: reload with the latest filters. */
  // An install adds a catalogue backup; the backups panel reads an open list again.
  const [installRevision, setInstallRevision] = useState(0);
  useEffect(() => { if (catalogueRevision > 0n) setInstallRevision(n => n + 1); },[catalogueRevision]);
  async function reloadAfterProductInstall() {
    setInstallRevision((n) => n + 1);
    await reloadAfterInstall();
  }

  async function reloadAfterInstall() {
    const requestId = ++requestIdRef.current;
    try {
      const filters = filtersRef.current;
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
      setCatalogueReadError(null);
    } catch (e) {
      if (requestId === requestIdRef.current) setCatalogueReadError(api.errorMessage(e));
    }
  }

  async function create() {
    if (!selected || name.trim() === "" || !validQuantity || createInFlightRef.current || createdWithDiagnostics) return;
    await submit([lineId, selected.id, name.trim(), quantityNumber, newRequestId(),
      { allocateAddresses: allocateAddresses && lineId !== null, uniqueNames }], serverIncarnation);
  }

  async function retryPending() {
    const pending = pendingRef.current;
    if (!pending || createInFlightRef.current) return;
    createInFlightRef.current = true;
    setCreating(true);
    let incarnation: string | undefined;
    try {
      incarnation = (await api.currentProject()).server_incarnation;
    } catch (e) {
      // Still unreachable: the record may well survive, keep the offer.
      setError(api.errorMessage(e));
      return;
    } finally {
      createInFlightRef.current = false;
      setCreating(false);
    }
    if (incarnation !== pending.incarnation) {
      // A restarted server forgot the request; resending could add the
      // devices a second time.
      pendingRef.current = null;
      setRetry("restarted");
      setError(t("catalog.retryServerRestarted"));
      return;
    }
    await submit(pending.args, pending.incarnation);
  }

  async function submit(args: CreateArgs, incarnation: string | undefined) {
    const quantityNumber = args[3];
    createInFlightRef.current = true;
    setCreating(true);
    setError(null);
    try {
      const response = await api.createDevice(...args);
      pendingRef.current = null;
      setRetry("none");
      setBatchOutcomeUnconfirmed(false);
      onCreated(response.tree);
      setDiagnostics(response.diagnostics);
      setCreatedItems(response.items ?? []);
      if (quantityNumber > 1 && response.items?.length !== quantityNumber) {
        // An older server may ignore `quantity` and create just one device.
        // The returned tree is real; never retry silently or claim success.
        setError(t("catalog.batchUnsupported"));
        setBatchOutcomeUnconfirmed(true);
        setCreatedWithDiagnostics(true);
      } else if (quantityNumber === 1 && response.diagnostics.length === 0) {
        onClose();
      } else {
        setCreatedWithDiagnostics(true);
      }
    } catch (e) {
      const status = (e as { status?: unknown } | null)?.status;
      if (typeof status !== "number" || status >= 500) {
        // A lost response does not prove the server rolled back. Never send a
        // new request that could make a second batch of the same devices; only
        // the same request (same requestId) may be resent, and only to the
        // same server process (ADR-0069).
        setError(`${api.errorMessage(e)} ${t("catalog.unconfirmedBatch")}`);
        setBatchOutcomeUnconfirmed(true);
        setCreatedWithDiagnostics(true);
        pendingRef.current = incarnation ? { args, incarnation } : null;
        setRetry(incarnation ? "available" : "none");
      } else {
        pendingRef.current = null;
        setRetry("none");
        setError(api.errorMessage(e));
      }
    } finally {
      createInFlightRef.current = false;
      setCreating(false);
    }
  }

  return (
    <section className="catalog-workspace" aria-label={t("catalog.title")} hidden={!active}>
      <header className="catalog-workspace-heading">
        <h1>{t("catalog.title")}</h1>
        <button type="button" onClick={onClose}>{t("catalog.close")}</button>
      </header>
      <ProductInstallControl className="catalog-install" label={t("catalog.installLabel")}
        onInstalled={reloadAfterProductInstall} />
      <CatalogUpdatePanel onInstalled={reloadAfterInstall} installRevision={installRevision} />
      <button type="button" className="catalog-cleanup-open" disabled={creating || batchOutcomeUnconfirmed} onClick={() => setCleanupOpen(true)}>{t("catalog.cleanup.open")}</button>
      {cleanupOpen && active && <CatalogCleanupDialog onClose={() => setCleanupOpen(false)} onRemoved={async (result) => {
        setCleanupResult(result);
        // Only a picked entry that is gone now leaves the creation form.
        if (selected && result.removedItemIds.includes(selected.id)) { setSelected(null); setName(""); }
        await reloadAfterProductInstall();
      }} />}
      {cleanupResult && <p className="catalog-cleanup-notice" role="status">
        {t("catalog.cleanup.done", { count: cleanupResult.removedItemIds.length, backup: cleanupResult.backup })}
      </p>}
      <select value={manufacturer} onChange={(e) => changeManufacturer(e.target.value)}>
        <option value="">{t("catalog.allManufacturers")}</option>
        {manufacturers.map((m) => (
          <option key={m.id} value={m.id}>
            {displayText(m.name) || m.id}
          </option>
        ))}
      </select>
      <input
        ref={searchRef}
        type="search"
        value={search}
        onChange={(e) => changeSearch(e.target.value)}
        onKeyDown={handleSearchKeyDown}
        placeholder={t("catalog.searchPlaceholder")}
        aria-label={t("catalog.searchPlaceholder")}
      />
      <div className="catalog-highlight-status" role="status" aria-live="polite" aria-atomic="true">{active ? announcement : ""}</div>
      {itemsLoaded && items.length === 0 && <p className="search-empty">{t("catalog.noMatches")}</p>}
      <CatalogResultsTable rows={visibleItems} sort={sort} onSort={setSort} manufacturerNames={manufacturerNames}
        language={language} highlightIndex={highlight} selectedId={selected?.id ?? null}
        disabled={batchOutcomeUnconfirmed} active={active} onPick={pick} />
      {selected && !createdWithDiagnostics && (
        <>
          <div className="catalog-create-row">
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") void create();
              }}
              placeholder={t("catalog.deviceNamePlaceholder")}
              aria-label={t("catalog.deviceNamePlaceholder")}
            />
            <label>{t("catalog.quantity")}
              <input type="number" min="1" max="32" step="1" value={quantity}
                aria-label={t("catalog.quantity")} onChange={(e) => setQuantity(e.target.value)} />
            </label>
            <button onClick={create} disabled={name.trim() === "" || !validQuantity || creating}>
              {creating ? t("catalog.creating") : t("catalog.create")}
            </button>
            {onWizard && (
              <button type="button" className="catalog-with-wizard" disabled={creating} onClick={() => onWizard(selected)}>
                {t("catalog.withWizard")}
              </button>
            )}
          </div>
          <fieldset className="catalog-create-options">
            <legend>{t("catalog.optionsLegend")}</legend>
            <label>
              <input type="checkbox" name="allocateAddresses" checked={allocateAddresses && lineId !== null}
                disabled={lineId === null} aria-describedby={lineId === null ? "catalog-allocate-hint" : undefined}
                onChange={(e) => setAllocateAddresses(e.target.checked)} />
              {t("catalog.allocateAddresses")}
            </label>
            {lineId === null && <small id="catalog-allocate-hint">{t("catalog.allocateNeedsLine")}</small>}
            <label>
              <input type="checkbox" name="uniqueNames" checked={uniqueNames}
                onChange={(e) => setUniqueNames(e.target.checked)} />
              {t("catalog.uniqueNames")}
            </label>
          </fieldset>
          <section className="catalog-create-preview" aria-live="polite">
            <h3>{t("catalog.preview")}</h3>
            {!validQuantity ? <p>{t("catalog.quantityInvalid")}</p> : (
              <ul>{Array.from({ length: quantityNumber }, (_, index) => (
                <li key={index}>{quantityNumber === 1 ? name.trim() : `${name.trim()} ${index + 1}`}</li>
              ))}</ul>
            )}
            <p>{lineId === null ? t("catalog.noTargetLine") : t("catalog.targetLine", { lineId })}</p>
            <p>{allocateAddresses && lineId !== null ? t("catalog.addressAllocated") : t("catalog.addressUnassigned")}</p>
            {uniqueNames && <p>{t("catalog.uniqueNamesNote")}</p>}
          </section>
        </>
      )}
      {createdItems.length > 1 ? (
        <section className="catalog-diagnostics" aria-live="polite">
          <h3>{t("catalog.diagnosticsHeading")}</h3>
          <ul>{createdItems.map((item) => (
            <li className="catalog-created-item" key={item.deviceId}>
              <strong>{t("catalog.itemLabel", { index: item.index, name: item.name })}</strong>
              {item.address && <span className="catalog-created-address"> — {t("catalog.itemAddress", { address: item.address })}</span>}
              {item.diagnostics.length > 0 ? (
                <ul>{item.diagnostics.map((diagnostic, index) => (
                  <li key={`${diagnostic.kind}-${index}`}>{describeCreationDiagnostic(t, diagnostic)}</li>
                ))}</ul>
              ) : <span> — {t("catalog.noDiagnostics")}</span>}
            </li>
          ))}</ul>
        </section>
      ) : diagnostics.length > 0 && (
        <section className="catalog-diagnostics" aria-live="polite">
          <h3>{t("catalog.diagnosticsHeading")}</h3>
          <ul>
            {diagnostics.map((diagnostic, index) => (
              <li key={`${diagnostic.kind}-${index}`}>{describeCreationDiagnostic(t, diagnostic)}</li>
            ))}
          </ul>
        </section>
      )}
      {createdWithDiagnostics && (
        <div className="catalog-create-row">
          <span>{error ? t("catalog.creationNeedsReview") : quantityNumber > 1 ? t("catalog.createdMany") : t("catalog.createdWithDiagnostics")}</span>
          {retry === "available" && (
            <button type="button" className="catalog-retry" disabled={creating} onClick={() => void retryPending()}>
              {t("catalog.retrySafely")}
            </button>
          )}
          <button onClick={onClose}>{t("catalog.done")}</button>
        </div>
      )}
      {error && <span className="field-error" role="alert">{error}</span>}
      {catalogueReadError && <span className="field-error" role="alert">{catalogueReadError} {catalogueRevision > 0n && t("catalogJob.refreshDirty")} <button type="button" onClick={() => { setCatalogueReadError(null); void reloadAfterInstall(); }}>{t("catalogJob.retryRefresh")}</button></span>}
      {retry === "available" && <p className="catalog-retry-hint">{t("catalog.retryHint")}</p>}
    </section>
  );
}
