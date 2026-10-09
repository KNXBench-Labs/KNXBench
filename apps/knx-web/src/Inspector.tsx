/** Properties inspector showing and editing details for whatever tree entity is selected. */
import { useEffect, useRef, useState } from "react";
import { carriesGroupAddress, readDraggedGroupAddress } from "./groupAddressDrag";
import * as api from "./api";
import { emitAchievementEvent } from "./achievementEvents";
import type { DeviceDetail } from "./bindings/DeviceDetail";
import type { ComObjectNode } from "./bindings/ComObjectNode";
import type { ComObjectActivation } from "./bindings/ComObjectActivation";
import type { ComObjectChannel } from "./bindings/ComObjectChannel";
import type { DeviceProductNode } from "./bindings/DeviceProductNode";
import type { DeviceProductCatalog } from "./bindings/DeviceProductCatalog";
import type { ProductResolution } from "./bindings/ProductResolution";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { AreaNode } from "./bindings/AreaNode";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { DeclaredDptNode } from "./bindings/DeclaredDptNode";
import type { GroupAddressDptOutcome } from "./bindings/GroupAddressDptOutcome";
import type { GroupLinkNode } from "./bindings/GroupLinkNode";
import type { GroupRangeNode } from "./bindings/GroupRangeNode";
import type { LineNode } from "./bindings/LineNode";
import type { BuildingNode } from "./bindings/BuildingNode";
import type { Selection } from "./selection";
import { ParameterPanelContent, useDeviceParameters } from "./ParameterPanel";
import HelpTip from "./HelpTip";
import DeviceLink from "./DeviceLink";
import { useTranslate, type MessageKey, type Translate } from "./i18n";
import { useProductLanguage } from "./productLanguage";
import { LanguageFallbackBadge, fellBack } from "./languageFallback";
import { useGroupAddressFormat } from "./gaNotation";
import {
  directionLabel,
  dptText,
  hasDptConflict,
  linkDirectionCounts,
} from "./groupAddressView";
import {
  findArea,
  findBuildingPart,
  deviceInstallation,
  devicePlacementSlots,
  findDeviceBuildingPart,
  findDeviceLine,
  owningInstallation,
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

// The six near-duplicate "Delete is only available for … in the first
// installation."/"Rename and Delete are only available for … in the first
// installation." sentences (one per entity type this file gates a create/
// edit/delete affordance on `installations[0]` for) collapsed into one
// template. `action` carries its own verb ("Delete is"/"Rename and Delete
// are") so the base sentence never needs to conjugate around how many
// verbs it's naming — see `inspector.restrictedAction.*`'s own comment in
// `messages/en.ts`.
function restrictedToOneInstallationMessage(
  t: Translate,
  action: "delete" | "renameAndDelete" | "renameMoveAndDelete",
  entityKey: MessageKey,
): string {
  const actionKey: MessageKey =
    action === "delete"
      ? "inspector.restrictedAction.delete"
      : action === "renameAndDelete"
        ? "inspector.restrictedAction.renameAndDelete"
        : "inspector.restrictedAction.renameMoveAndDelete";
  return t("inspector.restrictedToOneInstallation", {
    action: t(actionKey),
    entity: t(entityKey),
  });
}

// ProjectTree nests lines under their owning areas. A device listed in
// multiple topology positions (including unassigned + line) or under a
// malformed area has no trustworthy address prefix.
function addressLineContext(tree: ProjectTree, deviceId: number):
  | { kind: "assigned"; area: number; line: number }
  | { kind: "unassigned" | "ambiguous" } {
  let found: { kind: "assigned"; area: number; line: number } | null = null;
  let placements = 0;
  for (const installation of tree.installations) {
    placements += installation.unassigned.filter((device) => device.id === deviceId).length;
    for (const area of installation.topology) {
      for (const line of area.lines) {
        const occurrences = line.devices.filter((device) => device.id === deviceId).length;
        if (occurrences === 0) continue;
        if (occurrences > 1 || found || !Number.isInteger(area.address) || area.address < 0 || area.address > 15
          || !Number.isInteger(line.address) || line.address < 0 || line.address > 15) {
          return { kind: "ambiguous" };
        }
        found = { kind: "assigned", area: area.address, line: line.address };
        placements += 1;
      }
    }
  }
  return placements > 1 ? { kind: "ambiguous" } : found ?? { kind: "unassigned" };
}

function AddressField(props: { detail: DeviceDetail; tree: ProjectTree; onApplied: (tree: ProjectTree) => void }) {
  const { detail, tree, onApplied } = props;
  const t = useTranslate();
  const context = addressLineContext(tree, detail.id);
  const prefix = context.kind === "assigned" ? `${context.area}.${context.line}.` : null;
  const current = detail.address ?? "";
  const initial = prefix && detail.address ? detail.address.split(".").at(-1) ?? "" : current;
  const [value, setValue] = useState(initial);
  const [dirty, setDirty] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setValue(initial);
    setDirty(false);
    setError(null);
  }, [detail.id, initial, prefix]);

  async function apply() {
    if (!dirty || context.kind === "ambiguous") return;
    let address: string | null = value === "" ? null : value;
    if (prefix && address !== null) {
      if (!/^\d{1,3}$/.test(address) || Number(address) > 255) {
        setError(t("inspector.address.invalidDevice"));
        return;
      }
      address = `${prefix}${Number(address)}`;
      // Device number 0 is reserved for couplers. Whether this product is
      // one is a product-database fact only the server can check
      // (`Hardware/@IsCoupler`, MODEL-03), so the server decides and its
      // refusal is shown like any other address error.
    }
    if ((address ?? "") === current) {
      setDirty(false);
      setError(null);
      return;
    }
    setError(null);
    try {
      const result = await api.setIndividualAddress(detail.id, address);
      setDirty(false);
      onApplied(result);
    } catch (e) {
      setError(api.errorMessage(e));
      setValue(initial);
      setDirty(false);
    }
  }

  const mismatch = prefix !== null && current !== "" && !current.startsWith(prefix);
  return (
    <label className="inspector-field individual-address-field">
      {prefix ? t("inspector.address.deviceOctet") : t("inspector.address")}
      <span className="address-editor">
        {prefix && <span className="address-prefix" id={`device-address-prefix-${detail.id}`}>{prefix}</span>}
        <input
          value={value}
          placeholder={prefix ? "1–255" : "1.1.1"}
          aria-label={prefix ? t("inspector.address.deviceOctet") : t("inspector.address")}
          aria-describedby={prefix ? `device-address-prefix-${detail.id}` : undefined}
          inputMode={prefix ? "numeric" : "text"}
          disabled={context.kind === "ambiguous"}
          onChange={(e) => { setValue(e.target.value); setDirty(true); setError(null); }}
          onBlur={apply}
          onKeyDown={(e) => {
            if (e.key === "Enter") (e.target as HTMLInputElement).blur();
          }}
        />
      </span>
      {context.kind === "ambiguous" && <small className="field-error">{t("inspector.address.ambiguous")}</small>}
      {mismatch && <small className="inspector-description">{t("inspector.address.mismatch", { address: current, line: prefix.slice(0, -1) })}</small>}
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

// Six checkboxes, one per `ComFlagName` — each toggle applies immediately
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

  // The code letter and API flag name are stable KNX/wire values; the
  // visible long name and tooltip both use the existing localized key.
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
    {
      label: "I",
      name: "ReadOnInit",
      titleKey: "inspector.comFlag.readOnInit",
      value: com.read_on_init,
    },
  ];

  return (
    <div className="com-object-flags">
      {/* The six `title` attributes below stay — they are a fine mouse
          affordance for a one-word flag name. The tip is the keyboard's
          way in, and carries the sentence the letters cannot (ADR-0024). */}
      <HelpTip labelKey="help.tip.comFlags.label" textKey="help.tip.comFlags.text" topicId="comObjectFlags" />
      {flags.map((f) => (
        <label key={f.name} title={t(f.titleKey)}>
          <input
            type="checkbox"
            checked={f.value}
            onChange={(e) => toggle(f.name, e.target.checked)}
          />
          <span className="flag-code" aria-hidden="true">{f.label}</span>
          <span className="flag-name">{t(f.titleKey)}</span>
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
  const formatGa = useGroupAddressFormat();
  const [error, setError] = useState<string | null>(null);

  async function remove(direction: string) {
    setError(null);
    try {
      const tree = await api.unlinkComObject(com.id, link.ga_id, direction);
      onApplied(tree);
    } catch (e) {
      setError(api.errorMessage(e));
    }
  }

  return (
    <li className="group-link-row">
      <span>
        {directionLabel(t, link.direction)}:{" "}
        <span className="ga-address">{link.address === null ? `#${link.ga_id}` : formatGa(link.address)}</span>
        {link.name ? ` ${link.name}` : ""}
      </span>
      <button onClick={() => remove(link.direction)}>{t("inspector.unlink")}</button>
      {link.direction === "Send" && com.links.some((other) => other.ga_id === link.ga_id && other.direction === "Receive") && (
        <button className="unlink-both" onClick={() => remove("Both")}>{t("inspector.unlinkBoth")}</button>
      )}
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
  const formatGa = useGroupAddressFormat();
  const [gaId, setGaId] = useState("");
  const [direction, setDirection] = useState<"Send" | "Receive" | "Both">("Send");
  const [error, setError] = useState<string | null>(null);
  const [dropReady, setDropReady] = useState(false);
  const canLink = gaId !== "";

  async function linkTo(id: number, viaDrop = false) {
    setError(null);
    try {
      const tree = await api.linkComObject(com.id, id, direction);
      onApplied(tree);
      if (viaDrop) emitAchievementEvent({ type: "dragDropApplied" });
      setGaId("");
    } catch (e) {
      setError(api.errorMessage(e));
    }
  }

  async function link() {
    if (!canLink) return;
    await linkTo(Number(gaId));
  }

  // UX-01: a group address dropped on this row is linked exactly like
  // choosing it here and pressing Link, in the direction chosen in the row.
  // Only addresses this device may link (its installation's, MODEL-01) are
  // sent; anything else stays a local refusal without a request.
  function drop(event: React.DragEvent<HTMLLIElement>) {
    if (!carriesGroupAddress(event.dataTransfer)) return;
    event.preventDefault();
    setDropReady(false);
    const id = readDraggedGroupAddress(event.dataTransfer);
    if (id === null) return;
    if (!groupAddresses.some((ga) => ga.id === id)) {
      setError(t("inspector.dropNotLinkable"));
      return;
    }
    void linkTo(id, true);
  }

  return (
    <li className="tree-new-row" data-drop-ready={dropReady ? "true" : undefined}
      onDragOver={(event) => {
        if (!carriesGroupAddress(event.dataTransfer)) return;
        event.preventDefault();
        event.dataTransfer.dropEffect = "link";
        setDropReady(true);
      }}
      onDragLeave={() => setDropReady(false)}
      onDrop={drop}>
      <select value={gaId} onChange={(e) => setGaId(e.target.value)}>
        <option value="">{t("inspector.chooseGroupAddress")}</option>
        {groupAddresses.map((ga) => (
          <option key={ga.id} value={ga.id}>
            {formatGa(ga.address)} {ga.name}
          </option>
        ))}
      </select>
      <select
        value={direction}
        onChange={(e) => setDirection(e.target.value as "Send" | "Receive" | "Both")}
      >
        <option value="Send">{t("inspector.direction.send")}</option>
        <option value="Receive">{t("inspector.direction.receive")}</option>
        <option value="Both">{t("inspector.direction.both")}</option>
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
  const current = findDeviceLine(tree, detail.id);
  const installation = deviceInstallation(tree, detail.id);
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
        {installation?.topology.map((area) => (
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
  const current = findDeviceLine(tree, detail.id);
  const currentPart = findDeviceBuildingPart(tree, detail.id);
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

  const parts = flattenBuildingParts(deviceInstallation(tree, detail.id)?.buildings ?? [], []);

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

// T16's four `ProductResolution` variants, each with its own badge word and
// its own sentence. Kept as a map rather than a chain of ternaries so the
// compiler catches a fifth variant arriving in `bindings/ProductResolution.ts`
// instead of it quietly falling through to whatever the last `else` said —
// which is exactly the "bare unknown" the brief forbids. `Resolved` is the
// one variant with no sentence: the catalogue below says everything.
const RESOLUTION_KEYS: Record<ProductResolution, { badge: MessageKey; explain: MessageKey | null }> = {
  Resolved: { badge: "deviceIdentity.resolution.resolved", explain: null },
  NoDatabase: { badge: "deviceIdentity.resolution.noDatabase", explain: "deviceIdentity.explain.noDatabase" },
  NotInDatabase: { badge: "deviceIdentity.resolution.notInDatabase", explain: "deviceIdentity.explain.notInDatabase" },
  NoReference: { badge: "deviceIdentity.resolution.noReference", explain: "deviceIdentity.explain.noReference" },
};

// The three fields that answer "which product is this" on sight. They stay
// above the fold next to the refs; everything else lives behind the
// disclosure below, so a fully resolved device does not push the
// communication object table off the screen to say so.
const HEADLINE_FIELDS: CatalogField[] = [
  { label: "deviceIdentity.manufacturer", of: (c) => c.manufacturer_name },
  { label: "deviceIdentity.productText", of: (c) => c.product_text,
    language: (c) => ({ answered: c.product_text_language, source: c.product_source_language }) },
  { label: "deviceIdentity.orderNumber", of: (c) => c.order_number, mono: true },
];

// The remaining catalogue fields, grouped the way an engineer asks for them:
// which product entry, which physical hardware, which application program.
// `mono: true` marks the identifier-shaped fields (order numbers, program
// ids, versions) — prose names stay in the body face so they don't read
// like codes.
// `language` (AR10 slice 2b, KL-37): which stored language answered the
// text, and the declared language of the package's own text.
type CatalogField = {
  label: MessageKey;
  of: (c: DeviceProductCatalog) => string | null;
  mono?: boolean;
  language?: (c: DeviceProductCatalog) => { answered: string | undefined; source: string | undefined };
};

const CATALOG_GROUPS: { title: MessageKey; fields: CatalogField[] }[] = [
  { title: "deviceIdentity.group.product", fields: [
    { label: "deviceIdentity.manufacturerId", of: (c) => c.manufacturer_id, mono: true },
    { label: "deviceIdentity.catalogItemName", of: (c) => c.catalog_item_name,
      language: (c) => ({ answered: c.catalog_item_name_language, source: c.catalog_item_source_language }) },
    { label: "deviceIdentity.catalogItemNumber", of: (c) => c.catalog_item_number, mono: true },
  ] },
  { title: "deviceIdentity.group.hardware", fields: [
    { label: "deviceIdentity.hardwareName", of: (c) => c.hardware_name },
    { label: "deviceIdentity.hardwareVersion", of: (c) => c.hardware_version, mono: true },
    { label: "deviceIdentity.hardwareSerial", of: (c) => c.hardware_serial_number, mono: true },
  ] },
  { title: "deviceIdentity.group.application", fields: [
    { label: "deviceIdentity.applicationName", of: (c) => c.application_name,
      language: (c) => ({ answered: c.application_name_language, source: c.application_source_language }) },
    { label: "deviceIdentity.applicationNumber", of: (c) => c.application_number, mono: true },
    { label: "deviceIdentity.applicationVersion", of: (c) => c.application_version, mono: true },
    { label: "deviceIdentity.applicationProgramId", of: (c) => c.application_program_id, mono: true },
    { label: "deviceIdentity.maskVersion", of: (c) => c.mask_version, mono: true },
  ] },
];

type IdentityRowData = {
  label: MessageKey;
  value: string;
  mono?: boolean;
  /** Set when the value is the package's own text in a selected product language. */
  fallback?: { selected: string; source: string | undefined };
};

/** The fields of `group` that the database actually filled, in declared order. */
function presentRows(fields: CatalogField[], catalog: DeviceProductCatalog, selected: string | null): IdentityRowData[] {
  return fields.flatMap((f) => {
    const value = f.of(catalog);
    if (value === null) return [];
    const language = f.language?.(catalog);
    const fallback = selected !== null && language && fellBack(value, language.answered)
      ? { selected, source: language.source }
      : undefined;
    return [{ label: f.label, value, mono: f.mono, fallback }];
  });
}

function IdentityFields(props: { rows: IdentityRowData[]; t: Translate }) {
  return <dl className="identity-fields">
    {props.rows.map((row) => (
      <div className="identity-row" key={row.label}>
        <dt>{props.t(row.label)}</dt>
        <dd className={row.mono ? "mono" : undefined}>
          {row.value}
          {row.fallback && <LanguageFallbackBadge selected={row.fallback.selected} source={row.fallback.source} />}
        </dd>
      </div>
    ))}
  </dl>;
}

/**
 * The device's product identity (T16), shown in the workspace's third tab
 * ("Product data"), beside communication objects and parameters.
 *
 * `product_ref`/`program_ref` are printed verbatim in monospace — they are
 * ETS identifiers, and an engineer comparing one against a manufacturer
 * package needs the exact string, not a prettified one. The catalogue half
 * is only ever rendered from a real `catalog`: a `Resolved` verdict with no
 * catalogue behind it (which the server never produces, but the generated
 * type permits) says so in words rather than rendering as a resolved device
 * with a suspiciously empty field list.
 *
 * Fields the product database has no value for are omitted rather than
 * printed as dashes, and the count of omissions is stated at the bottom of
 * the disclosure — a partially-installed manufacturer catalogue is valid
 * database state, and the user should be able to tell "the database is
 * silent here" from "this view only shows six fields".
 */
function DeviceIdentity(props: { product: DeviceProductNode }) {
  const { product_ref, program_ref, catalog, resolution } = props.product;
  const t = useTranslate();
  const [productLanguage] = useProductLanguage();
  // A variant outside the generated union cannot arise from a matching
  // server, only from a frontend older than the one it talks to. It gets its
  // own wording rather than borrowing another variant's: reusing
  // `NoReference`'s would print "No product reference" directly above two
  // references the user can read, which is the exact dishonesty this panel
  // exists to avoid.
  const copy: { badge: MessageKey; explain: MessageKey | null } = RESOLUTION_KEYS[resolution] ?? {
    badge: "deviceIdentity.resolution.unrecognised",
    explain: "deviceIdentity.explain.unrecognised",
  };

  // `NoReference` means both refs are empty at the source, so there is
  // nothing to print verbatim — the sentence says that once instead of two
  // rows each saying it again.
  // The monospace face is for the ref itself; the "not stated" placeholder is
  // prose, and setting it in mono would make an absence look like a value.
  const ref = (label: MessageKey, value: string | null): IdentityRowData =>
    value === null
      ? { label, value: t("deviceIdentity.refNotStated") }
      : { label, value, mono: true };
  const refRows: IdentityRowData[] = resolution === "NoReference" ? [] : [
    ref("deviceIdentity.productRef", product_ref),
    ref("deviceIdentity.programRef", program_ref),
  ];
  // Kept separate from `refRows` rather than subtracted back out of a merged
  // list: the ref rows are not catalogue fields, and "however many rows are
  // in the headline, minus two" stops being true the moment `refRows` is
  // empty — which `NoReference` makes it.
  const headlineCatalogRows = catalog ? presentRows(HEADLINE_FIELDS, catalog, productLanguage) : [];
  const headline = [...refRows, ...headlineCatalogRows];
  const groups = catalog
    ? CATALOG_GROUPS.map((group) => ({ title: group.title, rows: presentRows(group.fields, catalog, productLanguage) }))
    : [];
  // Every catalogue field the database left null, counted across the headline
  // and the groups alike.
  const catalogFields = HEADLINE_FIELDS.length + CATALOG_GROUPS.reduce((n, g) => n + g.fields.length, 0);
  const shownFields = headlineCatalogRows.length + groups.reduce((n, g) => n + g.rows.length, 0);
  const omitted = catalog ? catalogFields - shownFields : 0;

  return <section className="device-identity" aria-label={t("deviceIdentity.title")}>
    <div className="device-identity-head">
      <h3>{t("deviceIdentity.title")}</h3>
      <span className="resolution-badge" data-resolution={resolution}>{t(copy.badge)}</span>
    </div>
    {headline.length > 0 && <IdentityFields rows={headline} t={t} />}
    {copy.explain && <p className="identity-note">{t(copy.explain)}</p>}
    {resolution === "Resolved" && !catalog && (
      <p className="identity-note">{t("deviceIdentity.explain.resolvedWithoutCatalog")}</p>
    )}
    {catalog && (
      <details className="identity-more">
        <summary>{t("deviceIdentity.more")}</summary>
        {groups.map((group) => (
          <div className="identity-group" key={group.title}>
            <h4>{t(group.title)}</h4>
            {group.rows.length === 0
              ? <p className="identity-note">{t("deviceIdentity.groupEmpty")}</p>
              : <IdentityFields rows={group.rows} t={t} />}
          </div>
        ))}
        {omitted > 0 && <p className="identity-note">{t("deviceIdentity.omitted", { count: omitted })}</p>}
      </details>
    )}
  </section>;
}

type ComObjectGroup = {
  key: string;
  channel: ComObjectChannel | null;
  objects: ComObjectNode[];
  order: number;
  firstIndex: number;
};

// Channel keys are opaque per ADR-0052. Do not derive identity from text,
// object names or a parsed channel number; data-half `channel.order` is the
// evaluated document order, including non-contiguous positions.
function groupComObjects(objects: ComObjectNode[]): ComObjectGroup[] {
  const groups = new Map<string, ComObjectGroup>();
  objects.forEach((com, index) => {
    // A missing owner in an older/malformed response is not a channel key.
    const channel = com.activation === "Active" ? com.channel ?? null : null;
    const key = channel === null ? "unassigned" : `channel:${channel.key}`;
    const existing = groups.get(key);
    if (existing) {
      existing.objects.push(com);
    } else {
      groups.set(key, {
        key, channel, objects: [com],
        order: channel?.order ?? Number.MAX_SAFE_INTEGER, firstIndex: index,
      });
    }
  });
  return [...groups.values()].sort((a, b) => a.order - b.order || a.firstIndex - b.firstIndex);
}

const ACTIVATION_KEYS: Record<ComObjectActivation, MessageKey> = {
  Active: "inspector.activation.active",
  Inactive: "inspector.activation.inactive",
  Undetermined: "inspector.activation.undetermined",
  NotEvaluated: "inspector.activation.notEvaluated",
};

function ComObjectRow(props: {
  com: ComObjectNode;
  groupAddresses: GroupAddressNode[];
  onApplied: (tree: ProjectTree) => void;
}) {
  const { com, groupAddresses, onApplied } = props;
  const t = useTranslate();
  const [productLanguage] = useProductLanguage();
  const formatGa = useGroupAddressFormat();
  const dpt = com.dpt ?? com.program_dpt;
  const usesProgramDefault = com.dpt === null && com.program_dpt !== null;
  // A newer server's unknown state must be visible, never treated as inactive.
  const statusKey = ACTIVATION_KEYS[com.activation] ?? "inspector.activation.unknown";
  return <li data-activation={com.activation}>
    <details className="com-object-detail">
      <summary className="com-object-summary">
        <span className="mono">{com.number}</span>
        <span className="com-object-name">
          <strong>{com.name ?? t("inspector.unnamed")}</strong>
          {com.function_text && <small>{com.function_text}</small>}
          <span className="com-object-status" data-activation={com.activation}>{t(statusKey)}</span>
          {com.activation === "Active" && !com.is_active && <small className="com-object-stored-status">{t("inspector.storedInactive")}</small>}
          {com.activation === "Inactive" && com.is_active && <small className="com-object-stored-status">{t("inspector.storedActive")}</small>}
        </span>
        <span className="com-object-effective-dpt">
          {usesProgramDefault && <small className="com-object-dpt-origin">{t("inspector.programDefault")}</small>}
          <span className="mono">{dpt ?? "—"}</span>
          {com.dpt_text && <small>{com.dpt_text}</small>}
          {productLanguage !== null && fellBack(com.dpt_text, com.dpt_text_language) && (
            <LanguageFallbackBadge selected={productLanguage} source={null} />
          )}
        </span>
        <span className="mono ga-address">{com.links.map((link) => (link.address === null ? "—" : formatGa(link.address))).join(", ") || "—"}</span>
      </summary>
      <div className="com-object-edit-fields">
        <DptField com={com} onApplied={onApplied} />
        {com.dpt_layer && <span className="provenance-badge">{com.dpt_layer}</span>}
        <ComObjectDescriptionField com={com} onApplied={onApplied} />
        {com.description_layer && <span className="provenance-badge">{com.description_layer}</span>}
        <ComObjectFlagsRow com={com} onApplied={onApplied} />
        <ul className="group-link-list">
          {com.links.map((link) => <GroupLinkRow key={`${link.ga_id}-${link.direction}`} com={com} link={link} onApplied={onApplied} />)}
          <NewGroupLinkRow com={com} groupAddresses={groupAddresses} onApplied={onApplied} />
        </ul>
      </div>
    </details>
  </li>;
}

// MODEL-01 / ADR-0070, mirroring `Command::LinkComObject`: a device placed
// in one installation links only to that installation's group addresses; a
// device placed nowhere may link to any; one placed in two is refused.
function linkableGroupAddresses(tree: ProjectTree, deviceId: number): GroupAddressNode[] {
  const owner = deviceInstallation(tree, deviceId);
  if (owner) return owner.group_addresses;
  const placed = tree.installations.some((installation) =>
    installation.unassigned.some((device) => device.id === deviceId)
    || installation.topology.some((area) => area.lines.some((line) => line.devices.some((device) => device.id === deviceId))));
  return placed ? [] : tree.installations.flatMap((installation) => installation.group_addresses);
}

export function DeviceWorkspace(props: {
  detail: DeviceDetail; tree: ProjectTree; onApplied: (tree: ProjectTree) => void;
}) {
  const { detail, tree, onApplied } = props;
  const groupAddresses = linkableGroupAddresses(tree, detail.id);
  const parameterState = useDeviceParameters(detail.id, tree);
  const t = useTranslate();
  const [tab, setTab] = useState(0);
  const [expandedGroups, setExpandedGroups] = useState<{ deviceId: number; keys: Set<string> }>({
    deviceId: detail.id, keys: new Set(),
  });
  const groups = groupComObjects(detail.com_objects);
  // One array and index arithmetic derived from its length:
  // the previous `1 - tab` toggle silently encoded "there are exactly two
  // tabs" three times over (it also hardcoded `End` and treated both arrow
  // keys as the same key, which a left-arrow-only test could never catch).
  const tabs = [t("inspector.communicationObjects"), t("workbench.parameters"), t("deviceIdentity.tab"),
    t("parameters.diagnosticsTab"), t("parameters.restrictedTab")];
  return <section className="device-workspace">
    <header className="workspace-heading"><div><h2>{detail.name}</h2><span className="mono">{detail.address ?? t("workbench.unassigned")}</span></div></header>
    <div className="device-tabs" role="tablist" aria-label={detail.name} onKeyDown={(e) => {
      if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(e.key)) return;
      e.preventDefault();
      const step = e.key === "ArrowRight" ? 1 : -1;
      const next = e.key === "Home" ? 0
        : e.key === "End" ? tabs.length - 1
        : (tab + step + tabs.length) % tabs.length;
      setTab(next); e.currentTarget.querySelectorAll<HTMLButtonElement>("button")[next].focus();
    }}>
      {tabs.map((label, index) => <button key={label} id={`device-tab-${detail.id}-${index}`} role="tab" aria-selected={tab === index} aria-controls={`device-panel-${detail.id}-${index}`} tabIndex={tab === index ? 0 : -1} onClick={() => setTab(index)}>{label}</button>)}
    </div>
    {/* `tabIndex={0}` on every panel, not only the ones that can end up
        with nothing focusable inside them: a panel's content is data-driven
        (a device with no communication objects, an application with no
        parameters, an identity that is one sentence of prose under
        `NoReference`) so "does this panel contain a tab stop?" cannot be
        answered here at all. WAI-ARIA APG asks for the tab stop exactly when
        the content is unfocusable; applying it unconditionally costs one
        extra stop on a populated panel and never strands the content of an
        empty one. A hidden panel is not focusable, so only the selected one
        is ever in the tab order. */}
    <div role="tabpanel" id={`device-panel-${detail.id}-0`} aria-labelledby={`device-tab-${detail.id}-0`} hidden={tab !== 0} tabIndex={0}>
      <h3>{t("inspector.communicationObjects")}</h3>
      <div className="com-object-groups">
        {groups.map((group) => {
          const label = group.channel === null
            ? t("inspector.noEvaluatedChannel")
            : group.channel.kind === "ChannelIndependentBlock"
              ? t("inspector.channelIndependent")
              : group.channel.text || group.channel.name || t("inspector.untitledChannel");
          const open = expandedGroups.deviceId === detail.id && expandedGroups.keys.has(group.key);
          return <details className="com-object-channel" key={group.key} open={open} onToggle={(e) => {
            const nextOpen = e.currentTarget.open;
            setExpandedGroups((previous) => {
              const keys = new Set(previous.deviceId === detail.id ? previous.keys : []);
              if (nextOpen) keys.add(group.key); else keys.delete(group.key);
              return { deviceId: detail.id, keys };
            });
          }}>
            <summary className="com-object-channel-summary">
              <strong>{label}</strong>
              {group.channel?.kind === "Channel" && group.channel.text && group.channel.name &&
                <span className="com-object-channel-name">{t("inspector.channelName", { value: group.channel.name })}</span>}
              {group.channel?.kind === "Channel" && group.channel.number &&
                <span className="com-object-channel-number">{t("inspector.channelNumber", { value: group.channel.number })}</span>}
              <span className="com-object-channel-count">{t(group.objects.length === 1 ? "inspector.objectCount.one" : "inspector.objectCount.other", { count: group.objects.length })}</span>
            </summary>
            <ul className="com-object-list">
              {group.objects.map((com) => <ComObjectRow key={com.id} com={com} groupAddresses={groupAddresses} onApplied={onApplied} />)}
            </ul>
          </details>;
        })}
      </div>

    </div>
    <div role="tabpanel" id={`device-panel-${detail.id}-1`} aria-labelledby={`device-tab-${detail.id}-1`} hidden={tab !== 1} tabIndex={0}>
      {/* `ParameterPanel`'s `onValueApplied` now hands back the server's
          own freshly rebuilt `ProjectTree` (T3 fix round 1, item 6) — the one
          `apply(state, cmd)` already built from the genuine post-write
          `CommandStack`, not a hand-built `{ ...tree, can_undo: true,
          can_redo: false, is_modified: true }` overlay assembled from a tree this component
          happened to be holding. `onApplied` takes exactly that shape, so
          it wires straight through, the same as every field above it. */}
      <ParameterPanelContent deviceId={detail.id} state={parameterState} onValueApplied={onApplied} />
    </div>
    {/* Hidden, not unmounted — the same shape as the parameter panel above,
        whose fetch is keyed to its mount and must not restart on every tab
        switch. */}
    <div role="tabpanel" id={`device-panel-${detail.id}-2`} aria-labelledby={`device-tab-${detail.id}-2`} hidden={tab !== 2} tabIndex={0}>
      <DeviceIdentity product={detail.product} />
    </div>
    <div role="tabpanel" id={`device-panel-${detail.id}-3`} aria-labelledby={`device-tab-${detail.id}-3`} hidden={tab !== 3} tabIndex={0}>
      <ParameterPanelContent deviceId={detail.id} state={parameterState} view="diagnostics" onValueApplied={onApplied} />
    </div>
    <div role="tabpanel" id={`device-panel-${detail.id}-4`} aria-labelledby={`device-tab-${detail.id}-4`} hidden={tab !== 4} tabIndex={0}>
      <ParameterPanelContent deviceId={detail.id} state={parameterState} view="restricted" onValueApplied={onApplied} />
    </div>
  </section>;
}

// MODEL-02 / ADR-0071: a device listed in more than one topology slot is
// repaired only by the user naming the slot to keep. Addresses, links,
// parameters and building placement are left alone by the server.
function DevicePlacementRepair(props: {
  detail: DeviceDetail;
  tree: ProjectTree;
  onApplied: (tree: ProjectTree) => void;
}) {
  const { detail, tree, onApplied } = props;
  const t = useTranslate();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const slots = devicePlacementSlots(tree, detail.id);
  const total = slots.reduce((sum, slot) => sum + slot.count, 0);
  if (total <= 1) return null;

  async function keep(slot: (typeof slots)[number]) {
    setBusy(true);
    setError(null);
    try {
      onApplied(await api.repairDevicePlacement(detail.id, slot.kind === "line"
        ? { lineId: slot.line.id } : { unassignedInstallationId: slot.installation.id }));
    } catch (e) {
      setError(api.errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="placement-repair">
      <h3>{t("inspector.placementConflict")}</h3>
      <p className="inspector-description">{t("inspector.placementConflictHint", { count: total })}</p>
      <ul>
        {slots.map((slot) => (
          <li key={slot.kind === "line" ? `line-${slot.installation.id}-${slot.line.id}` : `unassigned-${slot.installation.id}`}>
            <span>
              {slot.kind === "line"
                ? t("inspector.placementLine", { area: slot.area.address, line: slot.line.address,
                  name: slot.line.name, installation: slot.installation.name })
                : t("inspector.placementUnassigned", { installation: slot.installation.name })}
              {slot.count > 1 && ` ${t("inspector.placementListedTimes", { count: slot.count })}`}
            </span>
            <button type="button" disabled={busy} onClick={() => void keep(slot)}>
              {t("inspector.keepPlacement")}
            </button>
          </li>
        ))}
      </ul>
      {error && <span className="field-error">{error}</span>}
    </section>
  );
}

// MODEL-02 / ADR-0071: the same line listed by several areas of one
// installation. Offered only when every occurrence is the same line (same
// name, address and devices) in at least two areas; two different lines that
// share an id are a duplicate-id problem with no repair here.
function LineOwnerRepair(props: { tree: ProjectTree; lineId: number; onApplied: (tree: ProjectTree) => void }) {
  const { tree, lineId, onApplied } = props;
  const t = useTranslate();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const holders = tree.installations.filter((installation) =>
    installation.topology.some((area) => area.lines.some((line) => line.id === lineId)));
  const areas = holders.length === 1
    ? holders[0].topology.filter((area) => area.lines.some((line) => line.id === lineId)) : [];
  const shapes = new Set(areas.flatMap((area) => area.lines.filter((line) => line.id === lineId))
    .map((line) => JSON.stringify([line.name, line.address, line.devices.map((device) => device.id)])));
  if (areas.length < 2 || shapes.size !== 1) return null;

  async function keep(areaId: number) {
    setBusy(true);
    setError(null);
    try {
      onApplied(await api.repairLineOwner(lineId, areaId));
    } catch (e) {
      setError(api.errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="placement-repair">
      <p className="inspector-description">{t("inspector.lineOwnerConflictHint", { count: areas.length })}</p>
      <ul>
        {areas.map((area) => (
          <li key={area.id}>
            <span>{t("inspector.areaLabel", { address: area.address, name: area.name })}</span>
            <button type="button" disabled={busy} onClick={() => void keep(area.id)}>{t("inspector.keepLineOwner")}</button>
          </li>
        ))}
      </ul>
      {error && <span className="field-error">{error}</span>}
    </section>
  );
}

function DeviceInspector(props: {
  propertiesOnly?: boolean;
  detail: DeviceDetail;
  tree: ProjectTree;
  // Delete is offered for a device placed in the topology of exactly one
  // installation (ADR-0070), the reachability `findDeviceLine` reports for
  // `LineMoveField`.
  canDelete: boolean;
  onApplied: (tree: ProjectTree) => void;
  onDeleted: (tree: ProjectTree) => void;
}) {
  const { detail, tree, canDelete, onApplied, onDeleted } = props;
  const t = useTranslate();
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
          {restrictedToOneInstallationMessage(t, "delete", "inspector.entity.devices")}
        </p>
      )}
      {error && <span className="field-error">{error}</span>}
      <DevicePlacementRepair detail={detail} tree={tree} onApplied={onApplied} />
      <AddressField detail={detail} tree={tree} onApplied={onApplied} />
      <LineMoveField detail={detail} tree={tree} onApplied={onApplied} />
      <BuildingPartMoveField detail={detail} tree={tree} onApplied={onApplied} />
      <DeviceDescriptionField detail={detail} onApplied={onApplied} />
      {!props.propertiesOnly && <DeviceWorkspace detail={detail} tree={tree} onApplied={onApplied} />}
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
  const formatGa = useGroupAddressFormat();
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

  const counts = linkDirectionCounts(ga);

  return (
    <div className="inspector">
      <h2>{ga.name}</h2>
      <p className="inspector-address ga-address">{formatGa(ga.address)}</p>
      <dl className="inspector-facts">
        <dt>{t("addressTable.dpt")}</dt>
        <dd className={hasDptConflict(ga) ? "mono dpt-conflict" : "mono"}>
          {dptText(t, ga)}
          {hasDptConflict(ga) && <small> {t("addressTable.dptConflict")}</small>}
        </dd>
        {ga.dpt_detail && (
          <>
            <dt>{t("gaType.declared")}</dt>
            <dd className="mono">{declaredDptText(t, ga.dpt_detail.declared)}</dd>
            <dt>{t("gaType.linked")}</dt>
            <dd className="mono">
              {ga.dpt_detail.linked.length === 0 ? t("addressTable.noDpt") : ga.dpt_detail.linked.join(" · ")}
            </dd>
          </>
        )}
        <dt>{t("addressTable.links")}</dt>
        <dd>
          {counts.total === 0
            ? t("addressTable.noLinks")
            : `${t("addressTable.linkTotal", { count: counts.total })} · ${t("addressTable.linkCounts", { senders: counts.senders, receivers: counts.receivers })}`}
        </dd>
      </dl>
      {ga.dpt_detail && (
        <p className={ga.dpt_detail.outcome === "SizeConflict" ? "dpt-outcome dpt-conflict" : "dpt-outcome"}>
          {t(DPT_OUTCOME_KEYS[ga.dpt_detail.outcome])}
        </p>
      )}
      {ga.links.length > 0 && <ul className="ga-linked-devices" aria-label={t("workbench.devices")}>
        {[...new Map(ga.links.map((link) => [link.device_id, link])).values()].map((link) => <li key={link.device_id}>
          <DeviceLink deviceId={link.device_id}>{link.device_name ?? t("addressTable.unknownDevice", { id: link.device_id })}</DeviceLink>
          {link.device_address && <small className="mono"> {link.device_address}</small>}
        </li>)}
      </ul>}
      {canDelete ? (
        <button onClick={remove}>{t("inspector.delete")}</button>
      ) : (
        <p className="inspector-description">
          {restrictedToOneInstallationMessage(t, "delete", "inspector.entity.groupAddresses")}
        </p>
      )}
      {error && <span className="field-error">{error}</span>}
    </div>
  );
}

const DPT_OUTCOME_KEYS: Record<GroupAddressDptOutcome, MessageKey> = {
  Declared: "gaType.outcome.Declared",
  DeclaredDiffersFromLinked: "gaType.outcome.DeclaredDiffersFromLinked",
  SizeConflict: "gaType.outcome.SizeConflict",
  Unverifiable: "gaType.outcome.Unverifiable",
  DeclarationNotLifted: "gaType.outcome.DeclarationNotLifted",
  Inferred: "gaType.outcome.Inferred",
};

// The address's own `DatapointType` exactly as stored (ADR-0078): an absent,
// empty or unreadable declaration is named as such, never shown as a type.
function declaredDptText(t: Translate, declared: DeclaredDptNode): string {
  switch (declared.state) {
    case "Value":
      return declared.text ?? "";
    case "Malformed":
      return t("gaType.declaredMalformed", { text: declared.text ?? "" });
    case "Empty":
      return t("gaType.declaredEmpty");
    case "Absent":
      return t("gaType.declaredAbsent");
  }
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
  ranges: GroupRangeNode[];
  // Group-range commands currently edit only the first installation.
  canEdit: boolean;
  onApplied: (tree: ProjectTree) => void;
  onDeleted: (tree: ProjectTree) => void;
}) {
  const { range, ranges, canEdit, onApplied, onDeleted } = props;
  const t = useTranslate();
  const formatGa = useGroupAddressFormat();
  const [error, setError] = useState<string | null>(null);
  const [moving, setMoving] = useState(false);
  const uniqueIds = new Set(ranges.map((candidate) => candidate.id)).size === ranges.length;
  const unambiguous = uniqueIds &&
    (range.parent === null || ranges.some((candidate) => candidate.id === range.parent));
  const childrenByParent = new Map<number, number[]>();
  for (const candidate of ranges) {
    if (candidate.parent === null) continue;
    const children = childrenByParent.get(candidate.parent) ?? [];
    children.push(candidate.id);
    childrenByParent.set(candidate.parent, children);
  }
  const excluded = new Set([range.id]);
  const queue = [range.id];
  for (let index = 0; index < queue.length; index += 1) {
    for (const child of childrenByParent.get(queue[index]) ?? []) {
      if (excluded.has(child)) continue;
      excluded.add(child);
      queue.push(child);
    }
  }

  async function move(parentId: number | null) {
    if (moving || parentId === range.parent) return;
    setError(null);
    setMoving(true);
    try {
      const updated = await api.moveGroupRange(range.id, parentId);
      onApplied(updated);
    } catch (e) {
      setError(api.errorMessage(e));
    } finally {
      setMoving(false);
    }
  }

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
      <p className="inspector-address ga-address">
        {formatGa(range.start)}–{formatGa(range.end)}
      </p>
      {canEdit ? (
        <>
          <GroupRangeNameField range={range} onApplied={onApplied} />
          {unambiguous ? (
            <label className="inspector-field">
              {t("inspector.parentGroupRange")}
              <select
                value={range.parent ?? ""}
                disabled={moving}
                onChange={(event) => void move(event.target.value === "" ? null : Number(event.target.value))}
              >
                <option value="">{t("inspector.rootRange")}</option>
                {ranges.filter((candidate) => !excluded.has(candidate.id)).map((candidate) => (
                  <option key={candidate.id} value={candidate.id}>
                    {candidate.name} ({formatGa(candidate.start)}–{formatGa(candidate.end)})
                  </option>
                ))}
              </select>
            </label>
          ) : (
            <p className="inspector-description">{t("inspector.rangePlacementAmbiguous")}</p>
          )}
          <button onClick={remove}>{t("inspector.delete")}</button>
        </>
      ) : (
        <p className="inspector-description">
          {restrictedToOneInstallationMessage(t, "renameMoveAndDelete", "inspector.entity.groupRanges")}
        </p>
      )}
      {error && <span className="field-error">{error}</span>}
    </div>
  );
}

// Area and line names use the same command-backed edit path as other
// project structure Properties; address numbers remain fixed by topology.
const PROJECT_GROUP_STYLES: readonly api.GroupAddressStyle[] = ["ThreeLevel", "TwoLevel", "Free"];

function ProjectInspector(props: { tree: ProjectTree; onApplied: (tree: ProjectTree) => void }) {
  const t = useTranslate();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const inFlight = useRef(false);
  useEffect(() => { setError(null); }, [props.tree.group_address_style]);

  async function apply(style: api.GroupAddressStyle) {
    if (inFlight.current || style === props.tree.group_address_style) return;
    inFlight.current = true;
    setBusy(true);
    setError(null);
    try {
      props.onApplied(await api.setGroupAddressStyle(style));
    } catch (e) {
      setError(api.errorMessage(e));
    } finally {
      inFlight.current = false;
      setBusy(false);
    }
  }

  return (
    <div className="inspector">
      <h2>{t("inspector.project")}</h2>
      <dl className="inspector-facts">
        <dt>{t("inspector.groupAddressStyle")}</dt>
        <dd className="mono">{props.tree.group_address_style}</dd>
      </dl>
      <label className="inspector-field">
        {t("inspector.groupAddressStyle")}
        <select aria-label={t("inspector.groupAddressStyle")} value={props.tree.group_address_style}
          disabled={busy} onChange={(event) => {
            const style = PROJECT_GROUP_STYLES.find((candidate) => candidate === event.currentTarget.value);
            if (style) void apply(style);
          }}>
          {!PROJECT_GROUP_STYLES.some((style) => style === props.tree.group_address_style) && (
            <option value={props.tree.group_address_style} disabled>{props.tree.group_address_style}</option>
          )}
          {PROJECT_GROUP_STYLES.map((style) => (
            <option key={style} value={style}>{t(`newProject.style.${style}`)}</option>
          ))}
        </select>
      </label>
      {error && <p className="field-error" role="alert">{error}</p>}
      {/* MODEL-01: `Command::RenameInstallation`, one undo step each. */}
      <h3>{t("inspector.installations")}</h3>
      {props.tree.installations.map((installation, index) => (
        <NameField key={installation.id} id={installation.id} name={installation.name}
          label={t("inspector.name")} inputLabel={t("inspector.installationName", { n: index + 1 })}
          rename={(value) => api.renameInstallation(installation.id, value)} onApplied={props.onApplied} />
      ))}
    </div>
  );
}

// A name edited in place and committed on blur/Enter; a refusal restores
// the authoritative name and shows the server's reason.
function NameField(props: {
  id: number;
  name: string;
  label: string;
  inputLabel?: string;
  rename: (name: string) => Promise<ProjectTree>;
  onApplied: (tree: ProjectTree) => void;
}) {
  const { id, name, rename, onApplied } = props;
  const [value, setValue] = useState(name);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    setValue(name);
    setError(null);
  }, [id, name]);

  async function apply() {
    if (value === name || value.trim() === "") {
      setValue(name);
      return;
    }
    setError(null);
    try {
      onApplied(await rename(value));
    } catch (e) {
      setError(api.errorMessage(e));
      setValue(name);
    }
  }

  return <label className="inspector-field">
    {props.label}
    <input value={value} aria-label={props.inputLabel} onChange={(e) => setValue(e.target.value)} onBlur={apply}
      onKeyDown={(e) => { if (e.key === "Enter") (e.target as HTMLInputElement).blur(); }} />
    {error && <span className="field-error">{error}</span>}
  </label>;
}

function TopologyNameField(props: {
  kind: "area" | "line";
  id: number;
  name: string;
  onApplied: (tree: ProjectTree) => void;
}) {
  const { kind, id } = props;
  const t = useTranslate();
  return <NameField id={id} name={props.name} label={t("inspector.name")} onApplied={props.onApplied}
    rename={(value) => kind === "area" ? api.renameArea(id, value) : api.renameLine(id, value)} />;
}

function AreaInspector(props: {
  area: AreaNode;
  // Topology commands currently edit only the first installation.
  canEdit: boolean;
  onApplied: (tree: ProjectTree) => void;
  onDeleted: (tree: ProjectTree) => void;
}) {
  const { area, canEdit, onApplied, onDeleted } = props;
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
      {canEdit ? (
        <>
          <TopologyNameField kind="area" id={area.id} name={area.name} onApplied={onApplied} />
          <button onClick={remove}>{t("inspector.delete")}</button>
        </>
      ) : (
        <p className="inspector-description">
          {restrictedToOneInstallationMessage(t, "renameAndDelete", "inspector.entity.areas")}
        </p>
      )}
      {error && <span className="field-error">{error}</span>}
    </div>
  );
}

function LineInspector(props: {
  line: LineNode;
  tree: ProjectTree;
  canEdit: boolean;
  onApplied: (tree: ProjectTree) => void;
  onDeleted: (tree: ProjectTree) => void;
}) {
  const { line, tree, canEdit, onApplied, onDeleted } = props;
  const t = useTranslate();
  const [error, setError] = useState<string | null>(null);
  const [moving, setMoving] = useState(false);
  const areas = owningInstallation(tree, "line", line.id)?.topology ?? [];
  const owners = areas.flatMap((area) =>
    area.lines.filter((candidate) => candidate.id === line.id).map(() => area),
  );
  const occurrences = tree.installations.reduce(
    (total, installation) => total + installation.topology.reduce(
      (count, area) => count + area.lines.filter((candidate) => candidate.id === line.id).length,
      0,
    ),
    0,
  );
  const unambiguous = owners.length === 1 && occurrences === 1 &&
    new Set(areas.map((area) => area.id)).size === areas.length;
  const currentAreaId = owners[0]?.id;

  async function move(areaId: number) {
    if (moving || areaId === currentAreaId) return;
    setError(null);
    setMoving(true);
    try {
      const updated = await api.moveLineToArea(line.id, areaId);
      onApplied(updated);
    } catch (e) {
      setError(api.errorMessage(e));
    } finally {
      setMoving(false);
    }
  }

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
      {canEdit ? (
        <>
          <TopologyNameField kind="line" id={line.id} name={line.name} onApplied={onApplied} />
          {unambiguous ? (
            <label className="inspector-field">
              {t("inspector.assignedArea")}
              <select
                value={currentAreaId ?? ""}
                disabled={moving}
                onChange={(event) => void move(Number(event.target.value))}
              >
                {areas.map((area) => (
                  <option key={area.id} value={area.id}>
                    {t("inspector.areaLabel", { address: area.address, name: area.name })}
                  </option>
                ))}
              </select>
            </label>
          ) : (
            <p className="inspector-description">{t("inspector.linePlacementAmbiguous")}</p>
          )}
          <button onClick={remove}>{t("inspector.delete")}</button>
        </>
      ) : (
        <p className="inspector-description">
          {restrictedToOneInstallationMessage(t, "renameMoveAndDelete", "inspector.entity.lines")}
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
  tree: ProjectTree;
  // Topology commands currently target only the first installation.
  canEdit: boolean;
  onApplied: (tree: ProjectTree) => void;
  onDeleted: (tree: ProjectTree) => void;
}) {
  const { node, path, tree, canEdit, onApplied, onDeleted } = props;
  const t = useTranslate();
  const [error, setError] = useState<string | null>(null);
  const [moving, setMoving] = useState(false);
  const allParts = flattenBuildingParts(owningInstallation(tree, "building_part", node.id)?.buildings ?? [], []);
  const occurrences = allParts.filter((entry) => entry.node.id === node.id).length;
  const parentMatches = allParts.filter((entry) =>
    entry.node.children.some((child) => child.id === node.id),
  );
  const uniqueIds = new Set(allParts.map((entry) => entry.node.id)).size === allParts.length;
  const unambiguous = occurrences === 1 && parentMatches.length <= 1 && uniqueIds;
  const currentParentId = parentMatches[0]?.node.id ?? null;
  const excluded = new Set([
    node.id,
    ...flattenBuildingParts(node.children, []).map((entry) => entry.node.id),
  ]);

  async function move(parentId: number | null) {
    if (moving || parentId === currentParentId) return;
    setError(null);
    setMoving(true);
    try {
      const updated = await api.moveBuildingPart(node.id, parentId);
      onApplied(updated);
    } catch (e) {
      setError(api.errorMessage(e));
    } finally {
      setMoving(false);
    }
  }

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
          {unambiguous ? (
            <label className="inspector-field">
              {t("inspector.parentBuildingPart")}
              <select
                value={currentParentId ?? ""}
                disabled={moving}
                onChange={(event) => void move(event.target.value === "" ? null : Number(event.target.value))}
              >
                <option value="">{t("inspector.rootBuilding")}</option>
                {allParts.filter((entry) => !excluded.has(entry.node.id)).map(({ node: target, path }) => (
                  <option key={target.id} value={target.id}>{path} (#{target.id})</option>
                ))}
              </select>
            </label>
          ) : (
            <p className="inspector-description">{t("inspector.buildingPlacementAmbiguous")}</p>
          )}
          <button onClick={remove}>{t("inspector.delete")}</button>
        </>
      ) : (
        <p className="inspector-description">
          {restrictedToOneInstallationMessage(
            t,
            "renameMoveAndDelete",
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
  propertiesOnly?: boolean;
  selection: Selection;
  tree: ProjectTree;
  deviceDetail: DeviceDetail | null;
  onApplied: (tree: ProjectTree) => void;
  onDeleted: (tree: ProjectTree) => void;
}) {
  const { selection, tree, deviceDetail, onApplied, onDeleted } = props;
  const t = useTranslate();

  if (selection.kind === "project") {
    return <ProjectInspector tree={tree} onApplied={onApplied} />;
  }

  if (selection.kind === "device") {
    if (!deviceDetail || deviceDetail.id !== selection.id) return null;
    const canDelete = deviceInstallation(tree, deviceDetail.id) !== undefined;
    return (
      <DeviceInspector
        propertiesOnly={props.propertiesOnly}
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
    const canDelete = owningInstallation(tree, "group_address", ga.id) !== undefined;
    return <GroupAddressInspector ga={ga} canDelete={canDelete} onDeleted={onDeleted} />;
  }

  const matchingStructureIds = tree.installations.reduce((count, installation) => {
    if (selection.kind === "group_range") {
      return count + installation.group_ranges.filter((range) => range.id === selection.id).length;
    }
    if (selection.kind === "area") {
      return count + installation.topology.filter((area) => area.id === selection.id).length;
    }
    if (selection.kind === "line") {
      return count + installation.topology.reduce(
        (lines, area) => lines + area.lines.filter((line) => line.id === selection.id).length, 0,
      );
    }
    return count + flattenBuildingParts(installation.buildings, []).filter(
      ({ node }) => node.id === selection.id,
    ).length;
  }, 0);
  if (matchingStructureIds > 1) {
    return <>
      <p role="alert" className="inspector-description">{t("inspector.duplicateStructureId")}</p>
      {selection.kind === "line" && <LineOwnerRepair tree={tree} lineId={selection.id} onApplied={onApplied} />}
    </>;
  }

  if (selection.kind === "group_range") {
    const range = findGroupRange(tree, selection.id);
    if (!range) return null;
    const owner = owningInstallation(tree, "group_range", range.id);
    return (
      <GroupRangeInspector
        range={range}
        ranges={owner?.group_ranges ?? []}
        canEdit={owner !== undefined}
        onApplied={onApplied}
        onDeleted={onDeleted}
      />
    );
  }

  if (selection.kind === "area") {
    const area = findArea(tree, selection.id);
    if (!area) return null;
    const canEdit = owningInstallation(tree, "area", area.id) !== undefined;
    return <AreaInspector area={area} canEdit={canEdit} onApplied={onApplied} onDeleted={onDeleted} />;
  }

  if (selection.kind === "line") {
    const line = findLine(tree, selection.id);
    if (!line) return null;
    const canEdit = owningInstallation(tree, "line", line.id) !== undefined;
    return <LineInspector line={line} tree={tree} canEdit={canEdit} onApplied={onApplied} onDeleted={onDeleted} />;
  }

  const found = findBuildingPart(tree, selection.id);
  if (!found) return null;
  const canEdit = owningInstallation(tree, "building_part", found.node.id) !== undefined;
  return (
    <BuildingPartInspector
      node={found.node}
      path={found.path}
      tree={tree}
      canEdit={canEdit}
      onApplied={onApplied}
      onDeleted={onDeleted}
    />
  );
}
