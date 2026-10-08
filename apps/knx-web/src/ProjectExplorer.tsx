/** Navigation tree for the project's installations, buildings, devices, and group addresses. */
import { createContext, useContext, useEffect, useRef, useState } from "react";
import * as api from "./api";
import { emitAchievementEvent } from "./achievementEvents";
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
import { deviceInstallations, nestGroupRanges, owningInstallation, type GroupRangeTreeNode } from "./treeUtils";
import type { DeviceWizardTarget } from "./deviceWizardPlacement";
import { useTranslate, type MessageKey, type Translate } from "./i18n";
import { canonicalGroupAddress, useGroupAddressFormat } from "./gaNotation";
import { writeDraggedGroupAddress } from "./groupAddressDrag";

const DEVICE_DRAG_MIME = "application/x-knxbench-device-id";

type DragSource = { deviceId: number };

function parseDraggedDevice(dataTransfer: DataTransfer): number | null {
  if (!Array.from(dataTransfer.types).includes(DEVICE_DRAG_MIME)) return null;
  const raw = dataTransfer.getData(DEVICE_DRAG_MIME);
  if (!/^[1-9]\d*$/.test(raw)) return null;
  const id = Number(raw);
  return Number.isSafeInteger(id) ? id : null;
}

function acceptsDraggedDevice(
  dataTransfer: DataTransfer,
  dragSource: DragSource | null,
  eligibleDeviceIds: ReadonlySet<number>,
): boolean {
  return Array.from(dataTransfer.types).includes(DEVICE_DRAG_MIME)
    && dragSource !== null
    && eligibleDeviceIds.has(dragSource.deviceId);
}

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
  Stairway: "buildingPartKind.stairway",
  RoomPart: "buildingPartKind.roomPart",
  Area: "buildingPartKind.area",
  Ground: "buildingPartKind.ground",
  Segment: "buildingPartKind.segment",
};
function buildingPartKindLabel(t: Translate, kind: string): string {
  const key = BUILDING_PART_KIND_KEYS[kind];
  return key ? t(key) : kind;
}

const RevealCompletionContext = createContext<((generation: number) => void) | undefined>(undefined);

function TreeNode(props: {
  label: string;
  children?: React.ReactNode;
  selected?: boolean;
  onSelect?: (e: React.MouseEvent) => void;
  draggable?: boolean;
  dragging?: boolean;
  onDragStart?: React.DragEventHandler<HTMLButtonElement>;
  onDragEnd?: React.DragEventHandler<HTMLButtonElement>;
  dropReady?: boolean;
  onDragOver?: React.DragEventHandler<HTMLButtonElement>;
  onDrop?: React.DragEventHandler<HTMLButtonElement>;
  revealGeneration?: number;
  scrollOnReveal?: boolean;
}) {
  const [open, setOpen] = useState(true);
  const labelRef = useRef<HTMLButtonElement>(null);
  const onRevealComplete = useContext(RevealCompletionContext);
  useEffect(() => {
    if (props.revealGeneration !== undefined) setOpen(true);
  }, [props.revealGeneration]);
  useEffect(() => {
    if (props.scrollOnReveal && props.revealGeneration !== undefined) {
      labelRef.current?.scrollIntoView({ block: "nearest" });
      onRevealComplete?.(props.revealGeneration);
    }
  }, [onRevealComplete, props.revealGeneration, props.scrollOnReveal]);
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
        <button
          ref={labelRef}
          type="button"
          className={labelClasses.join(" ")}
          data-crt-surface="tree"
          data-crt-activate={labelClick ? "" : undefined}
          onClick={labelClick}
          aria-pressed={props.onSelect ? !!props.selected : undefined}
          aria-expanded={!props.onSelect && hasChildren ? open : undefined}
          draggable={props.draggable || undefined}
          data-dragging={props.dragging ? "true" : undefined}
          data-drop-ready={props.dropReady ? "true" : undefined}
          onDragStart={props.onDragStart}
          onDragEnd={props.onDragEnd}
          onDragOver={props.onDragOver}
          onDrop={props.onDrop}
        >
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

type RevealRequest = { selection: Selection; generation: number };

function selectionIs(request: RevealRequest | null | undefined, kind: Selection["kind"], id: number): boolean {
  return request?.selection.kind === kind && request.selection.id === id;
}

function revealGeneration(
  request: RevealRequest | null | undefined,
  containsSelection: boolean,
): number | undefined {
  return containsSelection ? request?.generation : undefined;
}

function lineContainsSelection(line: LineNode, request: RevealRequest | null | undefined): boolean {
  return selectionIs(request, "line", line.id)
    || (request?.selection.kind === "device" && line.devices.some((device) => device.id === request.selection.id));
}

function areaContainsSelection(area: AreaNode, request: RevealRequest | null | undefined): boolean {
  return selectionIs(request, "area", area.id) || area.lines.some((line) => lineContainsSelection(line, request));
}

type BuildingDeviceRevealTarget = { buildingId: number; deviceId: number };

function findBuildingDevice(
  buildings: BuildingNode[],
  deviceId: number,
): BuildingDeviceRevealTarget | undefined {
  for (const building of buildings) {
    if (building.devices.some((device) => device.id === deviceId)) {
      return { buildingId: building.id, deviceId };
    }
    const found = findBuildingDevice(building.children, deviceId);
    if (found) return found;
  }
  return undefined;
}

function buildingContainsSelection(
  building: BuildingNode,
  request: RevealRequest | null | undefined,
  deviceTarget?: BuildingDeviceRevealTarget,
): boolean {
  return selectionIs(request, "building_part", building.id)
    || deviceTarget?.buildingId === building.id
    || building.children.some((child) => buildingContainsSelection(child, request, deviceTarget));
}

function groupRangeContainsSelection(
  node: GroupRangeTreeNode,
  request: RevealRequest | null | undefined,
): boolean {
  return selectionIs(request, "group_range", node.range.id)
    || node.children.some((child) => groupRangeContainsSelection(child, request));
}

type DeviceDragProps = {
  eligibleDeviceIds: ReadonlySet<number>;
  dragSource: DragSource | null;
  onDeviceDragStart: (device: DeviceNode, event: React.DragEvent<HTMLButtonElement>) => void;
  onDeviceDragEnd: () => void;
};

type LineDropProps = {
  onDeviceDropOnLine: (
    line: LineNode,
    event: React.DragEvent<HTMLButtonElement>,
  ) => void;
};

type BuildingDropProps = {
  onDeviceDropOnBuildingPart: (
    building: BuildingNode,
    event: React.DragEvent<HTMLButtonElement>,
  ) => void;
};

function DeviceItem(props: {
  device: DeviceNode;
  revealGeneration?: number;
  scrollOnReveal?: boolean;
} & SelectionProps & DeviceDragProps) {
  const { device, selection, multiSelection, onItemClick } = props;
  const draggable = props.eligibleDeviceIds.has(device.id);
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
      draggable={draggable}
      dragging={props.dragSource?.deviceId === device.id}
      revealGeneration={props.revealGeneration}
      scrollOnReveal={props.scrollOnReveal}
      onDragStart={draggable ? (event) => props.onDeviceDragStart(device, event) : undefined}
      onDragEnd={draggable ? props.onDeviceDragEnd : undefined}
    />
  );
}

// The topology counterpart of `NewGroupAddressRow`/`NewGroupRangeRow`.
// `medium_ref` (`MediumTypeRefId` — an opaque product reference `knx-core`
// deliberately does not interpret, see `Line`'s own doc comment) has no
// dropdown to pick from for the same reason; `"MT-0"` (ETS's own default
// for twisted-pair) is pre-filled so the common case needs no typing.
export function NewLineRow(props: { areaId: number; onCreated: (tree: ProjectTree) => void }) {
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
        aria-label={t("workbench.address")}
        placeholder="1"
        onChange={(e) => setAddress(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") void create();
        }}
      />
      <input
        value={name}
        aria-label={t("inspector.name")}
        placeholder={t("explorer.newLinePlaceholder")}
        onChange={(e) => setName(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") void create();
        }}
      />
      <input
        value={mediumRef}
        aria-label={t("structure.mediumRef")}
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

// `installationId` names the installation the area is created in (MODEL-01).
export function NewAreaRow(props: { installationId?: number; onCreated: (tree: ProjectTree) => void }) {
  const { installationId, onCreated } = props;
  const t = useTranslate();
  const [name, setName] = useState("");
  const [address, setAddress] = useState("");
  const [error, setError] = useState<string | null>(null);
  const canCreate = name.trim() !== "" && address.trim() !== "";

  async function create() {
    if (!canCreate) return;
    setError(null);
    try {
      const tree = await api.createArea(name, Number(address), installationId);
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
        aria-label={t("workbench.address")}
        placeholder="1"
        onChange={(e) => setAddress(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") void create();
        }}
      />
      <input
        value={name}
        aria-label={t("inspector.name")}
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

// The trigger for the add-device wizard (ADR-0093), aimed at this line,
// this installation's Unassigned bucket, or this room. A device on a line
// goes to that line's installation (ADR-0070); a line-less one names its
// installation explicitly, so every installation offers the row.
function AddDeviceRow(props: { onAdd: () => void }) {
  const t = useTranslate();
  return (
    <li className="tree-new-row">
      <button onClick={props.onAdd}>{t("explorer.addDevice")}</button>
    </li>
  );
}

function LineItem(
  props: { line: LineNode; onAddDevice: (lineId: number) => void }
    & SelectionProps
    & DeviceDragProps
    & LineDropProps
    & { revealRequest?: RevealRequest | null },
) {
  const { line, onAddDevice, selection, onSelect, multiSelection, onItemClick } = props;
  const t = useTranslate();
  const lineReveals = lineContainsSelection(line, props.revealRequest);
  return (
    <TreeNode
      label={t("explorer.lineLabel", { address: line.address, name: line.name })}
      selected={selection?.kind === "line" && selection.id === line.id}
      onSelect={() => onSelect({ kind: "line", id: line.id })}
      // `eligibleDeviceIds` holds only devices of this line's installation:
      // nothing connects two installations (ADR-0070).
      dropReady={props.dragSource !== null
        && props.eligibleDeviceIds.has(props.dragSource.deviceId)}
      onDragOver={(event) => {
        if (!acceptsDraggedDevice(
          event.dataTransfer,
          props.dragSource,
          props.eligibleDeviceIds,
        )) return;
        event.preventDefault();
        event.dataTransfer.dropEffect = "move";
      }}
      onDrop={(event) => props.onDeviceDropOnLine(line, event)}
      revealGeneration={revealGeneration(props.revealRequest, lineReveals)}
      scrollOnReveal={selectionIs(props.revealRequest, "line", line.id)}
    >
      {line.devices.map((d) => (
        <DeviceItem
          key={d.id}
          device={d}
          selection={selection}
          onSelect={onSelect}
          multiSelection={multiSelection}
          onItemClick={onItemClick}
          eligibleDeviceIds={props.eligibleDeviceIds}
          dragSource={props.dragSource}
          onDeviceDragStart={props.onDeviceDragStart}
          onDeviceDragEnd={props.onDeviceDragEnd}
          revealGeneration={revealGeneration(
            props.revealRequest,
            selectionIs(props.revealRequest, "device", d.id),
          )}
          scrollOnReveal={selectionIs(props.revealRequest, "device", d.id)}
        />
      ))}
      <AddDeviceRow onAdd={() => onAddDevice(line.id)} />
    </TreeNode>
  );
}

function AreaItem(
  props: {
    area: AreaNode;
    onCreated: (tree: ProjectTree) => void;
    onAddDevice: (lineId: number) => void;
  } & SelectionProps & DeviceDragProps & LineDropProps & { revealRequest?: RevealRequest | null },
) {
  const { area, onCreated, onAddDevice, selection, onSelect, multiSelection, onItemClick } =
    props;
  const t = useTranslate();
  const areaReveals = areaContainsSelection(area, props.revealRequest);
  return (
    <TreeNode
      label={t("explorer.areaLabel", { address: area.address, name: area.name })}
      selected={selection?.kind === "area" && selection.id === area.id}
      onSelect={() => onSelect({ kind: "area", id: area.id })}
      revealGeneration={revealGeneration(props.revealRequest, areaReveals)}
      scrollOnReveal={selectionIs(props.revealRequest, "area", area.id)}
    >
      {area.lines.map((l) => (
        <LineItem
          key={l.id}
          line={l}
          onAddDevice={onAddDevice}
          selection={selection}
          onSelect={onSelect}
          multiSelection={multiSelection}
          onItemClick={onItemClick}
          eligibleDeviceIds={props.eligibleDeviceIds}
          dragSource={props.dragSource}
          onDeviceDragStart={props.onDeviceDragStart}
          onDeviceDragEnd={props.onDeviceDragEnd}
          onDeviceDropOnLine={props.onDeviceDropOnLine}
          revealRequest={props.revealRequest}
        />
      ))}
      <NewLineRow areaId={area.id} onCreated={onCreated} />
    </TreeNode>
  );
}

function GroupAddressItem(props: { ga: GroupAddressNode; revealRequest?: RevealRequest | null } & SelectionProps) {
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
      // UX-01: drag onto a communication object's link row to link it.
      draggable
      onDragStart={(event) => writeDraggedGroupAddress(event.dataTransfer, ga.id)}
      revealGeneration={revealGeneration(
        props.revealRequest,
        selectionIs(props.revealRequest, "group_address", ga.id),
      )}
      scrollOnReveal={selectionIs(props.revealRequest, "group_address", ga.id)}
    />
  );
}

// One of two affordances in the tree that create a domain object rather
// than select one (the other is `NewGroupRangeRow`, below) — kept as an
// inline row rather than a dialog, the same way `AddressField`/`DptField`
// (Inspector.tsx) edit inline rather than popping a modal. Rendered under
// every installation: a range-less address names `installationId`, one in a
// range goes to the range's installation (ADR-0070). `ranges` is the
// installation's own flat `group_ranges` list
// (main and middle ranges alike) — an unset selection creates the address
// with no range, same as every group address created before this cycle.
function NewGroupAddressRow(props: {
  ranges: GroupRangeNode[];
  installationId: number;
  onCreated: (tree: ProjectTree) => void;
}) {
  const { ranges, installationId, onCreated } = props;
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
      // Both notations accepted whatever is on screen; the API only ever
      // sees the canonical `/` form (`gaNotation.ts`).
      const canonical = canonicalGroupAddress(address);
      const tree = rangeId === ""
        ? await api.createGroupAddress(name, canonical, undefined, installationId)
        : await api.createGroupAddress(name, canonical, Number(rangeId));
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
export function NewGroupRangeRow(props: {
  parentId?: number;
  /** Installation of a main range (no `parentId`), MODEL-01. */
  installationId?: number;
  onCreated: (tree: ProjectTree) => void;
}) {
  const { parentId, installationId, onCreated } = props;
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
      const tree = parentId === undefined && installationId !== undefined
        ? await api.createGroupRange(name, canonicalGroupAddress(start), canonicalGroupAddress(end),
          undefined, installationId)
        : await api.createGroupRange(name, canonicalGroupAddress(start), canonicalGroupAddress(end), parentId);
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
        aria-label={t("structure.rangeStart")}
        placeholder={formatGa("1/0/0")}
        onChange={(e) => setStart(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") void create();
        }}
      />
      <input
        value={end}
        aria-label={t("structure.rangeEnd")}
        placeholder={formatGa("1/7/255")}
        onChange={(e) => setEnd(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") void create();
        }}
      />
      <input
        value={name}
        aria-label={t("inspector.name")}
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
  "Stairway",
  "RoomPart",
  "Area",
  "Ground",
  "Segment",
] as const;

// The building-part counterpart of `NewGroupRangeRow`. `parentId` is
// `undefined` when rendered directly under the "Buildings" branch
// (creates a root part) and set to a `BuildingItem`'s own id when
// rendered under that item (creates a nested part) — unlike
// `NewGroupRangeRow`, nesting isn't capped at one level here: every
// `BuildingItem` gets its own row, since `BuildingPart` has no depth
// limit (`building.rs`'s own doc comment), unlike `GroupRange`'s
// observed two-level depth.
export function NewBuildingPartRow(props: {
  parentId?: number;
  fixedKind?: "Ground";
  /** Installation of a root part (no `parentId`), MODEL-01. */
  installationId?: number;
  onCreated: (tree: ProjectTree) => void;
}) {
  const { parentId, fixedKind, installationId, onCreated } = props;
  const t = useTranslate();
  const [name, setName] = useState("");
  const [kind, setKind] = useState<(typeof BUILDING_PART_KINDS)[number]>("Room");
  const [error, setError] = useState<string | null>(null);
  const canCreate = name.trim() !== "";

  async function create() {
    if (!canCreate) return;
    setError(null);
    try {
      const tree = parentId === undefined && installationId !== undefined
        ? await api.createBuildingPart(name, fixedKind ?? kind, undefined, installationId)
        : await api.createBuildingPart(name, fixedKind ?? kind, parentId);
      onCreated(tree);
      setName("");
    } catch (e) {
      setError(api.errorMessage(e));
    }
  }

  return (
    <li className="tree-new-row">
      {fixedKind ? <span className="tree-fixed-kind">{buildingPartKindLabel(t, fixedKind)}</span> :
        <select aria-label={t("structure.buildingKind")} value={kind} onChange={(e) => setKind(e.target.value as typeof kind)}>
          {BUILDING_PART_KINDS.map((k) => (
            <option key={k} value={k}>
              {buildingPartKindLabel(t, k)}
            </option>
          ))}
        </select>}
      <input
        value={name}
        aria-label={t("inspector.name")}
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
    onCreated: (tree: ProjectTree) => void;
    onAddDevice: (target: DeviceWizardTarget) => void;
    buildingDeviceRevealTarget?: BuildingDeviceRevealTarget;
  } & SelectionProps & DeviceDragProps & BuildingDropProps & { revealRequest?: RevealRequest | null },
) {
  const { building, onCreated, selection, onSelect, multiSelection, onItemClick } = props;
  const t = useTranslate();
  const buildingReveals = buildingContainsSelection(
    building,
    props.revealRequest,
    props.buildingDeviceRevealTarget,
  );
  return (
    <TreeNode
      label={t("explorer.buildingLabel", { name: building.name, kind: buildingPartKindLabel(t, building.kind) })}
      selected={selection?.kind === "building_part" && selection.id === building.id}
      onSelect={() => onSelect({ kind: "building_part", id: building.id })}
      dropReady={props.dragSource !== null
        && props.eligibleDeviceIds.has(props.dragSource.deviceId)}
      onDragOver={(event) => {
        if (!acceptsDraggedDevice(
          event.dataTransfer,
          props.dragSource,
          props.eligibleDeviceIds,
        )) return;
        event.preventDefault();
        event.dataTransfer.dropEffect = "move";
      }}
      onDrop={(event) => props.onDeviceDropOnBuildingPart(building, event)}
      revealGeneration={revealGeneration(props.revealRequest, buildingReveals)}
      scrollOnReveal={selectionIs(props.revealRequest, "building_part", building.id)}
    >
      {building.children.map((c) => (
        <BuildingItem
          key={c.id}
          building={c}
          onCreated={onCreated}
          onAddDevice={props.onAddDevice}
          selection={selection}
          onSelect={onSelect}
          multiSelection={multiSelection}
          onItemClick={onItemClick}
          eligibleDeviceIds={props.eligibleDeviceIds}
          dragSource={props.dragSource}
          onDeviceDragStart={props.onDeviceDragStart}
          onDeviceDragEnd={props.onDeviceDragEnd}
          onDeviceDropOnBuildingPart={props.onDeviceDropOnBuildingPart}
          revealRequest={props.revealRequest}
          buildingDeviceRevealTarget={props.buildingDeviceRevealTarget}
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
          eligibleDeviceIds={props.eligibleDeviceIds}
          dragSource={props.dragSource}
          onDeviceDragStart={props.onDeviceDragStart}
          onDeviceDragEnd={props.onDeviceDragEnd}
          // Topology/unassigned owns duplicates. A building-only device gets
          // exactly its depth-first first building occurrence as fallback.
          revealGeneration={revealGeneration(
            props.revealRequest,
            props.buildingDeviceRevealTarget?.buildingId === building.id
              && props.buildingDeviceRevealTarget.deviceId === d.id,
          )}
          scrollOnReveal={props.buildingDeviceRevealTarget?.buildingId === building.id
            && props.buildingDeviceRevealTarget.deviceId === d.id}
        />
      ))}
      {building.kind === "Room" && (
        <AddDeviceRow onAdd={() => props.onAddDevice({ buildingPartId: building.id })} />
      )}
      <NewBuildingPartRow parentId={building.id} onCreated={onCreated} />
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
    onCreated: (tree: ProjectTree) => void;
  } & Pick<SelectionProps, "selection" | "onSelect"> & { revealRequest?: RevealRequest | null },
) {
  const { node, onCreated, selection, onSelect } = props;
  const formatGa = useGroupAddressFormat();
  const { range, children } = node;
  const rangeReveals = groupRangeContainsSelection(node, props.revealRequest);
  return (
    <TreeNode
      label={`${formatGa(range.start)}–${formatGa(range.end)} ${range.name}`}
      selected={selection?.kind === "group_range" && selection.id === range.id}
      onSelect={() => onSelect({ kind: "group_range", id: range.id })}
      revealGeneration={revealGeneration(props.revealRequest, rangeReveals)}
      scrollOnReveal={selectionIs(props.revealRequest, "group_range", range.id)}
    >
      {children.map((c) => (
        <GroupRangeItem
          key={c.range.id}
          node={c}
          onCreated={onCreated}
          selection={selection}
          onSelect={onSelect}
          revealRequest={props.revealRequest}
        />
      ))}
      {range.parent === null && (
        <NewGroupRangeRow parentId={range.id} onCreated={onCreated} />
      )}
    </TreeNode>
  );
}

function InstallationItem(
  props: {
    installation: InstallationNode;
    onTreeUpdate: (tree: ProjectTree) => void;
    onAddDevice: (target: DeviceWizardTarget) => void;
  } & SelectionProps & DeviceDragProps & LineDropProps & BuildingDropProps & { revealRequest?: RevealRequest | null },
) {
  const {
    installation,
    onTreeUpdate,
    onAddDevice,
    selection,
    onSelect,
    multiSelection,
    onItemClick,
  } = props;
  const t = useTranslate();
  const requestedSelection = props.revealRequest?.selection;
  const topologyReveals = installation.topology.some((area) => areaContainsSelection(area, props.revealRequest));
  const unassignedReveal = requestedSelection?.kind === "device"
    && installation.unassigned.some((device) => device.id === requestedSelection.id);
  const buildingDeviceRevealTarget = requestedSelection?.kind === "device"
    && !topologyReveals
    && !unassignedReveal
    ? findBuildingDevice(installation.buildings, requestedSelection.id)
    : undefined;
  const buildingsReveal = installation.buildings.some((building) => buildingContainsSelection(
    building,
    props.revealRequest,
    buildingDeviceRevealTarget,
  ));
  const groupAddressesReveal = requestedSelection?.kind === "group_address"
    && installation.group_addresses.some((address) => address.id === requestedSelection.id);
  const groupRangesReveal = requestedSelection?.kind === "group_range"
    && installation.group_ranges.some((range) => range.id === requestedSelection.id);
  const installationReveals = topologyReveals || buildingsReveal || unassignedReveal
    || groupAddressesReveal || groupRangesReveal;
  return (
    <TreeNode label={installation.name} revealGeneration={revealGeneration(props.revealRequest, installationReveals)}>
      <TreeNode label={t("explorer.topology")} revealGeneration={revealGeneration(props.revealRequest, topologyReveals)}>
        {installation.topology.map((a) => (
          <AreaItem
            key={a.id}
            area={a}
            onCreated={onTreeUpdate}
            onAddDevice={(lineId) => onAddDevice({ lineId })}
            selection={selection}
            onSelect={onSelect}
            multiSelection={multiSelection}
            onItemClick={onItemClick}
            eligibleDeviceIds={props.eligibleDeviceIds}
            dragSource={props.dragSource}
            onDeviceDragStart={props.onDeviceDragStart}
            onDeviceDragEnd={props.onDeviceDragEnd}
            onDeviceDropOnLine={props.onDeviceDropOnLine}
            revealRequest={props.revealRequest}
          />
        ))}
        <NewAreaRow installationId={installation.id} onCreated={onTreeUpdate} />
      </TreeNode>
      <TreeNode label={t("explorer.buildings")} revealGeneration={revealGeneration(props.revealRequest, buildingsReveal)}>
        {installation.buildings.map((b) => (
          <BuildingItem
            key={b.id}
            building={b}
            onCreated={onTreeUpdate}
            onAddDevice={onAddDevice}
            selection={selection}
            onSelect={onSelect}
            multiSelection={multiSelection}
            onItemClick={onItemClick}
            eligibleDeviceIds={props.eligibleDeviceIds}
            dragSource={props.dragSource}
            onDeviceDragStart={props.onDeviceDragStart}
            onDeviceDragEnd={props.onDeviceDragEnd}
            onDeviceDropOnBuildingPart={props.onDeviceDropOnBuildingPart}
            revealRequest={props.revealRequest}
            buildingDeviceRevealTarget={buildingDeviceRevealTarget}
          />
        ))}
        <NewBuildingPartRow installationId={installation.id} onCreated={onTreeUpdate} />
      </TreeNode>
      <TreeNode label={t("explorer.unassigned")} revealGeneration={revealGeneration(props.revealRequest, unassignedReveal)}>
          {installation.unassigned.map((d) => (
            <DeviceItem
              key={d.id}
              device={d}
              selection={selection}
              onSelect={onSelect}
              multiSelection={multiSelection}
              onItemClick={onItemClick}
              eligibleDeviceIds={props.eligibleDeviceIds}
              dragSource={props.dragSource}
              onDeviceDragStart={props.onDeviceDragStart}
              onDeviceDragEnd={props.onDeviceDragEnd}
              revealGeneration={revealGeneration(
                props.revealRequest,
                selectionIs(props.revealRequest, "device", d.id),
              )}
              scrollOnReveal={selectionIs(props.revealRequest, "device", d.id)}
            />
          ))}
          <AddDeviceRow onAdd={() => onAddDevice({ lineId: null, installationId: installation.id })} />
        </TreeNode>
      <TreeNode label={t("explorer.groupAddresses")} revealGeneration={revealGeneration(props.revealRequest, groupAddressesReveal)}>
        {installation.group_addresses.map((ga) => (
          <GroupAddressItem
            key={ga.id}
            ga={ga}
            selection={selection}
            onSelect={onSelect}
            multiSelection={multiSelection}
            onItemClick={onItemClick}
            revealRequest={props.revealRequest}
          />
        ))}
        <NewGroupAddressRow ranges={installation.group_ranges} installationId={installation.id}
          onCreated={onTreeUpdate} />
      </TreeNode>
      <TreeNode label={t("explorer.groupRanges")} revealGeneration={revealGeneration(props.revealRequest, groupRangesReveal)}>
        {nestGroupRanges(installation.group_ranges).map((node) => (
          <GroupRangeItem
            key={node.range.id}
            node={node}
            onCreated={onTreeUpdate}
            selection={selection}
            onSelect={onSelect}
            revealRequest={props.revealRequest}
          />
        ))}
        <NewGroupRangeRow installationId={installation.id} onCreated={onTreeUpdate} />
      </TreeNode>
    </TreeNode>
  );
}

export default function ProjectExplorer(
  props: {
    tree: ProjectTree;
    onTreeUpdate: (tree: ProjectTree) => void;
    onSummary: (message: string) => void;
    onError: (error: unknown) => void;
    onRevealComplete?: (generation: number) => void;
    /** ADR-0093: opens the add-device wizard; the App owns it. */
    onAddDevice: (target: DeviceWizardTarget) => void;
  } & SelectionProps & { revealRequest?: RevealRequest | null },
) {
  const { tree, onTreeUpdate, selection, onSelect, multiSelection, onItemClick } = props;
  const t = useTranslate();
  const [dragSource, setDragSource] = useState<DragSource | null>(null);
  // MODEL-01 / ADR-0070: a device can be moved inside the one installation
  // whose topology places it; each installation only accepts its own.
  const topologyDevices = tree.installations.flatMap((installation) => [
    ...installation.topology.flatMap((area) => area.lines.flatMap((line) => line.devices)),
    ...installation.unassigned,
  ]);
  const owners = deviceInstallations(tree);
  const eligibleByInstallation = new Map<number, Set<number>>(
    tree.installations.map((installation) => [installation.id, new Set<number>()]));
  for (const [deviceId, owner] of owners) eligibleByInstallation.get(owner.id)?.add(deviceId);

  function onDeviceDragStart(
    device: DeviceNode,
    event: React.DragEvent<HTMLButtonElement>,
  ): void {
    if (!owners.has(device.id)) return;
    event.dataTransfer.setData(DEVICE_DRAG_MIME, String(device.id));
    event.dataTransfer.effectAllowed = "move";
    setDragSource({ deviceId: device.id });
  }

  function currentDraggedDevice(dataTransfer: DataTransfer): DeviceNode | null {
    const deviceId = parseDraggedDevice(dataTransfer);
    if (deviceId === null || deviceId !== dragSource?.deviceId) return null;
    return topologyDevices.find((candidate) => candidate.id === deviceId) ?? null;
  }

  // A drop arrives even when the target refused the dragover, so the drop
  // itself checks that source and target share one installation (ADR-0070).
  function sameInstallation(device: DeviceNode, target: InstallationNode | undefined): boolean {
    const source = owners.get(device.id);
    return source !== undefined && source === target;
  }

  async function onDeviceDropOnLine(
    line: LineNode,
    event: React.DragEvent<HTMLButtonElement>,
  ): Promise<void> {
    const device = currentDraggedDevice(event.dataTransfer);
    if (!device || !sameInstallation(device, owningInstallation(tree, "line", line.id))) return;

    event.preventDefault();
    try {
      const next = await api.moveDeviceToLine(device.id, line.id);
      onTreeUpdate(next);
      emitAchievementEvent({ type: "dragDropApplied" });
      props.onSummary(t("dragDrop.movedToLine", {
        device: device.name,
        line: t("explorer.lineLabel", { address: line.address, name: line.name }),
      }));
    } catch (error) {
      props.onError(error);
    } finally {
      setDragSource(null);
    }
  }

  async function onDeviceDropOnBuildingPart(
    building: BuildingNode,
    event: React.DragEvent<HTMLButtonElement>,
  ): Promise<void> {
    const device = currentDraggedDevice(event.dataTransfer);
    if (!device || !sameInstallation(device, owningInstallation(tree, "building_part", building.id))) return;

    event.preventDefault();
    try {
      const next = await api.moveDeviceToBuildingPart(device.id, building.id);
      onTreeUpdate(next);
      emitAchievementEvent({ type: "dragDropApplied" });
      props.onSummary(t("dragDrop.movedToBuildingPart", {
        device: device.name,
        buildingPart: building.name,
      }));
    } catch (error) {
      props.onError(error);
    } finally {
      setDragSource(null);
    }
  }

  return (
    <RevealCompletionContext.Provider value={props.onRevealComplete}>
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
        {tree.installations.map((inst) => (
          <InstallationItem
            key={inst.id}
            installation={inst}
            onTreeUpdate={onTreeUpdate}
            onAddDevice={props.onAddDevice}
            selection={selection}
            onSelect={onSelect}
            multiSelection={multiSelection}
            onItemClick={onItemClick}
            eligibleDeviceIds={eligibleByInstallation.get(inst.id) ?? new Set()}
            dragSource={dragSource}
            onDeviceDragStart={onDeviceDragStart}
            onDeviceDragEnd={() => setDragSource(null)}
            onDeviceDropOnLine={onDeviceDropOnLine}
            onDeviceDropOnBuildingPart={onDeviceDropOnBuildingPart}
            revealRequest={props.revealRequest}
          />
        ))}
      </ul>
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
    </RevealCompletionContext.Provider>
  );
}
