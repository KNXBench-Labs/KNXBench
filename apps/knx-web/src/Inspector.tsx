import { useEffect, useState } from "react";
import * as api from "./api";
import type { DeviceDetail } from "./bindings/DeviceDetail";
import type { ComObjectNode } from "./bindings/ComObjectNode";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { AreaNode } from "./bindings/AreaNode";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { GroupLinkNode } from "./bindings/GroupLinkNode";
import type { GroupRangeNode } from "./bindings/GroupRangeNode";
import type { LineNode } from "./bindings/LineNode";
import type { BuildingNode } from "./bindings/BuildingNode";
import type { Selection } from "./selection";
import ParameterPanel from "./ParameterPanel";
import { useTranslate, type MessageKey, type Translate } from "./i18n";
import {
  findArea,
  findBuildingPart,
  findDeviceBuildingPartInFirstInstallation,
  findDeviceLineInFirstInstallation,
  findGroupAddress,
  findGroupRange,
  findLine,
  flattenBuildingParts,
} from "./treeUtils";

// `BuildingNode.kind` is the raw `BuildingPartType` discriminant from the
// server ("Building", "Floor", … — see `bindings/BuildingNode.ts`'s own
// comment on why it stays a plain `string` rather than a `ts-rs` union).
// It doubles as a rendered label in `BuildingPartInspector` below, which is
// exactly the discriminant trap the brief calls out: this map translates
// the *display* text without ever touching `kind` itself, which keeps
// flowing untranslated into `treeUtils`/the API calls that compare it.
// `ProjectExplorer.tsx` carries the identical map for the same reason —
// see `buildingPartKind.*`'s own comment in `messages/en.ts` for why that
// one namespace is shared instead of split per surface.
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

// `GroupLinkNode.direction`/`NewGroupLinkRow`'s own `direction` state are
// `"Send"`/`"Receive"` wire values (`Direction`'s `Debug` form, sent
// straight into `unlinkComObject`/`linkComObject`) — never translated.
// Only the rendered word is.
const DIRECTION_KEYS: Record<string, MessageKey> = {
  Send: "inspector.direction.send",
  Receive: "inspector.direction.receive",
};

function directionLabel(t: Translate, direction: string): string {
  const key = DIRECTION_KEYS[direction];
  return key ? t(key) : direction;
}

// The six near-duplicate "Delete is only available for … in the first
// installation."/"Rename and Delete are only available for … in the first
// installation." sentences (one per entity type this file gates a create/
// edit/delete affordance on `installations[0]` for) collapsed into one
// template. `action` carries its own verb ("Delete is"/"Rename and Delete
// are") so the base sentence never needs to conjugate around how many
// verbs it's naming — see `inspector.restrictedAction.*`'s own comment in
// `messages/en.ts`.
function restrictedToFirstInstallationMessage(
  t: Translate,
  action: "delete" | "renameAndDelete",
  entityKey: MessageKey,
): string {
  const actionKey: MessageKey =
    action === "delete" ? "inspector.restrictedAction.delete" : "inspector.restrictedAction.renameAndDelete";
  return t("inspector.restrictedToFirstInstallation", {
    action: t(actionKey),
    entity: t(entityKey),
  });
}

function AddressField(props: { detail: DeviceDetail; onApplied: (tree: ProjectTree) => void }) {
  const { detail, onApplied } = props;
  const t = useTranslate();
  const [value, setValue] = useState(detail.address ?? "");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setValue(detail.address ?? "");
    setError(null);
  }, [detail.address]);

  async function apply() {
    const current = detail.address ?? "";
    if (value === current) return;
    setError(null);
    try {
      const tree = await api.setIndividualAddress(detail.id, value === "" ? null : value);
      onApplied(tree);
    } catch (e) {
      setError(api.errorMessage(e));
      setValue(current);
    }
  }

  return (
    <label className="inspector-field">
      {t("inspector.address")}
      <input
        value={value}
        placeholder="1.1.1"
        onChange={(e) => setValue(e.target.value)}
        onBlur={apply}
        onKeyDown={(e) => {
          if (e.key === "Enter") (e.target as HTMLInputElement).blur();
        }}
      />
      {error && <span className="field-error">{error}</span>}
    </label>
  );
}

function DeviceDescriptionField(props: {
  detail: DeviceDetail;
  onApplied: (tree: ProjectTree) => void;
}) {
  const { detail, onApplied } = props;
  const t = useTranslate();
  const [value, setValue] = useState(detail.description ?? "");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setValue(detail.description ?? "");
    setError(null);
  }, [detail.description]);

  async function apply() {
    const current = detail.description ?? "";
    if (value === current) return;
    setError(null);
    try {
      const tree = await api.setDeviceDescription(detail.id, value === "" ? null : value);
      onApplied(tree);
    } catch (e) {
      setError(api.errorMessage(e));
      setValue(current);
    }
  }

  return (
    <label className="inspector-field">
      {t("inspector.description")}
      <input
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onBlur={apply}
        onKeyDown={(e) => {
          if (e.key === "Enter") (e.target as HTMLInputElement).blur();
        }}
      />
      {error && <span className="field-error">{error}</span>}
    </label>
  );
}

function ComObjectDescriptionField(props: {
  com: ComObjectNode;
  onApplied: (tree: ProjectTree) => void;
}) {
  const { com, onApplied } = props;
  const t = useTranslate();
  const [value, setValue] = useState(com.description ?? "");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setValue(com.description ?? "");
    setError(null);
  }, [com.description]);

  async function apply() {
    const current = com.description ?? "";
    if (value === current) return;
    setError(null);
    try {
      const tree = await api.setComObjectDescription(com.id, value === "" ? null : value);
      onApplied(tree);
    } catch (e) {
      setError(api.errorMessage(e));
      setValue(current);
    }
  }

  return (
    <label className="inspector-field">
      {t("inspector.description")}
      <input
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onBlur={apply}
        onKeyDown={(e) => {
          if (e.key === "Enter") (e.target as HTMLInputElement).blur();
        }}
      />
      {error && <span className="field-error">{error}</span>}
    </label>
  );
}

function DptField(props: { com: ComObjectNode; onApplied: (tree: ProjectTree) => void }) {
  const { com, onApplied } = props;
  const t = useTranslate();
  const [value, setValue] = useState(com.dpt ?? "");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setValue(com.dpt ?? "");
    setError(null);
  }, [com.dpt]);

  async function apply() {
    const current = com.dpt ?? "";
    if (value === current) return;
    setError(null);
    try {
      const tree = await api.setComObjectDpt(com.id, value === "" ? null : value);
      onApplied(tree);
    } catch (e) {
      setError(api.errorMessage(e));
      setValue(current);
    }
  }

  return (
    <label className="inspector-field">
      {t("inspector.dpt")}
      <input
        value={value}
        placeholder="DPST-9-1"
        onChange={(e) => setValue(e.target.value)}
        onBlur={apply}
        onKeyDown={(e) => {
          if (e.key === "Enter") (e.target as HTMLInputElement).blur();
        }}
      />
      {error && <span className="field-error">{error}</span>}
    </label>
  );
}

// Five checkboxes, one per `ComFlagName` — each toggle applies immediately
// (no blur/Enter gesture, unlike `DptField`/`ComObjectDescriptionField`,
// since a checkbox's `onChange` already fires exactly once per intended
// edit). No "clear to inherited" affordance exists here, matching
// `Command::SetComObjectFlag`'s own bare-`bool` shape (see command.rs).
function ComObjectFlagsRow(props: { com: ComObjectNode; onApplied: (tree: ProjectTree) => void }) {
  const { com, onApplied } = props;
  const t = useTranslate();
  const [error, setError] = useState<string | null>(null);

  async function toggle(flag: api.ComFlagName, value: boolean) {
    setError(null);
    try {
      const tree = await api.setComObjectFlag(com.id, flag, value);
      onApplied(tree);
    } catch (e) {
      setError(api.errorMessage(e));
    }
  }

  // `label` (the R/W/T/U/C letter) and `name` (`api.ComFlagName`, sent
  // verbatim to `setComObjectFlag`) are not translatable — the letters are
  // KNX's own flag abbreviations and `name` is a wire value, not display
  // text. Only `titleKey` — the tooltip a mouse hover shows — is language
  // text, so only it gets a catalogue key.
  const flags: { label: string; name: api.ComFlagName; titleKey: MessageKey; value: boolean }[] = [
    { label: "R", name: "Read", titleKey: "inspector.comFlag.read", value: com.read },
    { label: "W", name: "Write", titleKey: "inspector.comFlag.write", value: com.write },
    { label: "T", name: "Transmit", titleKey: "inspector.comFlag.transmit", value: com.transmit },
    { label: "U", name: "Update", titleKey: "inspector.comFlag.update", value: com.update },
    {
      label: "C",
      name: "Communication",
      titleKey: "inspector.comFlag.communication",
      value: com.communication,
    },
  ];

  return (
    <div className="com-object-flags">
      {flags.map((f) => (
        <label key={f.name} title={t(f.titleKey)}>
          <input
            type="checkbox"
            checked={f.value}
            onChange={(e) => toggle(f.name, e.target.checked)}
          />
          {f.label}
        </label>
      ))}
      {error && <span className="field-error">{error}</span>}
    </div>
  );
}

// One existing `GroupLink`, with an Unlink button. `Command::UnlinkComObject`
// carries no `installations[0]`-only restriction the way Link/create/delete
// commands do (it removes whatever link already exists on the comm object,
// regardless of which installation the linked address lives in), so Unlink
// is always offered — unlike `NewGroupLinkRow`'s Link, which is gated by
// the picker only ever listing the first installation's addresses.
function GroupLinkRow(props: {
  com: ComObjectNode;
  link: GroupLinkNode;
  onApplied: (tree: ProjectTree) => void;
}) {
  const { com, link, onApplied } = props;
  const t = useTranslate();
  const [error, setError] = useState<string | null>(null);

  async function remove() {
    setError(null);
    try {
      const tree = await api.unlinkComObject(com.id, link.ga_id, link.direction);
      onApplied(tree);
    } catch (e) {
      setError(api.errorMessage(e));
    }
  }

  return (
    <li className="group-link-row">
      <span>
        {directionLabel(t, link.direction)}: {link.address ?? `#${link.ga_id}`}
        {link.name ? ` ${link.name}` : ""}
      </span>
      <button onClick={remove}>{t("inspector.unlink")}</button>
      {error && <span className="field-error">{error}</span>}
    </li>
  );
}

// The create counterpart of `GroupLinkRow` — same inline-row shape as
// `NewGroupAddressRow`/`NewGroupRangeRow`. `groupAddresses` is always the
// first installation's list: `Command::LinkComObject` only ever validates
// the target address against `installations[0]` (command.rs), the same
// constraint every other create affordance in this file already honors.
function NewGroupLinkRow(props: {
  com: ComObjectNode;
  groupAddresses: GroupAddressNode[];
  onApplied: (tree: ProjectTree) => void;
}) {
  const { com, groupAddresses, onApplied } = props;
  const t = useTranslate();
  const [gaId, setGaId] = useState("");
  const [direction, setDirection] = useState<"Send" | "Receive">("Send");
  const [error, setError] = useState<string | null>(null);
  const canLink = gaId !== "";

  async function link() {
    if (!canLink) return;
    setError(null);
    try {
      const tree = await api.linkComObject(com.id, Number(gaId), direction);
      onApplied(tree);
      setGaId("");
    } catch (e) {
      setError(api.errorMessage(e));
    }
  }

  return (
    <li className="tree-new-row">
      <select value={gaId} onChange={(e) => setGaId(e.target.value)}>
        <option value="">{t("inspector.chooseGroupAddress")}</option>
        {groupAddresses.map((ga) => (
          <option key={ga.id} value={ga.id}>
            {ga.address} {ga.name}
          </option>
        ))}
      </select>
      <select
        value={direction}
        onChange={(e) => setDirection(e.target.value as "Send" | "Receive")}
      >
        <option value="Send">{t("inspector.direction.send")}</option>
        <option value="Receive">{t("inspector.direction.receive")}</option>
      </select>
      <button onClick={link} disabled={!canLink}>
        {t("inspector.link")}
      </button>
      {error && <span className="field-error">{error}</span>}
    </li>
  );
}

// Moves a device between lines (or to/from unassigned) via
// `Command::MoveDeviceToLine` — independent of `AddressField`'s individual
// address, per that command's own doc comment ("a line move and a
// re-address are two separate user intents"). Renders nothing if
// `findDeviceLineInFirstInstallation` returns `undefined`: the device isn't
// reachable from `installations[0]`'s topology at all (building-only
// placement, or a later installation), so the command has nothing to
// target — same "hide rather than show a misleading value" rule as
// `GroupAddressInspector`'s `canDelete` gate, just applied to visibility
// instead of a disabled button, since there is no sensible current value to
// show disabled.
function LineMoveField(props: {
  detail: DeviceDetail;
  tree: ProjectTree;
  onApplied: (tree: ProjectTree) => void;
}) {
  const { detail, tree, onApplied } = props;
  const t = useTranslate();
  const current = findDeviceLineInFirstInstallation(tree, detail.id);
  const [error, setError] = useState<string | null>(null);

  if (current === undefined) return null;

  async function move(lineId: number | null) {
    setError(null);
    try {
      const tree = await api.moveDeviceToLine(detail.id, lineId);
      onApplied(tree);
    } catch (e) {
      setError(api.errorMessage(e));
    }
  }

  return (
    <label className="inspector-field">
      {t("inspector.line")}
      <select
        value={current ?? ""}
        onChange={(e) => move(e.target.value === "" ? null : Number(e.target.value))}
      >
        <option value="">{t("inspector.unassigned")}</option>
        {tree.installations[0]?.topology.map((area) => (
          <optgroup
            key={area.id}
            label={t("inspector.areaLabel", { address: area.address, name: area.name })}
          >
            {area.lines.map((line) => (
              <option key={line.id} value={line.id}>
                {t("inspector.lineLabel", { address: line.address, name: line.name })}
              </option>
            ))}
          </optgroup>
        ))}
      </select>
      {error && <span className="field-error">{error}</span>}
    </label>
  );
}

// The building-part counterpart of `LineMoveField`, via
// `Command::MoveDeviceToBuildingPart`. Unlike `LineMoveField`, `null` in
// the select means "not placed in any building part" — a normal steady
// state, not a bucket the device is moved *into* the way `unassigned`
// is for `MoveDeviceToLine` — so there is no dedicated "(unassigned)"
// semantic beyond the same empty option every optional select here
// uses. Rendering is gated by `findDeviceLineInFirstInstallation`, not
// a building-specific check: both commands share the same
// `installations[0]`-only restriction, and that helper already reports
// it accurately (`current === undefined` below).
function BuildingPartMoveField(props: {
  detail: DeviceDetail;
  tree: ProjectTree;
  onApplied: (tree: ProjectTree) => void;
}) {
  const { detail, tree, onApplied } = props;
  const t = useTranslate();
  const current = findDeviceLineInFirstInstallation(tree, detail.id);
  const currentPart = findDeviceBuildingPartInFirstInstallation(tree, detail.id);
  const [error, setError] = useState<string | null>(null);

  if (current === undefined) return null;

  async function move(partId: number | null) {
    setError(null);
    try {
      const tree = await api.moveDeviceToBuildingPart(detail.id, partId);
      onApplied(tree);
    } catch (e) {
      setError(api.errorMessage(e));
    }
  }

  const parts = flattenBuildingParts(tree.installations[0]?.buildings ?? [], []);

  return (
    <label className="inspector-field">
      {t("inspector.buildingPart")}
      <select
        value={currentPart ?? ""}
        onChange={(e) => move(e.target.value === "" ? null : Number(e.target.value))}
      >
        <option value="">{t("inspector.none")}</option>
        {parts.map(({ node, path }) => (
          <option key={node.id} value={node.id}>
            {path}
          </option>
        ))}
      </select>
      {error && <span className="field-error">{error}</span>}
    </label>
  );
}

function DeviceInspector(props: {
  detail: DeviceDetail;
  tree: ProjectTree;
  // Same `installations[0]`-only gate as every other Delete button in this
  // file — `Command::DeleteDevice` only ever searches the first
  // installation's topology (`remove_device_from_topology` in
  // command.rs), the exact reachability `findDeviceLineInFirstInstallation`
  // already reports for `LineMoveField`.
  canDelete: boolean;
  onApplied: (tree: ProjectTree) => void;
  onDeleted: (tree: ProjectTree) => void;
}) {
  const { detail, tree, canDelete, onApplied, onDeleted } = props;
  const t = useTranslate();
  const groupAddresses = tree.installations[0]?.group_addresses ?? [];
  const [error, setError] = useState<string | null>(null);

  async function remove() {
    setError(null);
    try {
      const tree = await api.deleteDevice(detail.id);
      onDeleted(tree);
    } catch (e) {
      // Also where `CommandError::DeviceHasLinks` surfaces — the server
      // refuses to delete a device whose comm objects still have group
      // links, so the user sees why instead of a silent no-op.
      setError(api.errorMessage(e));
    }
  }

  return (
    <div className="inspector">
      <h2>{detail.name}</h2>
      {canDelete ? (
        <button onClick={remove}>{t("inspector.delete")}</button>
      ) : (
        <p className="inspector-description">
          {restrictedToFirstInstallationMessage(t, "delete", "inspector.entity.devices")}
        </p>
      )}
      {error && <span className="field-error">{error}</span>}
      <AddressField detail={detail} onApplied={onApplied} />
      <LineMoveField detail={detail} tree={tree} onApplied={onApplied} />
      <BuildingPartMoveField detail={detail} tree={tree} onApplied={onApplied} />
      <DeviceDescriptionField detail={detail} onApplied={onApplied} />
      <h3>{t("inspector.communicationObjects")}</h3>
      <ul className="com-object-list">
        {detail.com_objects.map((com) => (
          <li key={com.id}>
            <span className="com-object-label">
              {com.number}: {com.name ?? t("inspector.unnamed")}
            </span>
            <DptField com={com} onApplied={onApplied} />
            {com.dpt_layer && <span className="provenance-badge">{com.dpt_layer}</span>}
            <ComObjectDescriptionField com={com} onApplied={onApplied} />
            {com.description_layer && (
              <span className="provenance-badge">{com.description_layer}</span>
            )}
            <ComObjectFlagsRow com={com} onApplied={onApplied} />
            <ul className="group-link-list">
              {com.links.map((link) => (
                <GroupLinkRow
                  key={`${link.ga_id}-${link.direction}`}
                  com={com}
                  link={link}
                  onApplied={onApplied}
                />
              ))}
              <NewGroupLinkRow com={com} groupAddresses={groupAddresses} onApplied={onApplied} />
            </ul>
          </li>
        ))}
      </ul>
      {/* T18 slice 3 task 4: fetches its own panel keyed on `detail.id`,
          unconditionally (see ParameterPanel.tsx's own comment on why). */}
      <ParameterPanel deviceId={detail.id} />
    </div>
  );
}

function GroupAddressInspector(props: {
  ga: GroupAddressNode;
  // `Command::apply`'s `DeleteGroupAddress` arm only ever searches
  // `installations[0]` (command.rs) — the same reason
  // `ProjectExplorer.tsx`'s inline create row only renders under the
  // first installation. Offering Delete for a group address that lives
  // in any other installation would always fail with a confusing
  // "not found" error, so the button itself is gated instead.
  canDelete: boolean;
  onDeleted: (tree: ProjectTree) => void;
}) {
  const { ga, canDelete, onDeleted } = props;
  const t = useTranslate();
  const [error, setError] = useState<string | null>(null);

  async function remove() {
    setError(null);
    try {
      const tree = await api.deleteGroupAddress(ga.id);
      onDeleted(tree);
    } catch (e) {
      setError(api.errorMessage(e));
    }
  }

  return (
    <div className="inspector">
      <h2>{ga.name}</h2>
      <p className="inspector-address">{ga.address}</p>
      {canDelete ? (
        <button onClick={remove}>{t("inspector.delete")}</button>
      ) : (
        <p className="inspector-description">
          {restrictedToFirstInstallationMessage(t, "delete", "inspector.entity.groupAddresses")}
        </p>
      )}
      {error && <span className="field-error">{error}</span>}
    </div>
  );
}

function GroupRangeNameField(props: {
  range: GroupRangeNode;
  onApplied: (tree: ProjectTree) => void;
}) {
  const { range, onApplied } = props;
  const t = useTranslate();
  const [value, setValue] = useState(range.name);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setValue(range.name);
    setError(null);
  }, [range.name]);

  async function apply() {
    if (value === range.name || value.trim() === "") {
      setValue(range.name);
      return;
    }
    setError(null);
    try {
      const tree = await api.renameGroupRange(range.id, value);
      onApplied(tree);
    } catch (e) {
      setError(api.errorMessage(e));
      setValue(range.name);
    }
  }

  return (
    <label className="inspector-field">
      {t("inspector.name")}
      <input
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onBlur={apply}
        onKeyDown={(e) => {
          if (e.key === "Enter") (e.target as HTMLInputElement).blur();
        }}
      />
      {error && <span className="field-error">{error}</span>}
    </label>
  );
}

function GroupRangeInspector(props: {
  range: GroupRangeNode;
  // Same `installations[0]`-only gate as `GroupAddressInspector` — every
  // group-range command (`Command::CreateGroupRange`/`DeleteGroupRange`/
  // `RenameGroupRange`) only ever searches the first installation
  // (command.rs).
  canEdit: boolean;
  onApplied: (tree: ProjectTree) => void;
  onDeleted: (tree: ProjectTree) => void;
}) {
  const { range, canEdit, onApplied, onDeleted } = props;
  const t = useTranslate();
  const [error, setError] = useState<string | null>(null);

  async function remove() {
    setError(null);
    try {
      const tree = await api.deleteGroupRange(range.id);
      onDeleted(tree);
    } catch (e) {
      setError(api.errorMessage(e));
    }
  }

  return (
    <div className="inspector">
      <h2>{range.name}</h2>
      <p className="inspector-address">
        {range.start}–{range.end}
      </p>
      {canEdit ? (
        <>
          <GroupRangeNameField range={range} onApplied={onApplied} />
          <button onClick={remove}>{t("inspector.delete")}</button>
        </>
      ) : (
        <p className="inspector-description">
          {restrictedToFirstInstallationMessage(t, "renameAndDelete", "inspector.entity.groupRanges")}
        </p>
      )}
      {error && <span className="field-error">{error}</span>}
    </div>
  );
}

// No `RenameArea` command exists — `CreateArea`/`DeleteArea` are the only
// two, so unlike `GroupRangeInspector` there is nothing to edit here, just
// a summary and Delete.
function AreaInspector(props: {
  area: AreaNode;
  // Same `installations[0]`-only gate as every other Delete button in this
  // file — `Command::DeleteArea` only ever searches the first installation
  // (command.rs).
  canDelete: boolean;
  onDeleted: (tree: ProjectTree) => void;
}) {
  const { area, canDelete, onDeleted } = props;
  const t = useTranslate();
  const [error, setError] = useState<string | null>(null);

  async function remove() {
    setError(null);
    try {
      const tree = await api.deleteArea(area.id);
      onDeleted(tree);
    } catch (e) {
      setError(api.errorMessage(e));
    }
  }

  return (
    <div className="inspector">
      <h2>{t("inspector.areaLabel", { address: area.address, name: area.name })}</h2>
      <p className="inspector-description">
        {t("inspector.lineCount", { count: area.lines.length })}
      </p>
      {canDelete ? (
        <button onClick={remove}>{t("inspector.delete")}</button>
      ) : (
        <p className="inspector-description">
          {restrictedToFirstInstallationMessage(t, "delete", "inspector.entity.areas")}
        </p>
      )}
      {error && <span className="field-error">{error}</span>}
    </div>
  );
}

function LineInspector(props: {
  line: LineNode;
  canDelete: boolean;
  onDeleted: (tree: ProjectTree) => void;
}) {
  const { line, canDelete, onDeleted } = props;
  const t = useTranslate();
  const [error, setError] = useState<string | null>(null);

  async function remove() {
    setError(null);
    try {
      const tree = await api.deleteLine(line.id);
      onDeleted(tree);
    } catch (e) {
      setError(api.errorMessage(e));
    }
  }

  return (
    <div className="inspector">
      <h2>{t("inspector.lineLabel", { address: line.address, name: line.name })}</h2>
      <p className="inspector-description">
        {t("inspector.deviceCount", { count: line.devices.length })}
      </p>
      {canDelete ? (
        <button onClick={remove}>{t("inspector.delete")}</button>
      ) : (
        <p className="inspector-description">
          {restrictedToFirstInstallationMessage(t, "delete", "inspector.entity.lines")}
        </p>
      )}
      {error && <span className="field-error">{error}</span>}
    </div>
  );
}

function BuildingPartNameField(props: {
  part: BuildingNode;
  onApplied: (tree: ProjectTree) => void;
}) {
  const { part, onApplied } = props;
  const t = useTranslate();
  const [value, setValue] = useState(part.name);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setValue(part.name);
    setError(null);
  }, [part.name]);

  async function apply() {
    if (value === part.name || value.trim() === "") {
      setValue(part.name);
      return;
    }
    setError(null);
    try {
      const tree = await api.renameBuildingPart(part.id, value);
      onApplied(tree);
    } catch (e) {
      setError(api.errorMessage(e));
      setValue(part.name);
    }
  }

  return (
    <label className="inspector-field">
      {t("inspector.name")}
      <input
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onBlur={apply}
        onKeyDown={(e) => {
          if (e.key === "Enter") (e.target as HTMLInputElement).blur();
        }}
      />
      {error && <span className="field-error">{error}</span>}
    </label>
  );
}

function BuildingPartInspector(props: {
  node: BuildingNode;
  path: string;
  // Same `installations[0]`-only gate as `GroupRangeInspector`'s
  // `canEdit` — `Command::CreateBuildingPart`/`DeleteBuildingPart`/
  // `RenameBuildingPart` only ever search the first installation
  // (command.rs).
  canEdit: boolean;
  onApplied: (tree: ProjectTree) => void;
  onDeleted: (tree: ProjectTree) => void;
}) {
  const { node, path, canEdit, onApplied, onDeleted } = props;
  const t = useTranslate();
  const [error, setError] = useState<string | null>(null);

  async function remove() {
    setError(null);
    try {
      const tree = await api.deleteBuildingPart(node.id);
      onDeleted(tree);
    } catch (e) {
      // Also where `CommandError::BuildingPartNotEmpty` surfaces — the
      // server refuses to delete a part that still has a child or a
      // device, so the user sees why instead of a silent no-op.
      setError(api.errorMessage(e));
    }
  }

  return (
    <div className="inspector">
      <h2>{node.name}</h2>
      {/* `node.kind` is the raw `BuildingPartType` discriminant — see
          `buildingPartKindLabel`'s own comment above for why only the
          rendered word goes through translation. */}
      <p className="inspector-description">{buildingPartKindLabel(t, node.kind)}</p>
      <p className="inspector-path">{path}</p>
      <p>
        {t("inspector.deviceCount", { count: node.devices.length })},{" "}
        {t("inspector.childPartCount", { count: node.children.length })}
      </p>
      {canEdit ? (
        <>
          <BuildingPartNameField part={node} onApplied={onApplied} />
          <button onClick={remove}>{t("inspector.delete")}</button>
        </>
      ) : (
        <p className="inspector-description">
          {restrictedToFirstInstallationMessage(
            t,
            "renameAndDelete",
            "inspector.entity.buildingParts",
          )}
        </p>
      )}
      {error && <span className="field-error">{error}</span>}
    </div>
  );
}

// `deviceDetail` is `null` both before the async `device_detail` fetch
// lands and if it errored (App.tsx clears it either way) — in either case
// there is nothing to show yet, so this renders nothing rather than a
// half-populated panel. Group-address and building-part selections have
// no such gap: both resolve synchronously from `tree`, which is always
// already loaded by the time Inspector can render at all.
export default function Inspector(props: {
  selection: Selection;
  tree: ProjectTree;
  deviceDetail: DeviceDetail | null;
  onApplied: (tree: ProjectTree) => void;
  onDeleted: (tree: ProjectTree) => void;
}) {
  const { selection, tree, deviceDetail, onApplied, onDeleted } = props;

  if (selection.kind === "device") {
    if (!deviceDetail) return null;
    const canDelete = findDeviceLineInFirstInstallation(tree, deviceDetail.id) !== undefined;
    return (
      <DeviceInspector
        detail={deviceDetail}
        tree={tree}
        canDelete={canDelete}
        onApplied={onApplied}
        onDeleted={onDeleted}
      />
    );
  }

  if (selection.kind === "group_address") {
    const ga = findGroupAddress(tree, selection.id);
    if (!ga) return null;
    const canDelete = tree.installations[0]?.group_addresses.some((g) => g.id === ga.id) ?? false;
    return <GroupAddressInspector ga={ga} canDelete={canDelete} onDeleted={onDeleted} />;
  }

  if (selection.kind === "group_range") {
    const range = findGroupRange(tree, selection.id);
    if (!range) return null;
    const canEdit = tree.installations[0]?.group_ranges.some((r) => r.id === range.id) ?? false;
    return (
      <GroupRangeInspector range={range} canEdit={canEdit} onApplied={onApplied} onDeleted={onDeleted} />
    );
  }

  if (selection.kind === "area") {
    const area = findArea(tree, selection.id);
    if (!area) return null;
    const canDelete = tree.installations[0]?.topology.some((a) => a.id === area.id) ?? false;
    return <AreaInspector area={area} canDelete={canDelete} onDeleted={onDeleted} />;
  }

  if (selection.kind === "line") {
    const line = findLine(tree, selection.id);
    if (!line) return null;
    const canDelete =
      tree.installations[0]?.topology.some((a) => a.lines.some((l) => l.id === line.id)) ?? false;
    return <LineInspector line={line} canDelete={canDelete} onDeleted={onDeleted} />;
  }

  const found = findBuildingPart(tree, selection.id);
  if (!found) return null;
  const canEdit = flattenBuildingParts(tree.installations[0]?.buildings ?? [], []).some(
    ({ node }) => node.id === found.node.id,
  );
  return (
    <BuildingPartInspector
      node={found.node}
      path={found.path}
      canEdit={canEdit}
      onApplied={onApplied}
      onDeleted={onDeleted}
    />
  );
}
