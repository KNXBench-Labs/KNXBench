/** Read-only projected link matrix with explicit bounded object-column choice. */
import type { GroupAddressWorkspaceViewProps } from "./groupAddressWorkspaceView";
import type { GroupAddressMatrixColumn, GroupAddressMatrixIndex } from "./groupAddressMatrix";
import type { GroupAddressObjectInventory } from "./groupAddressObjectInventory";
import { useTranslate } from "./i18n";
import { useGroupAddressFormat } from "./gaNotation";
import GroupAddressWindowNotice from "./GroupAddressWindowNotice";
import GroupAddressObjectInventoryList from "./GroupAddressObjectInventoryList";

const MAX_COLUMNS = 12;
const DEFAULT_COLUMNS = 6;
export default function GroupAddressMatrix(props: GroupAddressWorkspaceViewProps & {
  matrix: GroupAddressMatrixIndex; inventory: GroupAddressObjectInventory; columnKeys: ReadonlySet<string> | null;
  onColumns: (keys: Set<string>) => void; columnQuery: string; onColumnQuery: (query: string) => void;
}) {
  const t = useTranslate(); const formatGa = useGroupAddressFormat();
  // The default never picks an unlinked object: those columns are chosen explicitly.
  const selected = props.columnKeys ?? new Set(props.matrix.columns.filter(column => column.linked).slice(0, DEFAULT_COLUMNS).map(column => column.key));
  const columns = props.matrix.columns.filter(column => selected.has(column.key));
  const needle = props.columnQuery.trim().toLowerCase();
  const choices = props.matrix.columns.filter(column =>
    [column.key, column.deviceName ?? "", column.deviceAddress ?? "", column.objectName ?? "", String(column.objectNumber)].some(value => value.toLowerCase().includes(needle)),
  );
  const deviceLabel = (column: GroupAddressMatrixColumn) => column.deviceName ?? t("addressTable.unknownDevice", { id: column.deviceId });
  const objectLabel = (column: GroupAddressMatrixColumn) => `${column.objectName ?? t("addressTable.unnamedObject")} #${column.objectNumber}`;
  const rows = Number.isFinite(props.limit) ? props.rows.slice(0, props.limit) : props.rows;
  const visibleOrder = rows.filter(row => row.selectable).map(row => row.node.id);
  return <section data-ga-matrix className="ga-matrix" aria-label={t("gaWorkspace.view.matrix")}>
    <p className="ga-evidence-note" data-ga-matrix-scope>{t(props.matrix.scope === "linked-and-loaded" ? "gaWorkspace.matrixScopeLoaded" : "gaWorkspace.matrixScope")}</p>
    <p className="ga-evidence-note">{t("gaWorkspace.matrixLegend")}</p>
    <p role="status" data-ga-inventory-status>{t("gaWorkspace.inventoryStatus", { ...props.inventory.summary })}</p>
    {props.matrix.ambiguousObjectIds.length > 0 && <p role="status">{t("gaWorkspace.objectOwnerConflict", { ids: props.matrix.ambiguousObjectIds.join(", ") })}</p>}
    {props.matrix.missingFromInventory.length > 0 && <p role="status" data-ga-inventory-mismatch>{t("gaWorkspace.inventoryMismatch", { keys: props.matrix.missingFromInventory.join(", ") })}</p>}
    <details className="ga-column-picker" open>
      <summary>{t("gaWorkspace.columnScope", { selected: columns.length, total: props.matrix.columns.length, limit: MAX_COLUMNS })}</summary>
      <input type="search" aria-label={t("gaWorkspace.objectFilter")} placeholder={t("gaWorkspace.objectFilter")} value={props.columnQuery} onChange={e => props.onColumnQuery(e.target.value)} />
      <div className="ga-column-choices">{choices.map(column => <label key={column.key}>
        <input type="checkbox" data-ga-column={column.key} checked={selected.has(column.key)} disabled={!selected.has(column.key) && columns.length >= MAX_COLUMNS}
          onChange={() => { const next = new Set(columns.map(c => c.key)); if (next.has(column.key)) next.delete(column.key); else next.add(column.key); props.onColumns(next); }} />
        <span>{deviceLabel(column)} · {column.deviceAddress ?? t("gaWorkspace.noDeviceAddress")}
          <small>{objectLabel(column)} · {column.key}</small>
          {!column.linked && <small data-ga-unlinked>{t("gaWorkspace.unlinkedObject")}</small>}</span>
      </label>)}</div>
      <GroupAddressObjectInventoryList inventory={props.inventory} query={needle} />
    </details>
    <div className="workspace-table-wrap"><table className="workspace-table ga-matrix-table">
      <thead><tr><th>{t("addressTable.selectColumn")}</th><th>{t("workbench.address")}</th><th>{t("workbench.name")}</th>
        {columns.map(column => <th key={column.key} scope="col">
          <span>{deviceLabel(column)}</span>
          <small>{objectLabel(column)}</small><small>{column.key}</small>
          {!column.linked && <small data-ga-unlinked>{t("gaWorkspace.unlinkedObject")}</small>}
        </th>)}
      </tr></thead>
      <tbody>{rows.map(row => {
        const ga = row.node;
        const checked = props.multiSelection?.kind === "group_address" && props.multiSelection.ids.has(ga.id);
        const selectedAddress = props.selection?.kind === "group_address" && props.selection.id === ga.id;
        return <tr key={row.key} aria-selected={selectedAddress} className={checked ? "row-multi-selected" : undefined}
          data-rename-kind={row.selectable ? "group_address" : undefined} data-rename-id={row.selectable ? ga.id : undefined}>
          <td><input type="checkbox" disabled={!row.selectable} checked={!!checked} aria-label={`${formatGa(ga.address)} ${ga.name}`}
            onChange={() => props.onItemClick({ shiftKey: false, ctrlKey: true, metaKey: false, preventDefault: () => {} }, "group_address", ga.id, { kind: "group_address", id: ga.id }, visibleOrder)} /></td>
          <th scope="row"><button type="button" className="table-select mono" data-ga-select={ga.id} data-ga-nav="" data-crt-activate="" disabled={!row.selectable}
            onClick={e => props.onItemClick(e, "group_address", ga.id, { kind: "group_address", id: ga.id }, visibleOrder)}>{formatGa(ga.address)}</button></th>
          <td>{ga.name}</td>
          {columns.map(column => {
            const links = props.matrix.cells.get(row.key)?.get(column.key) ?? [];
            const key = `${row.key}|${column.key}`;
            if (links.length === 0) return <td key={column.key} data-ga-absence={key} title={t("gaWorkspace.noProjectedLink")}>—</td>;
            const send = links.some(link => link.direction === "Send");
            const receive = links.some(link => link.direction === "Receive");
            const unknown = links.some(link => link.direction !== "Send" && link.direction !== "Receive");
            const description = `${formatGa(ga.address)} · ${column.key} · ${links.map(link => link.direction).join(" / ")}`;
            return <td key={column.key}><button type="button" className="ga-matrix-cell" data-ga-cell={key} disabled={!row.selectable}
              title={description} aria-label={description} onClick={e => props.onItemClick(e, "group_address", ga.id, { kind: "group_address", id: ga.id }, visibleOrder)}>
              {send && <span>{t("gaWorkspace.symbol.send")}</span>}{receive && <span>{t("gaWorkspace.symbol.receive")}</span>}{unknown && <span>?</span>}
            </button></td>;
          })}
        </tr>;
      })}</tbody>
    </table></div>
    <GroupAddressWindowNotice shown={rows.length} total={props.rows.length} onLimit={props.onLimit} />
    {props.rows.length === 0 && <p role="status">{t("addressTable.noMatches")}</p>}
    {columns.length === 0 && <p role="status">{t("gaWorkspace.noColumns")}</p>}
  </section>;
}
