/** Graphical hierarchy over generated projections; no invented physical coordinates. */
import type { ReactNode } from "react";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { BuildingNode } from "./bindings/BuildingNode";
import type { DeviceNode } from "./bindings/DeviceNode";
import type { MultiSelection, Selection } from "./selection";
import type { ItemClickHandler } from "./multiSelection";
import { useTranslate } from "./i18n";
import WorkbenchIcon from "./WorkbenchIcon";
import GroupAddressTable from "./GroupAddressTable";
import { rangePath } from "./groupAddressView";
import { findBuildingPart, flattenBuildingParts } from "./treeUtils";
export type StructureView = "buildings" | "topology" | "addresses";
export default function StructureWorkspace(props: {
  tree: ProjectTree; view: StructureView; selection: Selection | null;
  buildingScope?: number | null;
  onBuildingScope?: (id: number | null) => void;
  // The group-address view's equivalent of `buildingScope`: both are owned
  // by `App`, so a range picked in the tree and the table's breadcrumb can
  // never disagree about what the view is scoped to.
  rangeScope?: number | null;
  onRangeScope?: (id: number | null) => void;
  // One shared multi-selection, owned by `App` (see `multiSelection.ts`) —
  // the address table feeds the same `BulkActionToolbar` the tree does.
  multiSelection: MultiSelection | null;
  onItemClick: ItemClickHandler;
  onTreeUpdate: (tree: ProjectTree) => void;
  // Group-address CSV export/import, mounted by `App` so this view owns no
  // toast or error plumbing of its own. The design inventory puts CSV
  // "directly at the group addresses"; the File menu keeps its copy too,
  // so neither entry point is lost.
  addressActions?: ReactNode;
  onSelect: (selection: Selection) => void; onCatalog: (line: number | null) => void;
}) {
  const { tree, view, selection, onSelect, onCatalog, multiSelection, onItemClick, onTreeUpdate } = props;
  const t = useTranslate();
  const focusedBuilding = view === "buildings" && props.buildingScope != null
    ? findBuildingPart(tree, props.buildingScope) : undefined;
  const scopedRange = view === "addresses" && props.rangeScope != null
    ? tree.installations.flatMap((i) => i.group_ranges).find((r) => r.id === props.rangeScope) : undefined;
  const scopedRangePath = scopedRange
    ? rangePath(tree.installations.flatMap((i) => i.group_ranges), scopedRange.id) : null;
  const selected = (kind: Selection["kind"], id: number) => selection?.kind === kind && selection.id === id;
  function devices(nodes: DeviceNode[]) {
    return <div className="diagram-devices">{nodes.map((device) => <button key={device.id} className="diagram-device" aria-pressed={selected("device", device.id)} onClick={() => onSelect({ kind: "device", id: device.id })}>
      <WorkbenchIcon name="catalog" /><span><strong>{device.name}</strong><small className="mono">{device.address ?? t("workbench.unassigned")}</small></span><span className="device-object-count">{device.com_object_count}</span>
    </button>)}</div>;
  }
  function building(node: BuildingNode) {
    return <section key={node.id} className="building-diagram-node">
      <button className="diagram-heading" aria-pressed={selected("building_part", node.id)} onClick={() => onSelect({ kind: "building_part", id: node.id })}><WorkbenchIcon name="buildings" />{node.name}<small>{node.kind}</small></button>
      {devices(node.devices)}
      {node.children.length > 0 && <div className="building-diagram-children">{node.children.map(building)}</div>}
    </section>;
  }
  function deviceTable(nodes: DeviceNode[]) {
    return <div className="workspace-table-wrap"><table className="workspace-table">
      <thead><tr><th>{t("workbench.device")}</th><th>{t("workbench.address")}</th><th>{t("inspector.communicationObjects")}</th></tr></thead>
      <tbody>{nodes.map((device) => <tr key={device.id} aria-selected={selected("device", device.id)}>
        <td><button className="table-select" onClick={() => onSelect({ kind: "device", id: device.id })}>{device.name}</button></td>
        <td className="mono">{device.address ?? t("workbench.unassigned")}</td><td>{device.com_object_count}</td>
      </tr>)}</tbody>
    </table>{nodes.length === 0 && <p className="empty-device-list" role="status">{t("workbench.noDevices")}</p>}</div>;
  }
  return <section className="structure-workspace" onKeyDown={(e) => {
    if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(e.key) || !(e.target instanceof HTMLButtonElement)) return;
    const buttons = [...e.currentTarget.querySelectorAll<HTMLButtonElement>("button:not(:disabled)")];
    const index = buttons.indexOf(e.target); if (index < 0) return;
    e.preventDefault();
    buttons[e.key === "Home" ? 0 : e.key === "End" ? buttons.length - 1 : Math.max(0, Math.min(buttons.length - 1, index + (e.key === "ArrowDown" ? 1 : -1)))]?.focus();
  }}>
    <header className="workspace-heading"><div><p className="eyebrow">{focusedBuilding?.path ?? (scopedRangePath ? `${t("workbench.addresses")} / ${scopedRangePath}` : tree.installations.map((i) => i.name).join(" / "))}</p><h1>{focusedBuilding?.node.name ?? scopedRange?.name ?? t(`workbench.${view}`)}</h1></div>
      {view === "addresses" ? props.addressActions : <button onClick={() => onCatalog(selection?.kind === "line" ? selection.id : null)}>+ {t("workbench.device")}</button>}
    </header>
    {focusedBuilding && <button className="building-overview-button" onClick={() => props.onBuildingScope?.(null)}>← {t("workbench.buildings")}</button>}
    {scopedRange && <button className="address-overview-button" onClick={() => props.onRangeScope?.(null)}>← {t("workbench.addresses")}</button>}
    {tree.installations.length === 0 && <p role="status">{t("workbench.emptyStructure")}</p>}
    {tree.installations.filter((installation) => !focusedBuilding || flattenBuildingParts(installation.buildings, []).some(({node}) => node.id === focusedBuilding.node.id)).map((installation) => <section key={installation.id} className="installation-diagram" aria-label={installation.name}>
      {view === "topology" && <>
        {installation.topology.length === 0 && <p role="status">{t("workbench.emptyStructure")}</p>}
        {installation.topology.map((area) => <section className="topology-area" key={area.id}>
          <button className="diagram-heading" aria-pressed={selected("area", area.id)} onClick={() => onSelect({ kind: "area", id: area.id })}><WorkbenchIcon name="topology" />{area.address} · {area.name}</button>
          <div className="topology-lines">{area.lines.map((line) => <section className="topology-line" key={line.id}>
            <div className="diagram-line-heading"><button className="diagram-heading" aria-pressed={selected("line", line.id)} onClick={() => onSelect({ kind: "line", id: line.id })}>{area.address}.{line.address} · {line.name}</button>
              <button data-catalog-line={line.id} aria-label={`${t("workbench.device")} · ${line.name}`} onClick={() => onCatalog(line.id)}>+</button></div>
            {devices(line.devices)}
          </section>)}</div>
        </section>)}
        {installation.unassigned.length > 0 && <section className="topology-line"><h2>{t("workbench.unassigned")}</h2>{devices(installation.unassigned)}</section>}
      </>}
      {view === "buildings" && <>
        {installation.buildings.length === 0 && <p role="status">{t("workbench.emptyStructure")}</p>}
        {focusedBuilding ? <>
          {deviceTable(focusedBuilding.node.devices)}
          <div className="building-diagram scoped-building-children">{focusedBuilding.node.children.map(building)}</div>
        </> : <div className="building-diagram">{installation.buildings.map(building)}</div>}
      </>}
      {view === "addresses" && <GroupAddressTable installation={installation} selection={selection}
        multiSelection={multiSelection} onItemClick={onItemClick} onTreeUpdate={onTreeUpdate}
        rangeScope={props.rangeScope ?? null} />}
    </section>)}
  </section>;
}
