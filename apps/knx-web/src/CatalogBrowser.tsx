/** Workbench view for browsing the product catalogue, installing packages, and creating devices. */
import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import * as api from "./api";
import type {
  CatalogInstallReport,
  CatalogItem,
  CatalogManufacturer,
  CreationDiagnostic,
  CreatedCatalogDevice,
} from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";

import { useProductLanguage } from "./productLanguage";
import { LanguageFallbackBadge, fellBack } from "./languageFallback";
import { useTranslate } from "./i18n";
import { useActiveOptionScroll } from "./useActiveOptionScroll";
import { CatalogInstallReportView, describeCreationDiagnostic, newRequestId } from "./CatalogInstallReport";

// T2 (GAP_ANALYSIS_ETS.md) — the device-from-catalog workspace. Feeds T1's
// `Command::CreateDevice` (backend-only since 2026-09-08). The search input is a combobox
// over the results list: ArrowUp/ArrowDown move a highlight and Enter
// picks the highlighted item, pre-filling the name field exactly as
// clicking the row does — it does not create the device, since creation
// stays behind the name field's own Enter/Create.
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
  const [language] = useProductLanguage();
  const [manufacturers, setManufacturers] = useState<CatalogManufacturer[]>([]);
  const [manufacturer, setManufacturer] = useState("");
  const [search, setSearch] = useState("");
  const [items, setItems] = useState<CatalogItem[]>([]);
  const [itemsLoaded, setItemsLoaded] = useState(false);
  const [highlight, setHighlight] = useState(0);
  const [selected, setSelected] = useState<CatalogItem | null>(null);
  const [name, setName] = useState("");
  const [quantity, setQuantity] = useState("1");
  const [error, setError] = useState<string | null>(null);
  const [installReport, setInstallReport] = useState<CatalogInstallReport | null>(null);
  const [diagnostics, setDiagnostics] = useState<CreationDiagnostic[]>([]);
  const [createdItems, setCreatedItems] = useState<CreatedCatalogDevice[]>([]);
  const [installing, setInstalling] = useState(false);
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
  const activeOptionRef = useActiveOptionScroll(highlight, items, active);
  const quantityNumber = Number(quantity);
  const validQuantity = /^\d+$/.test(quantity) && quantityNumber >= 1 && quantityNumber <= 32;

  useEffect(() => {
    if (active) searchRef.current?.focus();
  }, [active]);

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
      <label className="catalog-install">
        {installing ? t("catalog.installing") : t("catalog.installLabel")}
        <input
          type="file"
          accept=".knxprod,application/zip"
          disabled={installing}
          onChange={(e) => {
            const file = e.target.files?.[0];
            if (file) void install(file);
            e.currentTarget.value = "";
          }}
        />
      </label>
      {installReport && <CatalogInstallReportView report={installReport} />}
      <select value={manufacturer} onChange={(e) => changeManufacturer(e.target.value)}>
        <option value="">{t("catalog.allManufacturers")}</option>
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
        placeholder={t("catalog.searchPlaceholder")}
        role="combobox"
        aria-haspopup="listbox"
        aria-expanded={items.length > 0}
        aria-controls="catalog-results"
        aria-activedescendant={items[highlight] ? `catalog-option-${highlight}` : undefined}
      />
      {itemsLoaded && items.length === 0 && <p className="search-empty">{t("catalog.noMatches")}</p>}
      <ul className="search-results" id="catalog-results" role="listbox">
        {items.map((item, i) => (
          <li
            key={item.id}
            id={`catalog-option-${i}`}
            ref={i === highlight ? activeOptionRef : undefined}
            role="option"
            aria-selected={selected?.id === item.id}
            aria-disabled={batchOutcomeUnconfirmed}
            className={`${selected?.id === item.id ? "search-result selected" : "search-result"}${i === highlight ? " active" : ""}${batchOutcomeUnconfirmed ? " disabled" : ""}`}
            onClick={() => pick(item)}
          >
            {item.name ?? item.id}
            {language !== null && fellBack(item.name, item.nameLanguage) && (
              <LanguageFallbackBadge selected={language} source={item.sourceLanguage} />
            )}
            {item.number ? ` (${item.number})` : ""}
            {item.visibleDescription ? ` — ${item.visibleDescription}` : ""}
            {language !== null && fellBack(item.visibleDescription, item.visibleDescriptionLanguage) && (
              <LanguageFallbackBadge selected={language} source={item.sourceLanguage} />
            )}
          </li>
        ))}
      </ul>
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
      {error && <span className="field-error">{error}</span>}
      {retry === "available" && <p className="catalog-retry-hint">{t("catalog.retryHint")}</p>}
    </section>
  );
}
