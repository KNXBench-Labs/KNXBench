/** Properties inspector showing and editing details for whatever tree entity is selected. */
import { useEffect, useState } from "react";
import * as api from "./api";
import type { DeviceDetail } from "./bindings/DeviceDetail";
import type { ComObjectNode } from "./bindings/ComObjectNode";
import type { DeviceProductNode } from "./bindings/DeviceProductNode";
import type { DeviceProductCatalog } from "./bindings/DeviceProductCatalog";
import type { ProductResolution } from "./bindings/ProductResolution";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { AreaNode } from "./bindings/AreaNode";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { GroupLinkNode } from "./bindings/GroupLinkNode";
import type { GroupRangeNode } from "./bindings/GroupRangeNode";
import type { LineNode } from "./bindings/LineNode";
import type { BuildingNode } from "./bindings/BuildingNode";
import type { Selection } from "./selection";
import ParameterPanel from "./ParameterPanel";
import HelpTip from "./HelpTip";
import { useTranslate, type MessageKey, type Translate } from "./i18n";
import {
  directionLabel,
  dptText,
  hasDptConflict,
  linkDirectionCounts,
} from "./groupAddressView";
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
      {/* The five `title` attributes below stay — they are a fine mouse
          affordance for a one-word flag name. The tip is the keyboard's
          way in, and carries the sentence the letters cannot (ADR-0024). */}
      <HelpTip labelKey="help.tip.comFlags.label" textKey="help.tip.comFlags.text" />
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
  { label: "deviceIdentity.productText", of: (c) => c.product_text },
  { label: "deviceIdentity.orderNumber", of: (c) => c.order_number, mono: true },
];

// The remaining catalogue fields, grouped the way an engineer asks for them:
// which product entry, which physical hardware, which application program.
// `mono: true` marks the identifier-shaped fields (order numbers, program
// ids, versions) — prose names stay in the body face so they don't read
// like codes.
type CatalogField = { label: MessageKey; of: (c: DeviceProductCatalog) => string | null; mono?: boolean };

const CATALOG_GROUPS: { title: MessageKey; fields: CatalogField[] }[] = [
  { title: "deviceIdentity.group.product", fields: [
    { label: "deviceIdentity.manufacturerId", of: (c) => c.manufacturer_id, mono: true },
    { label: "deviceIdentity.catalogItemName", of: (c) => c.catalog_item_name },
    { label: "deviceIdentity.catalogItemNumber", of: (c) => c.catalog_item_number, mono: true },
  ] },
  { title: "deviceIdentity.group.hardware", fields: [
    { label: "deviceIdentity.hardwareName", of: (c) => c.hardware_name },
    { label: "deviceIdentity.hardwareVersion", of: (c) => c.hardware_version, mono: true },
    { label: "deviceIdentity.hardwareSerial", of: (c) => c.hardware_serial_number, mono: true },
  ] },
  { title: "deviceIdentity.group.application", fields: [
    { label: "deviceIdentity.applicationName", of: (c) => c.application_name },
    { label: "deviceIdentity.applicationNumber", of: (c) => c.application_number, mono: true },
    { label: "deviceIdentity.applicationVersion", of: (c) => c.application_version, mono: true },
    { label: "deviceIdentity.applicationProgramId", of: (c) => c.application_program_id, mono: true },
    { label: "deviceIdentity.maskVersion", of: (c) => c.mask_version, mono: true },
  ] },
];

type IdentityRowData = { label: MessageKey; value: string; mono?: boolean };

/** The fields of `group` that the database actually filled, in declared order. */
function presentRows(fields: CatalogField[], catalog: DeviceProductCatalog): IdentityRowData[] {
  return fields.flatMap((f) => {
    const value = f.of(catalog);
    return value === null ? [] : [{ label: f.label, value, mono: f.mono }];
  });
}

function IdentityFields(props: { rows: IdentityRowData[]; t: Translate }) {
  return <dl className="identity-fields">
    {props.rows.map((row) => (
      <div className="identity-row" key={row.label}>
        <dt>{props.t(row.label)}</dt>
        <dd className={row.mono ? "mono" : undefined}>{row.value}</dd>
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
  const headlineCatalogRows = catalog ? presentRows(HEADLINE_FIELDS, catalog) : [];
  const headline = [...refRows, ...headlineCatalogRows];
  const groups = catalog
    ? CATALOG_GROUPS.map((group) => ({ title: group.title, rows: presentRows(group.fields, catalog) }))
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

export function DeviceWorkspace(props: {
  detail: DeviceDetail; tree: ProjectTree; onApplied: (tree: ProjectTree) => void;
}) {
  const { detail, tree, onApplied } = props;
  const groupAddresses = tree.installations[0]?.group_addresses ?? [];
  const t = useTranslate();
  const [tab, setTab] = useState(0);
  // One array, three panels, and index arithmetic derived from its length:
  // the previous `1 - tab` toggle silently encoded "there are exactly two
  // tabs" three times over (it also hardcoded `End` and treated both arrow
  // keys as the same key, which a left-arrow-only test could never catch).
  const tabs = [t("inspector.communicationObjects"), t("workbench.parameters"), t("deviceIdentity.tab")];
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
    {/* `tabIndex={0}` on all three panels, not only the ones that can end up
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
      <ul className="com-object-list">
        {detail.com_objects.map((com) => (
          <li key={com.id}>
            <details className="com-object-detail">
            <summary className="com-object-summary"><span className="mono">{com.number}</span><strong>{com.name ?? t("inspector.unnamed")}</strong><span className="mono">{com.dpt ?? "—"}</span><span className="mono">{com.links.map((link) => link.address ?? "—").join(", ") || "—"}</span></summary>
            <div className="com-object-edit-fields">
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
            </div>
            </details>
          </li>
        ))}
      </ul>

    </div>
    <div role="tabpanel" id={`device-panel-${detail.id}-1`} aria-labelledby={`device-tab-${detail.id}-1`} hidden={tab !== 1} tabIndex={0}>
      {/* `ParameterPanel`'s `onValueApplied` now hands back the server's
          own freshly rebuilt `ProjectTree` (T3 fix round 1, item 6) — the one
          `apply(state, cmd)` already built from the genuine post-write
          `CommandStack`, not a hand-built `{ ...tree, can_undo: true,
          can_redo: false }` overlay assembled from a tree this component
          happened to be holding. `onApplied` takes exactly that shape, so
          it wires straight through, the same as every field above it. */}
      <ParameterPanel deviceId={detail.id} onValueApplied={onApplied} />
    </div>
    {/* Hidden, not unmounted — the same shape as the parameter panel above,
        whose fetch is keyed to its mount and must not restart on every tab
        switch. */}
    <div role="tabpanel" id={`device-panel-${detail.id}-2`} aria-labelledby={`device-tab-${detail.id}-2`} hidden={tab !== 2} tabIndex={0}>
      <DeviceIdentity product={detail.product} />
    </div>
  </section>;
}

function DeviceInspector(props: {
  propertiesOnly?: boolean;
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
      <p className="inspector-address">{ga.address}</p>
      <dl className="inspector-facts">
        <dt>{t("addressTable.dpt")}</dt>
        <dd className={hasDptConflict(ga) ? "mono dpt-conflict" : "mono"}>
          {dptText(t, ga)}
          {hasDptConflict(ga) && <small> {t("addressTable.dptConflict")}</small>}
        </dd>
        <dt>{t("addressTable.links")}</dt>
        <dd>
          {counts.total === 0
            ? t("addressTable.noLinks")
            : `${t("addressTable.linkTotal", { count: counts.total })} · ${t("addressTable.linkCounts", { senders: counts.senders, receivers: counts.receivers })}`}
        </dd>
      </dl>
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
// The project node's own panel (KNOWN_LIMITATIONS.md §84) — display only:
// no button, dropdown, or route call here restyles the project. Undoing
// the closed limitation's own "afterwards never seen" complaint needs no
// more than reading `tree.group_address_style` back onto the screen; a
// restyle affordance is deliberately out of this cycle's scope (dispatcher
// ruling: no UI trigger unless trivially additive, and a style change
// this consequential is not).
function ProjectInspector(props: { tree: ProjectTree }) {
  const t = useTranslate();
  return (
    <div className="inspector">
      <h2>{t("inspector.project")}</h2>
      <dl className="inspector-facts">
        <dt>{t("inspector.groupAddressStyle")}</dt>
        <dd className="mono">{props.tree.group_address_style}</dd>
      </dl>
    </div>
  );
}

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
  propertiesOnly?: boolean;
  selection: Selection;
  tree: ProjectTree;
  deviceDetail: DeviceDetail | null;
  onApplied: (tree: ProjectTree) => void;
  onDeleted: (tree: ProjectTree) => void;
}) {
  const { selection, tree, deviceDetail, onApplied, onDeleted } = props;

  if (selection.kind === "project") {
    return <ProjectInspector tree={tree} />;
  }

  if (selection.kind === "device") {
    if (!deviceDetail) return null;
    const canDelete = findDeviceLineInFirstInstallation(tree, deviceDetail.id) !== undefined;
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
