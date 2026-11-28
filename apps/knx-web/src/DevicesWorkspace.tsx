/** Device table with preserved list state, snapshot-bound metadata and a column chooser. */
import ResizableTable from "./ResizableTable";
import TableColumnChooser from "./TableColumnChooser";
import { DEVICE_COMPARISON_COLUMN, DEVICE_SELECT_COLUMN, DEVICE_TABLE_COLUMNS, visibleColumns } from "./tableColumns";
import { useTableColumnVisibility } from "./useTableColumnVisibility";
import { useEffect, useMemo, useState } from "react";
import * as api from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { MultiSelection, Selection } from "./selection";
import type { ItemClickHandler } from "./multiSelection";
import { catalogueMatchesTree, projectDeviceRows, visibleDeviceRows,
  type DeviceCatalog, type DeviceCatalogRow, type DeviceListRow, type DeviceSort } from "./deviceList";
import { useTranslate, type MessageKey } from "./i18n";
import { deviceLabel } from "./devicePresentation";
import { maskHex, systemLabel } from "./deviceSystem";
import { useProductLanguage } from "./productLanguage";
import { useUiLanguage } from "./uiLanguage";
import { fellBack, LanguageFallbackBadge } from "./languageFallback";
import { createProgrammingChips } from "./ProgrammingChips";
import { refreshSystem2Status, useSystem2Status } from "./system2Status";
import { matchSystem2Result } from "./system2Result";
import type { DeviceComparisons } from "./useDeviceComparisons";
import { ComparisonTag } from "./DeviceComparisonDialog";
import "./deviceComparisons.css";

const COMPACT_LABELS: Partial<Record<string, MessageKey>> = {
  address: "devices.addressShort", programming: "devices.programmingShort", manufacturer: "devices.manufacturerShort",
  orderNumber: "devices.orderNumberShort", topology: "devices.topologyShort", building: "devices.buildingShort",
  com_object_count: "devices.objectsShort", description: "devices.descriptionShort",
};
const CHECKBOX_CLICK = { shiftKey: false, ctrlKey: true, metaKey: false, preventDefault: () => {} };

export default function DevicesWorkspace({ tree, active, selection, multiSelection, onItemClick, onCatalogue, comparisons }: {
  tree: ProjectTree; active: boolean; selection: Selection | null;
  multiSelection: MultiSelection | null; onItemClick: ItemClickHandler;
  onCatalogue: (catalogue: DeviceCatalog) => void;
  comparisons: DeviceComparisons;
}) {
  const t = useTranslate();
  const [language] = useUiLanguage();
  const programmingChips = useMemo(() => createProgrammingChips(t, language), [t, language]);
  const [productLanguage] = useProductLanguage();
  // The panel's status lifetime is shared: no per-row request and no table-owned polling.
  // A visit and an explicit catalogue refresh each ask for one coalesced read; a newer
  // publication from the panel (start, terminal, recovery) supersedes any older read.
  const system2Snapshot = useSystem2Status();
  const system2Status = system2Snapshot.status;
  const [refresh, setRefresh] = useState(0);
  useEffect(() => {
    if (active) void refreshSystem2Status();
  }, [active, refresh, tree.server_incarnation, tree.project_incarnation]);
  const system2Result = useMemo(() => matchSystem2Result(system2Status, tree, system2Snapshot.authority), [system2Status, tree, system2Snapshot.authority]);
  const [query, setQuery] = useState("");
  const [pendingOnly, setPendingOnly] = useState(false);
  // Do not build thousands of hidden chip nodes on project open. Once
  // visited, retain them across editor visits with the table's other DOM.
  const [programmingMounted, setProgrammingMounted] = useState(active);
  if (active && !programmingMounted) setProgrammingMounted(true);
  const [sort, setSort] = useState<DeviceSort>({ column: "address", descending: false });
  const { visible } = useTableColumnVisibility("devices", DEVICE_TABLE_COLUMNS.map((column) => column.id));
  const columns = visibleColumns(DEVICE_TABLE_COLUMNS, visible);
  const [metadata, setMetadata] = useState<{ tree: ProjectTree; language: string | null; rows: DeviceCatalogRow[] } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
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
  const rows = useMemo(() => visibleDeviceRows(projectDeviceRows(tree, currentMetadata), query, sort, language, { pendingOnly, systemText, system2Text, system2Rank, label: (row) => deviceLabel(t, row) }),
    [tree, metadata, productLanguage, query, sort, language, pendingOnly, t, system2Result]);
  const visibleOrder = rows.map((row) => row.id);
  const total = useMemo(() => projectDeviceRows(tree, currentMetadata).length, [tree, metadata, productLanguage]);
  /** The visible, localized System-2 chip text of the one bound row (searchable). */
  function system2Text(row: DeviceListRow): string {
    if (!system2Result || row.id !== system2Result.deviceId) return "";
    return `${t(`devices.system2.short.${system2Result.result}`)} ${t(`system2.result.${system2Result.result}`)}`;
  }
  function system2Rank(row: DeviceListRow): number { return system2Result && row.id === system2Result.deviceId ? 1 : 0; }
  /** The declared family (WP-20); an unresolved product, or a row without metadata, shows why like its siblings. */
  function systemText(row: DeviceListRow): string {
    const label = row.catalog ? systemLabel(row.catalog.system) : null;
    return label ? t(label.key, label.params) : resolution(row);
  }
  function resolution(row: DeviceListRow): string {
    switch (row.catalog?.resolution) {
      case "NoReference": return t("devices.noReference");
      case "NoDatabase": return t("devices.noDatabase");
      case "NotInDatabase": return t("devices.notInDatabase");
      case "Unavailable": return t("devices.productLookupFailed");
      case "Resolved": return "—";
      default: return t("devices.notResolved");
    }
  }
  /** The cell content of one sortable column; the two fixed columns are rendered separately. */
  const cell = (row: DeviceListRow, id: string) => {
    switch (id) {
      case "address": return <td key={id} className="mono"><button className="table-select" onClick={(event) => onItemClick(event, "device", row.id, { kind: "device", id: row.id }, visibleOrder, true)}>{row.address ?? t("workbench.unassigned")}</button></td>;
      case "name": return <td key={id}><button className="table-select" onClick={(event) => onItemClick(event, "device", row.id, { kind: "device", id: row.id }, visibleOrder, true)}>{deviceLabel(t, row)}</button></td>;
      case "programming": return <td key={id}>{programmingMounted ? programmingChips(row, row.id === system2Result?.deviceId ? system2Result : null) : null}</td>;
      case "manufacturer": return <td key={id}>{row.catalog?.manufacturerName ?? row.catalog?.manufacturerId ?? resolution(row)}</td>;
      case "product": return <td key={id}>{row.catalog?.productText ?? resolution(row)}
        {productLanguage && fellBack(row.catalog?.productText, row.catalog?.productTextLanguage)
          && <LanguageFallbackBadge selected={productLanguage} source={row.catalog?.productSourceLanguage} />}
      </td>;
      case "orderNumber": return <td key={id}>{row.catalog?.orderNumber ?? resolution(row)}</td>;
      case "system": return <td key={id} title={row.catalog?.system.state === "family" ? t("devices.systemDeclared", { mask: maskHex(row.catalog.system.mask) }) : undefined}>
        {systemText(row)}</td>;
      case "topology": return <td key={id}>{row.topology.length ? row.topology.join(" · ") : t("workbench.unassigned")}</td>;
      case "building": return <td key={id}>{row.building.length ? row.building.join(" · ") : t("workbench.unassigned")}</td>;
      case "com_object_count": return <td key={id}>{row.com_object_count}</td>;
      case "description": return <td key={id}>{row.description ?? "—"}</td>;
      default: return null;
    }
  };
  return <section className="devices-workspace" aria-labelledby="devices-heading">
    <header className="workspace-heading"><div><h1 id="devices-heading">{t("workbench.devices")}</h1>
      <p role="status">{t("devices.count", { shown: rows.length, total })}</p></div></header>
    <div className="devices-toolbar">
      <input type="search" aria-label={t("devices.filter")} placeholder={t("devices.filterHint")}
        value={query} onChange={(event) => setQuery(event.target.value)} />
      <label className="devices-pending-only"><input type="checkbox" checked={pendingOnly}
        onChange={(event) => setPendingOnly(event.target.checked)} />{t("devices.pendingOnly")}</label>
      <button type="button" onClick={() => setRefresh((value) => value + 1)} disabled={loading}>{t("devices.refresh")}</button>
      <button type="button" disabled={comparisons.running || rows.length === 0} onClick={() => comparisons.open(rows)}>{t("compare.all")}</button>
      <TableColumnChooser tableId="devices" tableLabel="tableColumns.name.devices"
        columns={DEVICE_TABLE_COLUMNS.map((column) => ({ id: column.id, label: column.label }))} />
    </div>
    {loading && <p role="status"><span className="task-status-text">{t("devices.loading")}</span></p>}
    {system2Snapshot.failed && <p className="field-error" role="status" data-system2-read-failed>{t("devices.system2.readFailed")}</p>}
    {error && <p className="field-error" role="alert">{t("devices.catalogueFailed", { reason: error })}</p>}
      <ResizableTable scrollClassName="workspace-table-wrap devices-table-scroll" scrollLabel={t("devices.table")} tableId="devices"
        columns={[DEVICE_SELECT_COLUMN, ...columns, DEVICE_COMPARISON_COLUMN]} className="workspace-table devices-table"><thead><tr>
        <th><span className="sr-only">{t("addressTable.selectColumn")}</span></th>
        {columns.map(({ sort: column, label }) => { const sortColumn = column ?? "name"; return <th key={label} aria-sort={sort.column === sortColumn ? (sort.descending ? "descending" : "ascending") : "none"}>
          <button type="button" aria-label={t(label)} title={t(label)} onClick={() => setSort({ column: sortColumn, descending: sort.column === sortColumn && !sort.descending })}>
            <span className="table-column-label">{t(COMPACT_LABELS[sortColumn] ?? label)}</span><span className="table-sort-indicator" aria-hidden="true" style={{ visibility: sort.column === sortColumn ? "visible" : "hidden" }}>{sort.column === sortColumn ? sort.descending ? "↓" : "↑" : ""}</span>
          </button></th>; })}
      <th><span aria-label={t("compare.column")} title={t("compare.column")}>{t("compare.one")}</span></th>
      </tr></thead><tbody>{rows.map((row) => <tr key={row.id} data-rename-kind="device" data-rename-id={row.id} data-rename-value={row.name}
        onClick={(event) => {
          if (event.target instanceof Element && event.target.closest("button, input")) return;
          onItemClick(event, "device", row.id, { kind: "device", id: row.id }, visibleOrder, true);
        }}
        aria-selected={selection?.kind === "device" && selection.id === row.id || multiSelection?.kind === "device" && multiSelection.ids.has(row.id)}>
        <td><input type="checkbox" aria-label={t("devices.select", { name: deviceLabel(t, row) })}
          checked={multiSelection?.kind === "device" && multiSelection.ids.has(row.id) || false}
          onChange={() => onItemClick(CHECKBOX_CLICK, "device", row.id, { kind: "device", id: row.id }, visibleOrder)} /></td>
        {columns.map((column) => cell(row, column.id))}
        <td className="device-comparison-actions"><ComparisonTag entry={comparisons.entry(row.id)} onClick={() => comparisons.inspect(row)} />
          <button type="button" disabled={comparisons.running} onClick={() => comparisons.open([row])}>{t("compare.one")}</button></td>
      </tr>)}</tbody></ResizableTable>
    {rows.length === 0 && <p role="status">{t(total === 0 ? "workbench.noDevices" : "devices.noMatches")}</p>}
  </section>;
}
