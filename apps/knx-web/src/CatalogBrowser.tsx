/** Overlay for browsing the product catalogue, installing packages, and creating devices. */
import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import * as api from "./api";
import type {
  CatalogInstallReport,
  CatalogItem,
  CatalogManufacturer,
  CatalogInstallDiagnostic,
  CatalogInstallCount,
  CatalogUnknownConstruct,
  CreationDiagnostic,
} from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import Overlay from "./Overlay";
import { useProductLanguage } from "./productLanguage";
import { useTranslate } from "./i18n";
import type { MessageKey, Translate } from "./i18n";

// D4 exception (task-5 brief): `CreationDiagnostic.detail` is a ready-made
// English sentence composed server-side (`domain.rs`'s `.detail()`), but
// that means it can never be German. Instead of rendering it, this panel
// re-composes the sentence itself in the active UI language from the
// structured fields the server also sends — one catalogue key per `kind`.
// A `kind` this switch has never heard of (a future server variant an old
// frontend build doesn't know about yet) falls back to the server's raw
// `detail` string verbatim rather than rendering nothing — better an
// English sentence slips through once than a blank diagnostic line.
//
// The `!` below (task 5 review, minor): `CreationDiagnostic`'s TS interface
// is flat, so these fields are typed optional even though the server's
// tagged-enum serialization guarantees each is present for the `kind` that
// reads it — `catalogItemId` always accompanies `"programlessProduct"`,
// `refId` always accompanies `"ambiguousDpt"`/`"comObjectRefMissing"`, and
// so on. A `?? ""` fallback here was unreachable dead code, and worse, if
// it ever did fire (a future server bug lying about its own `kind`) it
// would render the sentence with a double space rather than surface the
// contradiction. A non-null assertion is the more honest way to say "this
// is guaranteed", loudly, at the one place it matters.
const categoryMessage: Record<CatalogInstallCount["category"], MessageKey> = {
  archive_member: "catalog.installReport.category.archiveMember",
  product: "catalog.installReport.category.product",
  application_program: "catalog.installReport.category.applicationProgram",
  parameter: "catalog.installReport.category.parameter",
  communication_object: "catalog.installReport.category.communicationObject",
  dynamic_node: "catalog.installReport.category.dynamicNode",
  module: "catalog.installReport.category.module",
  baggage_index: "catalog.installReport.category.baggageIndex",
  baggage: "catalog.installReport.category.baggage",
  unknown_construct: "catalog.installReport.category.unknownConstruct",
  master_section: "catalog.installReport.category.masterSection",
  master_subtree: "catalog.installReport.category.masterSubtree",
  datapoint_type: "catalog.installReport.category.datapointType",
};

function categoryLabel(t: Translate, category: CatalogInstallCount["category"]): string {
  return t(categoryMessage[category]);
}

const dispositionMessage: Record<CatalogInstallCount["disposition"], MessageKey> = {
  read: "catalog.installReport.disposition.read",
  stored: "catalog.installReport.disposition.stored",
  deduplicated: "catalog.installReport.disposition.deduplicated",
  "retained-but-uninterpreted": "catalog.installReport.disposition.retainedButUninterpreted",
  unsupported: "catalog.installReport.disposition.unsupported",
  dropped: "catalog.installReport.disposition.dropped",
};

function dispositionLabel(t: Translate, disposition: CatalogInstallCount["disposition"]): string {
  return t(dispositionMessage[disposition]);
}

const unknownKindMessage: Record<CatalogUnknownConstruct["kind"], MessageKey> = {
  Element: "catalog.installReport.unknownKind.element",
  Attribute: "catalog.installReport.unknownKind.attribute",
};

function unknownKindLabel(t: Translate, kind: CatalogUnknownConstruct["kind"]): string {
  return t(unknownKindMessage[kind]);
}

const diagnosticKindMessage: Record<CatalogInstallDiagnostic["kind"], MessageKey> = {
  "unsupported-master-section": "catalog.installReport.diagnosticKind.unsupportedMasterSection",
  "unsupported-master-subtree": "catalog.installReport.diagnosticKind.unsupportedMasterSubtree",
  "unresolved-baggage-declaration": "catalog.installReport.diagnosticKind.unresolvedBaggageDeclaration",
  "undeclared-baggage-payload": "catalog.installReport.diagnosticKind.undeclaredBaggagePayload",
};

const diagnosticDescriptionMessage: Record<CatalogInstallDiagnostic["kind"], MessageKey> = {
  "unsupported-master-section": "catalog.installReport.diagnostic.unsupportedMasterSection",
  "unsupported-master-subtree": "catalog.installReport.diagnostic.unsupportedMasterSubtree",
  "unresolved-baggage-declaration": "catalog.installReport.diagnostic.unresolvedBaggageDeclaration",
  "undeclared-baggage-payload": "catalog.installReport.diagnostic.undeclaredBaggagePayload",
};

function diagnosticLabel(t: Translate, kind: CatalogInstallDiagnostic["kind"]): string {
  return t(diagnosticKindMessage[kind]);
}

function describeInstallDiagnostic(t: Translate, diagnostic: CatalogInstallDiagnostic): string {
  const description = t(diagnosticDescriptionMessage[diagnostic.kind]);
  return t("catalog.installReport.diagnosticRow", {
    kind: diagnosticLabel(t, diagnostic.kind),
    detail: description,
    occurrences: diagnostic.occurrences,
    archivePath: diagnostic.archivePath,
    xmlPath: diagnostic.xmlPath,
  });
}
function describeCreationDiagnostic(t: Translate, diagnostic: CreationDiagnostic): string {
  switch (diagnostic.kind) {
    case "programlessProduct":
      return t("catalogDiagnostic.programlessProduct", {
        catalogItemId: diagnostic.catalogItemId!,
      });
    case "ambiguousDpt":
      return t("catalogDiagnostic.ambiguousDpt", {
        refId: diagnostic.refId!,
        alternatives: (diagnostic.alternatives ?? []).join(", "),
      });
    case "comObjectRefMissing":
      return t("catalogDiagnostic.comObjectRefMissing", { refId: diagnostic.refId! });
    case "programRefMissing":
      return t("catalogDiagnostic.programRefMissing", { programRef: diagnostic.programRef! });
    case "dynamicOrModuleNotEvaluated":
      return t("catalogDiagnostic.dynamicOrModuleNotEvaluated", {
        programId: diagnostic.programId!,
      });
    default:
      return diagnostic.detail;
  }
}

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
    <Overlay label={t("catalog.title")} onClose={onClose} initialFocusRef={searchRef}>
      <label className="catalog-install">
        {installing ? t("catalog.installing") : t("catalog.installLabel")}
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
        <section className="catalog-report" aria-live="polite" aria-label={t("catalog.installReport.heading")}>
          <p>
            {t("catalog.installReport.summary", {
              status: installReport.skipped
                ? t("catalog.installReport.status.already")
                : t("catalog.installReport.status.new"),
              scheme: installReport.scheme,
              members: t("catalog.installReport.membersCount", { count: installReport.members.length }),
              unknown: t("catalog.installReport.unknownCount", { count: installReport.unknown }),
              conflicts: t("catalog.installReport.conflictsCount", { count: installReport.conflicts }),
            })}
          </p>
          {installReport.facts ? (
            <>
              <h3>{t("catalog.installReport.factsMeasured")}</h3>
              <h4>{t("catalog.installReport.countsHeading")}</h4>
              <ul>
                {installReport.facts.counts.map((row) => (
                  <li key={`${row.category}:${row.disposition}`}>
                    {t("catalog.installReport.countRow", {
                      category: categoryLabel(t, row.category),
                      disposition: dispositionLabel(t, row.disposition),
                      count: row.count,
                    })}
                  </li>
                ))}
              </ul>
              <h4>{t("catalog.installReport.unknownHeading")}</h4>
              <p>
                {t("catalog.installReport.unknownSummary", {
                  distinct: installReport.facts.unknownConstructs.length,
                  occurrences: installReport.facts.unknownOccurrences,
                })}
              </p>
              {installReport.facts.unknownConstructs.length > 0 && (
                <ul>
                  {installReport.facts.unknownConstructs.map((unknown) => (
                    <li key={`${unknown.xpath}:${unknown.kind}:${unknown.name}`}>
                      {t("catalog.installReport.unknownRow", {
                        kind: unknownKindLabel(t, unknown.kind),
                        name: unknown.name,
                        xpath: unknown.xpath,
                        occurrences: unknown.occurrences,
                      })}
                      {unknown.sample ? ` — ${unknown.sample}` : ""}
                    </li>
                  ))}
                </ul>
              )}
              {installReport.facts.diagnostics.length > 0 && (
                <>
                  <h4>{t("catalog.installReport.diagnosticsHeading")}</h4>
                  <ul>
                    {installReport.facts.diagnostics.map((diagnostic) => (
                      <li key={`${diagnostic.kind}:${diagnostic.archivePath}:${diagnostic.xmlPath}`}>
                        {describeInstallDiagnostic(t, diagnostic)}
                      </li>
                    ))}
                  </ul>
                </>
              )}
            </>
          ) : (
            <p>{t("catalog.installReport.factsUnavailable")}</p>
          )}
        </section>
      )}
      {/* KNOWN_LIMITATIONS.md §85: a `.signature` member's `role` starts
          with "Signature" whether or not it was checked — because nothing
          ever checks it. This is the one place a person reads an install
          report; the count exists so "installed cleanly" and "signed and
          verified" cannot be the same sentence in either language. */}
      {installReport && installReport.members.some((member) => member.role.startsWith("Signature")) && (
        <p className="catalog-report catalog-report-caution">
          {t("catalog.installReport.unverifiedSignature", {
            count: installReport.members.filter((member) => member.role.startsWith("Signature")).length,
          })}
        </p>
      )}
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
            placeholder={t("catalog.deviceNamePlaceholder")}
          />
          <button onClick={create} disabled={name.trim() === "" || creating}>
            {creating ? t("catalog.creating") : t("catalog.create")}
          </button>
        </div>
      )}
      {diagnostics.length > 0 && (
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
          <span>{t("catalog.createdWithDiagnostics")}</span>
          <button onClick={onClose}>{t("catalog.done")}</button>
        </div>
      )}
      {error && <span className="field-error">{error}</span>}
    </Overlay>
  );
}
