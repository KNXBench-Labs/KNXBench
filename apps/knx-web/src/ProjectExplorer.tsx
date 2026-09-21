/** Navigation tree for the project's installations, buildings, devices, and group addresses. */
import { useState } from "react";
import * as api from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { AreaNode } from "./bindings/AreaNode";
import type { LineNode } from "./bindings/LineNode";
import type { BuildingNode } from "./bindings/BuildingNode";
import type { DeviceNode } from "./bindings/DeviceNode";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { GroupRangeNode } from "./bindings/GroupRangeNode";
import type { MultiSelection, Selection } from "./selection";
import type { ItemClickHandler } from "./multiSelection";
import { nestGroupRanges, type GroupRangeTreeNode } from "./treeUtils";
import CatalogBrowser from "./CatalogBrowser";
import { useTranslate, type MessageKey, type Translate } from "./i18n";
import { canonicalGroupAddress, useGroupAddressFormat } from "./gaNotation";

// The same discriminant-vs-label lookup `Inspector.tsx`'s
// `buildingPartKindLabel` uses, duplicated rather than shared — both files
// stay in their own edit scope (task 4 brief) and the table is four lines.
// `BUILDING_PART_KINDS` (below) stays the raw discriminant array sent to
// `api.createBuildingPart`; this table only maps that same value to its
// catalogue key for display.
const BUILDING_PART_KIND_KEYS: Record<string, MessageKey> = {
  Building: "buildingPartKind.building",
  Floor: "buildingPartKind.floor",
  Room: "buildingPartKind.room",
  Corridor: "buildingPartKind.corridor",
  DistributionBoard: "buildingPartKind.distributionBoard",
  BuildingPart: "buildingPartKind.buildingPart",
};
function buildingPartKindLabel(t: Translate, kind: string): string {
  const key = BUILDING_PART_KIND_KEYS[kind];
  return key ? t(key) : kind;
}

function TreeNode(props: {
  label: string;
  children?: React.ReactNode;
  selected?: boolean;
  onSelect?: (e: React.MouseEvent) => void;
}) {
  const [open, setOpen] = useState(true);
  const hasChildren = props.children !== undefined;
  const labelClasses = ["tree-label"];
  if (props.selected) labelClasses.push("selected");
  if (props.onSelect) labelClasses.push("selectable");
  // A node with children but no `onSelect` (every non-leaf label except
  // BuildingItem) keeps the old whole-row-toggles behavior. A node with
  // `onSelect` (DeviceItem, BuildingItem) selects on the label instead —
  // BuildingItem still gets its own dedicated chevron to expand/collapse.
  const labelClick = props.onSelect ?? (hasChildren ? () => setOpen(!open) : undefined);
  return (
    <li>
      <span className="tree-row">
        {hasChildren && (
          <button type="button" className="tree-toggle" aria-label={props.label} aria-expanded={open} onClick={() => setOpen(!open)}>
            {open ? "▾" : "▸"}
          </button>
        )}
        <button type="button" className={labelClasses.join(" ")} onClick={labelClick} aria-pressed={props.onSelect ? !!props.selected : undefined} aria-expanded={!props.onSelect && hasChildren ? open : undefined}>
          {props.label}
        </button>
      </span>
      {hasChildren && open && <ul>{props.children}</ul>}
    </li>
  );
}

// `multiSelection`/`onItemClick` are threaded alongside
// `selection`/`onSelect` through every intermediate tree component exactly
// the way that pair already is — `onSelect`'s existing plain-click contract
// is untouched (see `useMultiSelection` in `multiSelection.ts`, which owns
// the state machine this tree and the group-address table share). Only
// `DeviceItem`/`GroupAddressItem` actually call `onItemClick`; every other
// item type ignores it, the same way most item types already ignore
// `onSelect`'s sibling fields they don't need.
type SelectionProps = {
  selection: Selection | null;
  onSelect: (sel: Selection) => void;
  multiSelection: MultiSelection | null;
  onItemClick: ItemClickHandler;
};

function DeviceItem(props: { device: DeviceNode } & SelectionProps) {
  const { device, selection, multiSelection, onItemClick } = props;
  const label = device.address ? `${device.address} ${device.name}` : device.name;
  const sel: Selection = { kind: "device", id: device.id };
  return (
    <TreeNode
      label={label}
      selected={
        (selection?.kind === "device" && selection.id === device.id) ||
        (multiSelection?.kind === "device" && multiSelection.ids.has(device.id))
      }
      onSelect={(e) => onItemClick(e, "device", device.id, sel)}
    />
  );
}

// The topology counterpart of `NewGroupAddressRow`/`NewGroupRangeRow`.
// `medium_ref` (`MediumTypeRefId` — an opaque product reference `knx-core`
// deliberately does not interpret, see `Line`'s own doc comment) has no
// dropdown to pick from for the same reason; `"MT-0"` (ETS's own default
// for twisted-pair) is pre-filled so the common case needs no typing.
function NewLineRow(props: { areaId: number; onCreated: (tree: ProjectTree) => void }) {
  const { areaId, onCreated } = props;
  const t = useTranslate();
  const [name, setName] = useState("");
  const [address, setAddress] = useState("");
  const [mediumRef, setMediumRef] = useState("MT-0");
  const [error, setError] = useState<string | null>(null);
  const canCreate = name.trim() !== "" && address.trim() !== "" && mediumRef.trim() !== "";

  async function create() {
    if (!canCreate) return;
    setError(null);
    try {
      const tree = await api.createLine(areaId, name, Number(address), mediumRef);
      onCreated(tree);
      setName("");
      setAddress("");
    } catch (e) {
      setError(api.errorMessage(e));
    }
  }

  return (
    <li className="tree-new-row">
      <input
        value={address}
        placeholder="1"
        onChange={(e) => setAddress(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") void create();
        }}
      />
      <input
        value={name}
        placeholder={t("explorer.newLinePlaceholder")}
        onChange={(e) => setName(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") void create();
        }}
      />
      <input
        value={mediumRef}
        placeholder="MT-0"
        onChange={(e) => setMediumRef(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") void create();
        }}
      />
      <button onClick={create} disabled={!canCreate}>
        {t("explorer.add")}
      </button>
      {error && <span className="field-error">{error}</span>}
    </li>
  );
}

function NewAreaRow(props: { onCreated: (tree: ProjectTree) => void }) {
  const { onCreated } = props;
  const t = useTranslate();
  const [name, setName] = useState("");
  const [address, setAddress] = useState("");
  const [error, setError] = useState<string | null>(null);
  const canCreate = name.trim() !== "" && address.trim() !== "";

  async function create() {
    if (!canCreate) return;
    setError(null);
    try {
      const tree = await api.createArea(name, Number(address));
      onCreated(tree);
      setName("");
      setAddress("");
    } catch (e) {
      setError(api.errorMessage(e));
    }
  }

  return (
    <li className="tree-new-row">
      <input
        value={address}
        placeholder="1"
        onChange={(e) => setAddress(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") void create();
        }}
      />
      <input
        value={name}
        placeholder={t("explorer.newAreaPlaceholder")}
        onChange={(e) => setName(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") void create();
        }}
      />
      <button onClick={create} disabled={!canCreate}>
        {t("explorer.add")}
      </button>
      {error && <span className="field-error">{error}</span>}
    </li>
  );
}

// The trigger for CatalogBrowser (T2, GAP_ANALYSIS_ETS.md) — opens the
// modal targeting this line (or `null` for the Unassigned bucket below).
// `isFirst`-gated like every other create affordance: `Command::CreateDevice`
// only ever targets `installations[0]`.
function AddDeviceRow(props: { onAdd: () => void }) {
  const t = useTranslate();
  return (
    <li className="tree-new-row">
      <button onClick={props.onAdd}>{t("explorer.addDevice")}</button>
    </li>
  );
}

function LineItem(
  props: { line: LineNode; isFirst: boolean; onAddDevice: (lineId: number) => void } & SelectionProps,
) {
  const { line, isFirst, onAddDevice, selection, onSelect, multiSelection, onItemClick } = props;
  const t = useTranslate();
  return (
    <TreeNode
      label={t("explorer.lineLabel", { address: line.address, name: line.name })}
      selected={selection?.kind === "line" && selection.id === line.id}
      onSelect={() => onSelect({ kind: "line", id: line.id })}
    >
      {line.devices.map((d) => (
        <DeviceItem
          key={d.id}
          device={d}
          selection={selection}
          onSelect={onSelect}
          multiSelection={multiSelection}
          onItemClick={onItemClick}
        />
      ))}
      {isFirst && <AddDeviceRow onAdd={() => onAddDevice(line.id)} />}
    </TreeNode>
  );
}

function AreaItem(
  props: {
    area: AreaNode;
    isFirst: boolean;
    onCreated: (tree: ProjectTree) => void;
    onAddDevice: (lineId: number) => void;
  } & SelectionProps,
) {
  const { area, isFirst, onCreated, onAddDevice, selection, onSelect, multiSelection, onItemClick } =
    props;
  const t = useTranslate();
  return (
    <TreeNode
      label={t("explorer.areaLabel", { address: area.address, name: area.name })}
      selected={selection?.kind === "area" && selection.id === area.id}
      onSelect={() => onSelect({ kind: "area", id: area.id })}
    >
      {area.lines.map((l) => (
        <LineItem
          key={l.id}
          line={l}
          isFirst={isFirst}
          onAddDevice={onAddDevice}
          selection={selection}
          onSelect={onSelect}
          multiSelection={multiSelection}
          onItemClick={onItemClick}
        />
      ))}
      {isFirst && <NewLineRow areaId={area.id} onCreated={onCreated} />}
    </TreeNode>
  );
}

function GroupAddressItem(props: { ga: GroupAddressNode } & SelectionProps) {
  const { ga, selection, multiSelection, onItemClick } = props;
  const formatGa = useGroupAddressFormat();
  const sel: Selection = { kind: "group_address", id: ga.id };
  return (
    <TreeNode
      label={`${formatGa(ga.address)} ${ga.name}`}
      selected={
        (selection?.kind === "group_address" && selection.id === ga.id) ||
        (multiSelection?.kind === "group_address" && multiSelection.ids.has(ga.id))
      }
      onSelect={(e) => onItemClick(e, "group_address", ga.id, sel)}
    />
  );
}

// One of two affordances in the tree that create a domain object rather
// than select one (the other is `NewGroupRangeRow`, below) — kept as an
// inline row rather than a dialog, the same way `AddressField`/`DptField`
// (Inspector.tsx) edit inline rather than popping a modal. Only rendered
// under the first installation (`InstallationItem`'s `isFirst`):
// `Command::apply` only ever targets `installations[0]` (command.rs), so
// this is the only installation the affordance could honestly promise to
// create into. `ranges` is the installation's own flat `group_ranges` list
// (main and middle ranges alike) — an unset selection creates the address
// with no range, same as every group address created before this cycle.
function NewGroupAddressRow(props: {
  ranges: GroupRangeNode[];
  onCreated: (tree: ProjectTree) => void;
}) {
  const { ranges, onCreated } = props;
  const t = useTranslate();
  const formatGa = useGroupAddressFormat();
  const [address, setAddress] = useState("");
  const [name, setName] = useState("");
  const [rangeId, setRangeId] = useState("");
  const [error, setError] = useState<string | null>(null);
  const canCreate = address.trim() !== "" && name.trim() !== "";

  async function create() {
    if (!canCreate) return;
    setError(null);
    try {
      const tree = await api.createGroupAddress(
        name,
        // Both notations accepted whatever is on screen; the API only
        // ever sees the canonical `/` form (`gaNotation.ts`).
        canonicalGroupAddress(address),
        rangeId === "" ? undefined : Number(rangeId),
      );
      onCreated(tree);
      setAddress("");
      setName("");
      setRangeId("");
    } catch (e) {
      setError(api.errorMessage(e));
    }
  }

  return (
    <li className="tree-new-row">
      <input
        value={address}
        placeholder={formatGa("1/1/1")}
        onChange={(e) => setAddress(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") void create();
        }}
      />
      <input
        value={name}
        placeholder={t("explorer.newGroupAddressPlaceholder")}
        onChange={(e) => setName(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") void create();
        }}
      />
      <select value={rangeId} onChange={(e) => setRangeId(e.target.value)}>
        <option value="">{t("explorer.noRange")}</option>
        {ranges.map((r) => (
          <option key={r.id} value={r.id}>
            {formatGa(r.start)}–{formatGa(r.end)} {r.name}
          </option>
        ))}
      </select>
      <button onClick={create} disabled={!canCreate}>
        {t("explorer.add")}
      </button>
      {error && <span className="field-error">{error}</span>}
    </li>
  );
}

// The group-range counterpart of `NewGroupAddressRow`. `parentId` is
// `undefined` when rendered directly under the "Group Ranges" branch
// (creates a main range) and set to a main range's id when rendered under
// that range's own row (creates a middle range) — `GroupRangeItem` never
// nests a third `NewGroupRangeRow` under a middle range, matching the
// reference project's observed two-level depth (`GroupRange`'s own doc
// comment in knx-core), even though the model itself doesn't cap nesting.
function NewGroupRangeRow(props: {
  parentId?: number;
  onCreated: (tree: ProjectTree) => void;
}) {
  const { parentId, onCreated } = props;
  const t = useTranslate();
  const formatGa = useGroupAddressFormat();
  const [name, setName] = useState("");
  const [start, setStart] = useState("");
  const [end, setEnd] = useState("");
  const [error, setError] = useState<string | null>(null);
  const canCreate = name.trim() !== "" && start.trim() !== "" && end.trim() !== "";

  async function create() {
    if (!canCreate) return;
    setError(null);
    try {
      const tree = await api.createGroupRange(
        name,
        canonicalGroupAddress(start),
        canonicalGroupAddress(end),
        parentId,
      );
      onCreated(tree);
      setName("");
      setStart("");
      setEnd("");
    } catch (e) {
      setError(api.errorMessage(e));
    }
  }

  return (
    <li className="tree-new-row">
      <input
        value={start}
        placeholder={formatGa("1/0/0")}
        onChange={(e) => setStart(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") void create();
        }}
      />
      <input
        value={end}
        placeholder={formatGa("1/7/255")}
        onChange={(e) => setEnd(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") void create();
        }}
      />
      <input
        value={name}
        placeholder={
          parentId === undefined
            ? t("explorer.newGroupRangePlaceholder")
            : t("explorer.newMiddleRangePlaceholder")
        }
        onChange={(e) => setName(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") void create();
        }}
      />
      <button onClick={create} disabled={!canCreate}>
        {t("explorer.add")}
      </button>
      {error && <span className="field-error">{error}</span>}
    </li>
  );
}

const BUILDING_PART_KINDS = [
  "Building",
  "Floor",
  "Room",
  "Corridor",
  "DistributionBoard",
  "BuildingPart",
] as const;

// The building-part counterpart of `NewGroupRangeRow`. `parentId` is
// `undefined` when rendered directly under the "Buildings" branch
// (creates a root part) and set to a `BuildingItem`'s own id when
// rendered under that item (creates a nested part) — unlike
// `NewGroupRangeRow`, nesting isn't capped at one level here: every
// `BuildingItem` gets its own row, since `BuildingPart` has no depth
// limit (`building.rs`'s own doc comment), unlike `GroupRange`'s
// observed two-level depth.
function NewBuildingPartRow(props: {
  parentId?: number;
  onCreated: (tree: ProjectTree) => void;
}) {
  const { parentId, onCreated } = props;
  const t = useTranslate();
  const [name, setName] = useState("");
  const [kind, setKind] = useState<(typeof BUILDING_PART_KINDS)[number]>("Room");
  const [error, setError] = useState<string | null>(null);
  const canCreate = name.trim() !== "";

  async function create() {
    if (!canCreate) return;
    setError(null);
    try {
      const tree = await api.createBuildingPart(name, kind, parentId);
      onCreated(tree);
      setName("");
    } catch (e) {
      setError(api.errorMessage(e));
    }
  }

  return (
    <li className="tree-new-row">
      <select value={kind} onChange={(e) => setKind(e.target.value as typeof kind)}>
        {BUILDING_PART_KINDS.map((k) => (
          <option key={k} value={k}>
            {buildingPartKindLabel(t, k)}
          </option>
        ))}
      </select>
      <input
        value={name}
        placeholder={
          parentId === undefined
            ? t("explorer.newBuildingPlaceholder")
            : t("explorer.newBuildingPartPlaceholder")
        }
        onChange={(e) => setName(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") void create();
        }}
      />
      <button onClick={create} disabled={!canCreate}>
        {t("explorer.add")}
      </button>
      {error && <span className="field-error">{error}</span>}
    </li>
  );
}

function BuildingItem(
  props: {
    building: BuildingNode;
    isFirst: boolean;
    onCreated: (tree: ProjectTree) => void;
  } & SelectionProps,
) {
  const { building, isFirst, onCreated, selection, onSelect, multiSelection, onItemClick } = props;
  const t = useTranslate();
  return (
    <TreeNode
      label={t("explorer.buildingLabel", { name: building.name, kind: buildingPartKindLabel(t, building.kind) })}
      selected={selection?.kind === "building_part" && selection.id === building.id}
      onSelect={() => onSelect({ kind: "building_part", id: building.id })}
    >
      {building.children.map((c) => (
        <BuildingItem
          key={c.id}
          building={c}
          isFirst={isFirst}
          onCreated={onCreated}
          selection={selection}
          onSelect={onSelect}
          multiSelection={multiSelection}
          onItemClick={onItemClick}
        />
      ))}
      {building.devices.map((d) => (
        <DeviceItem
          key={d.id}
          device={d}
          selection={selection}
          onSelect={onSelect}
          multiSelection={multiSelection}
          onItemClick={onItemClick}
        />
      ))}
      {isFirst && <NewBuildingPartRow parentId={building.id} onCreated={onCreated} />}
    </TreeNode>
  );
}

// Group ranges aren't multi-selectable (only devices/group addresses are —
// see `MultiSelectionKind`), so this only needs the plain-selection pair,
// not the full `SelectionProps` every device/group-address-bearing
// component threads.
function GroupRangeItem(
  props: {
    node: GroupRangeTreeNode;
    isFirst: boolean;
    onCreated: (tree: ProjectTree) => void;
  } & Pick<SelectionProps, "selection" | "onSelect">,
) {
  const { node, isFirst, onCreated, selection, onSelect } = props;
  const formatGa = useGroupAddressFormat();
  const { range, children } = node;
  return (
    <TreeNode
      label={`${formatGa(range.start)}–${formatGa(range.end)} ${range.name}`}
      selected={selection?.kind === "group_range" && selection.id === range.id}
      onSelect={() => onSelect({ kind: "group_range", id: range.id })}
    >
      {children.map((c) => (
        <GroupRangeItem
          key={c.range.id}
          node={c}
          isFirst={isFirst}
          onCreated={onCreated}
          selection={selection}
          onSelect={onSelect}
        />
      ))}
      {isFirst && range.parent === null && (
        <NewGroupRangeRow parentId={range.id} onCreated={onCreated} />
      )}
    </TreeNode>
  );
}

function InstallationItem(
  props: {
    installation: InstallationNode;
    isFirst: boolean;
    onTreeUpdate: (tree: ProjectTree) => void;
    onAddDevice: (lineId: number | null) => void;
  } & SelectionProps,
) {
  const {
    installation,
    isFirst,
    onTreeUpdate,
    onAddDevice,
    selection,
    onSelect,
    multiSelection,
    onItemClick,
  } = props;
  const t = useTranslate();
  return (
    <TreeNode label={installation.name}>
      <TreeNode label={t("explorer.topology")}>
        {installation.topology.map((a) => (
          <AreaItem
            key={a.id}
            area={a}
            isFirst={isFirst}
            onCreated={onTreeUpdate}
            onAddDevice={onAddDevice}
            selection={selection}
            onSelect={onSelect}
            multiSelection={multiSelection}
            onItemClick={onItemClick}
          />
        ))}
        {isFirst && <NewAreaRow onCreated={onTreeUpdate} />}
      </TreeNode>
      <TreeNode label={t("explorer.buildings")}>
        {installation.buildings.map((b) => (
          <BuildingItem
            key={b.id}
            building={b}
            isFirst={isFirst}
            onCreated={onTreeUpdate}
            selection={selection}
            onSelect={onSelect}
            multiSelection={multiSelection}
            onItemClick={onItemClick}
          />
        ))}
        {isFirst && <NewBuildingPartRow onCreated={onTreeUpdate} />}
      </TreeNode>
      {(installation.unassigned.length > 0 || isFirst) && (
        <TreeNode label={t("explorer.unassigned")}>
          {installation.unassigned.map((d) => (
            <DeviceItem
              key={d.id}
              device={d}
              selection={selection}
              onSelect={onSelect}
              multiSelection={multiSelection}
              onItemClick={onItemClick}
            />
          ))}
          {isFirst && <AddDeviceRow onAdd={() => onAddDevice(null)} />}
        </TreeNode>
      )}
      <TreeNode label={t("explorer.groupAddresses")}>
        {installation.group_addresses.map((ga) => (
          <GroupAddressItem
            key={ga.id}
            ga={ga}
            selection={selection}
            onSelect={onSelect}
            multiSelection={multiSelection}
            onItemClick={onItemClick}
          />
        ))}
        {isFirst && (
          <NewGroupAddressRow ranges={installation.group_ranges} onCreated={onTreeUpdate} />
        )}
      </TreeNode>
      <TreeNode label={t("explorer.groupRanges")}>
        {nestGroupRanges(installation.group_ranges).map((node) => (
          <GroupRangeItem
            key={node.range.id}
            node={node}
            isFirst={isFirst}
            onCreated={onTreeUpdate}
            selection={selection}
            onSelect={onSelect}
          />
        ))}
        {isFirst && <NewGroupRangeRow onCreated={onTreeUpdate} />}
      </TreeNode>
    </TreeNode>
  );
}

export default function ProjectExplorer(
  props: {
    tree: ProjectTree;
    onTreeUpdate: (tree: ProjectTree) => void;
  } & SelectionProps,
) {
  const { tree, onTreeUpdate, selection, onSelect, multiSelection, onItemClick } = props;
  const t = useTranslate();
  // `undefined` = closed; `number | null` = open, targeting that line
  // (or `null` for unassigned) — CatalogBrowser (T2) is only ever opened
  // from the first installation, same restriction every other create
  // affordance here already carries.
  const [catalogTarget, setCatalogTarget] = useState<number | null | undefined>(undefined);

  return (
    <div className="project-explorer" onKeyDown={(e) => {
      if (!(e.target instanceof HTMLButtonElement) || !e.target.matches(".tree-label, .tree-toggle")) return;
      const current = e.target;
      if (e.key === "ArrowRight" || e.key === "ArrowLeft") {
        const toggle = current.closest(".tree-row")?.querySelector<HTMLButtonElement>(".tree-toggle");
        if (toggle && toggle.getAttribute("aria-expanded") !== String(e.key === "ArrowRight")) toggle.click();
        e.preventDefault(); return;
      }
      if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(e.key)) return;
      e.preventDefault();
      const labels = [...e.currentTarget.querySelectorAll<HTMLButtonElement>(".tree-label")];
      const index = labels.indexOf(current);
      labels[e.key === "Home" ? 0 : e.key === "End" ? labels.length - 1 : Math.max(0, Math.min(labels.length - 1, index + (e.key === "ArrowDown" ? 1 : -1)))]?.focus();
    }}>
      <ul className="tree-root">
        <TreeNode
          label={t("explorer.project")}
          selected={selection?.kind === "project"}
          onSelect={() => onSelect({ kind: "project", id: 0 })}
        />
        {tree.installations.map((inst, idx) => (
          <InstallationItem
            key={inst.id}
            installation={inst}
            isFirst={idx === 0}
            onTreeUpdate={onTreeUpdate}
            onAddDevice={setCatalogTarget}
            selection={selection}
            onSelect={onSelect}
            multiSelection={multiSelection}
            onItemClick={onItemClick}
          />
        ))}
      </ul>
      {catalogTarget !== undefined && (
        <CatalogBrowser
          lineId={catalogTarget}
          onCreated={onTreeUpdate}
          onClose={() => setCatalogTarget(undefined)}
        />
      )}
      {(tree.errors > 0 || tree.warnings > 0) && (
        <footer>
          {tree.errors > 0 && (
            <div className="import-errors">
              {t("explorer.importErrorsCount", { count: tree.errors })}
            </div>
          )}
          {tree.warnings > 0 && (
            <div className="import-warnings">
              {t("explorer.importWarningsCount", { count: tree.warnings })}
            </div>
          )}
        </footer>
      )}
    </div>
  );
}
