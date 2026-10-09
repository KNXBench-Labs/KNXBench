/** Device table with preserved list state and snapshot-bound product metadata. */
import { useEffect, useMemo, useState } from "react";
import * as api from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { MultiSelection, Selection } from "./selection";
import type { ItemClickHandler } from "./multiSelection";
import { catalogueMatchesTree, projectDeviceRows, visibleDeviceRows,
  type DeviceCatalog, type DeviceCatalogRow, type DeviceSort, type DeviceSortColumn } from "./deviceList";
import { useTranslate, type MessageKey } from "./i18n";
import { useProductLanguage } from "./productLanguage";
import { useUiLanguage } from "./uiLanguage";
import { fellBack, LanguageFallbackBadge } from "./languageFallback";

const COLUMNS: { column: DeviceSortColumn; label: MessageKey }[] = [
  { column: "address", label: "workbench.address" },
  { column: "name", label: "workbench.name" },
  { column: "manufacturer", label: "devices.manufacturer" },
  { column: "product", label: "devices.product" },
  { column: "orderNumber", label: "devices.orderNumber" },
  { column: "topology", label: "devices.topology" },
  { column: "building", label: "devices.building" },
  { column: "com_object_count", label: "inspector.communicationObjects" },
  { column: "description", label: "devices.description" },
];
const CHECKBOX_CLICK = { shiftKey: false, ctrlKey: true, metaKey: false, preventDefault: () => {} };

export default function DevicesWorkspace({ tree, active, selection, multiSelection, onItemClick, onCatalogue }: {
  tree: ProjectTree; active: boolean; selection: Selection | null;
  multiSelection: MultiSelection | null; onItemClick: ItemClickHandler;
  onCatalogue: (catalogue: DeviceCatalog) => void;
}) {
  const t = useTranslate();
  const [language] = useUiLanguage();
  const [productLanguage] = useProductLanguage();
  const [query, setQuery] = useState("");
  const [sort, setSort] = useState<DeviceSort>({ column: "address", descending: false });
  const [metadata, setMetadata] = useState<{ tree: ProjectTree; language: string | null; rows: DeviceCatalogRow[] } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [refresh, setRefresh] = useState(0);
  useEffect(() => {
    if (!active) return;
    let disposed = false;
    setError(null); setLoading(true);
    api.deviceCatalog(productLanguage).then((catalogue) => {
      if (disposed) return;
      if (!catalogueMatchesTree(catalogue, tree)) throw new Error(t("devices.snapshotChanged"));
      setMetadata({ tree, language: productLanguage, rows: catalogue.devices });
      onCatalogue(catalogue);
      setError(catalogue.problem ? t("devices.productLookupFailed") : null);
    }).catch((reason) => {
      if (!disposed) { setMetadata(null); setError(api.errorMessage(reason)); }
    }).finally(() => { if (!disposed) setLoading(false); });
    return () => { disposed = true; };
  }, [active, tree, productLanguage, refresh]);
  const currentMetadata = metadata?.tree === tree && metadata.language === productLanguage ? metadata.rows : [];
  const rows = useMemo(() => visibleDeviceRows(projectDeviceRows(tree, currentMetadata), query, sort, language),
    [tree, metadata, productLanguage, query, sort, language]);
  const visibleOrder = rows.map((row) => row.id);
  const total = useMemo(() => projectDeviceRows(tree, currentMetadata).length, [tree, metadata, productLanguage]);
  function resolution(row: (typeof rows)[number]): string {
    switch (row.catalog?.resolution) {
      case "NoReference": return t("devices.noReference");
      case "NoDatabase": return t("devices.noDatabase");
      case "NotInDatabase": return t("devices.notInDatabase");
      case "Unavailable": return t("devices.productLookupFailed");
      case "Resolved": return "—";
      default: return t("devices.notResolved");
    }
  }
  return <section className="devices-workspace" aria-labelledby="devices-heading">
    <header className="workspace-heading"><div><h1 id="devices-heading">{t("workbench.devices")}</h1>
      <p role="status">{t("devices.count", { shown: rows.length, total })}</p></div></header>
    <div className="devices-toolbar">
      <input type="search" aria-label={t("devices.filter")} placeholder={t("devices.filterHint")}
        value={query} onChange={(event) => setQuery(event.target.value)} />
      <button type="button" onClick={() => setRefresh((value) => value + 1)} disabled={loading}>{t("devices.refresh")}</button>
    </div>
    {loading && <p role="status">{t("devices.loading")}</p>}
    {error && <p className="field-error" role="alert">{t("devices.catalogueFailed", { reason: error })}</p>}
    <div className="workspace-table-wrap devices-table-scroll" tabIndex={0} aria-label={t("devices.table")}>
      <table className="workspace-table devices-table"><thead><tr>
        <th><span className="sr-only">{t("addressTable.selectColumn")}</span></th>
        {COLUMNS.map(({ column, label }) => <th key={column} aria-sort={sort.column === column ? (sort.descending ? "descending" : "ascending") : "none"}>
          <button type="button" onClick={() => setSort({ column, descending: sort.column === column && !sort.descending })}>
            {t(label)}{sort.column === column && <span aria-hidden="true">{sort.descending ? " ↓" : " ↑"}</span>}
          </button></th>)}
      </tr></thead><tbody>{rows.map((row) => <tr key={row.id}
        onClick={(event) => {
          if (event.target instanceof Element && event.target.closest("button, input")) return;
          onItemClick(event, "device", row.id, { kind: "device", id: row.id }, visibleOrder, true);
        }}
        aria-selected={selection?.kind === "device" && selection.id === row.id || multiSelection?.kind === "device" && multiSelection.ids.has(row.id)}>
        <td><input type="checkbox" aria-label={t("devices.select", { name: row.name })}
          checked={multiSelection?.kind === "device" && multiSelection.ids.has(row.id) || false}
          onChange={() => onItemClick(CHECKBOX_CLICK, "device", row.id, { kind: "device", id: row.id }, visibleOrder)} /></td>
        <td className="mono"><button className="table-select" onClick={(event) => onItemClick(event, "device", row.id, { kind: "device", id: row.id }, visibleOrder, true)}>{row.address ?? t("workbench.unassigned")}</button></td>
        <td><button className="table-select" onClick={(event) => onItemClick(event, "device", row.id, { kind: "device", id: row.id }, visibleOrder, true)}>{row.name}</button></td>
        <td>{row.catalog?.manufacturerName ?? row.catalog?.manufacturerId ?? resolution(row)}</td>
        <td>{row.catalog?.productText ?? resolution(row)}
          {productLanguage && fellBack(row.catalog?.productText, row.catalog?.productTextLanguage)
            && <LanguageFallbackBadge selected={productLanguage} source={row.catalog?.productSourceLanguage} />}
        </td>
        <td>{row.catalog?.orderNumber ?? resolution(row)}</td>
        <td>{row.topology.length ? row.topology.join(" · ") : t("workbench.unassigned")}</td>
        <td>{row.building.length ? row.building.join(" · ") : t("workbench.unassigned")}</td>
        <td>{row.com_object_count}</td><td>{row.description ?? "—"}</td>
      </tr>)}</tbody></table>
    </div>
    {rows.length === 0 && <p role="status">{t(total === 0 ? "workbench.noDevices" : "devices.noMatches")}</p>}
  </section>;
}
