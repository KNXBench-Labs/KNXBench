/** Communication-object table with stable editor ownership across all view changes. */
import ResizableTable from "./ResizableTable";
import { Fragment, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import type { ComObjectNode } from "./bindings/ComObjectNode";
import type { ComObjectChannel } from "./bindings/ComObjectChannel";
import type { ComObjectProjectFolder } from "./bindings/ComObjectProjectFolder";
import type { ProjectFolderCurrency } from "./bindings/ProjectFolderCurrency";
import { FolderCurrencyNotice } from "./FolderCurrency";
import type { TextFill } from "./bindings/TextFill";
import { useTranslate, type MessageKey, type Translate } from "./i18n";
import { useUiLanguage } from "./uiLanguage";
import { useProductLanguage } from "./productLanguage";
import { LanguageFallbackBadge, fellBack } from "./languageFallback";
import { formatGroupAddress } from "./gaNotation";
import { ACTIVATION_ORDER, DEFAULT_FILTERS, PRIORITY_ORDER, channelKey, deriveComObjectView, dptOptions, effectivePrioritySource, effectivePriorityToken, evaluatedChannel,
  effectiveDpt, hasFilters, projectFolderIds, projectFolderNodeLabel, projectFolderProductCaption, projectFolderRefusal, type ComFilters, type ComGrouping, type ComSort, type ComSortColumn,
  type RefusedProjectFolder } from "./comObjectView";
import ComMutationBoundary from "./ComMutationBoundary";
import { dptDisplay } from "./dptLabels";
import { effectivePriorityDescription } from "./EffectivePriority";
import { getSetting, setSetting, useSettingsRevision } from "./settingsStore";
/** Opt-in view preference (default off); stores no project data. */
export const PRIORITY_COLUMN_SETTING = "comTablePriorityColumn";
const COLUMNS: { column: ComSortColumn; label: MessageKey }[] = [
  { column: "number", label: "comTable.number" }, { column: "name", label: "workbench.name" },
  { column: "function", label: "comTable.function" }, { column: "dpt", label: "inspector.dpt" },
  { column: "addresses", label: "comTable.addresses" }, { column: "status", label: "comTable.status" },
];
const PRIORITY_COLUMN: { column: ComSortColumn; label: MessageKey } = { column: "priority", label: "comTable.priority" };
const PRIORITY_SOURCE_KEYS: Record<"userEdit" | "instance" | "product", MessageKey> = {
  userEdit: "comTable.prioritySource.userEdit", instance: "comTable.prioritySource.instance", product: "comTable.prioritySource.product" };
/** Effective value and a short winning-source tag; the full wording stays in the title. */
function PriorityCell({ com }: { com: ComObjectNode }) {
  const t = useTranslate();
  const token = effectivePriorityToken(com), source = effectivePrioritySource(com);
  return <td className="com-object-priority" data-priority={token ?? "unresolved"} data-priority-source={source ?? undefined}
    title={effectivePriorityDescription(t, (com as Partial<ComObjectNode>).effective_priority)}>
    {token === null ? <><span aria-hidden="true">—</span><span className="sr-only">{t("comTable.unresolvedPriority")}</span></> : <span>{token}</span>}
    {source !== null && <small className="com-object-priority-source">{t(PRIORITY_SOURCE_KEYS[source])}</small>}</td>;
}
const STATUS_KEYS: Record<string, MessageKey> = { Active: "inspector.activation.active", Inactive: "inspector.activation.inactive",
  Undetermined: "inspector.activation.undetermined", NotEvaluated: "inspector.activation.notEvaluated" };
function channelLabel(c: ComObjectChannel | null, t: Translate): string {
  return c === null ? t("inspector.noEvaluatedChannel") : c.kind === "ChannelIndependentBlock"
    ? t("inspector.channelIndependent") : c.text || c.name || t("inspector.untitledChannel");
}
const FILL_KEYS: Record<TextFill["source"], MessageKey> = {
  StoredValue: "comTable.textFill.StoredValue", ProductDefault: "comTable.textFill.ProductDefault",
  TemplateDefault: "comTable.textFill.TemplateDefault", EmptyValue: "comTable.textFill.EmptyValue",
  NoReference: "comTable.textFill.NoReference", UnknownParameter: "comTable.textFill.UnknownParameter",
  NotTextParameter: "comTable.textFill.NotTextParameter", Undecided: "comTable.textFill.Undecided" };
/** ADR-0126: says where a product text's `{{0}}` came from; nothing for any other text. */
function fillProps(fill: TextFill | undefined, t: Translate): { "data-text-fill"?: string; title?: string } {
  return fill === undefined ? {} : { "data-text-fill": fill.source,
    title: `${t(FILL_KEYS[fill.source])}\n${t("comTable.textFill.template", { value: fill.template })}` };
}
function ChannelText({ channel }: { channel: ComObjectChannel | null }) {
  const t = useTranslate();
  return <><strong {...fillProps(channel?.text ? channel.text_fill : undefined, t)}>{channelLabel(channel, t)}</strong>
    {channel?.kind === "Channel" && channel.text && channel.name && <span className="com-object-channel-name">{t("inspector.channelName", { value: channel.name })}</span>}
    {channel?.kind === "Channel" && channel.number && <span className="com-object-channel-number">{t("inspector.channelNumber", { value: channel.number })}</span>}</>;
}
/** Distinguishes each displayed literal declaration from the saved project membership. */
function FolderProductSources({ folder }: { folder: ComObjectProjectFolder }) {
  const t = useTranslate();
  return <>{folder.path.map((node, index) => {
    const caption = projectFolderProductCaption(node);
    return caption === null ? null : <small key={`${index}:${node.ref_id}`} className="provenance-badge"
      data-folder-caption-source={caption.source} title={t("comTable.productFolderTitle", {
        program: caption.application_program_id, block: caption.parameter_block_id, sha256: caption.source_sha256,
      })}>{t("comTable.productFolderSource", { id: node.ref_id })}</small>;
  })}</>;
}
/** Folder labels accompany IDs; the complete original saved ID path stays visible. */
function FolderText({ folder }: { folder: ComObjectProjectFolder | null }) {
  const t = useTranslate();
  if (folder === null) return <strong>{t("comTable.noFolderGroup")}</strong>;
  const path = folder.path.map(n => n.ref_id).join(" › ");
  return <><strong title={t("comTable.projectFolderTitle", { value: path })}>{folder.path.filter(n => n.kind === "Folder").map(projectFolderNodeLabel).join(" › ")}</strong>
    <FolderProductSources folder={folder} />
    <span className="com-object-folder-path">{t("comTable.folderPath", { value: path })}</span></>;
}
const FOLDER_REFUSAL_KEYS: Record<RefusedProjectFolder, MessageKey> = {
  Ambiguous: "comTable.projectFolderRefused.Ambiguous", UnidentifiedNode: "comTable.projectFolderRefused.UnidentifiedNode",
  TreeUnresolved: "comTable.projectFolderRefused.TreeUnresolved", ModuleObject: "comTable.projectFolderRefused.ModuleObject",
  Inactive: "comTable.projectFolderRefused.Inactive" };
/** Shows the selected literal folder labels with provenance, or the saved-placement refusal. */
function ProjectFolderNote({ com }: { com: ComObjectNode }) {
  const t = useTranslate();
  const ids = projectFolderIds(com), refusal = projectFolderRefusal(com);
  const folder = com.project_folder;
  const path = folder?.path.map(n => n.ref_id).join(" › ") ?? "";
  if (ids !== null) return <small className="com-object-project-folder" data-folder-status="Assigned"
    title={t("comTable.projectFolderTitle", { value: path })}>
    {t("comTable.projectFolder", { value: folder!.path.filter(n => n.kind === "Folder").map(projectFolderNodeLabel).join(" › ") })}
    <FolderProductSources folder={folder!} />
    {folder!.path.some(n => n.kind === "Folder" && (n.text !== undefined || projectFolderProductCaption(n) !== null)) && <><br /><span className="com-object-folder-path">{t("comTable.folderPath", { value: path })}</span></>}</small>;
  return refusal === null ? null
    : <small className="com-object-project-folder" data-folder-status={refusal}>{t(FOLDER_REFUSAL_KEYS[refusal])}</small>;
}
interface RowProps { com: ComObjectNode; flat: boolean; priority: boolean; span: number; groupKey: string | null; hidden: boolean; matches: boolean; open: boolean;
  descriptionMatch: boolean; editor: (com: ComObjectNode) => ReactNode; onOpen: (id: number, open: boolean) => void }
function ObjectRow({ com, flat, priority, span, groupKey, hidden, matches, open, descriptionMatch, editor, onOpen }: RowProps) {
  const t = useTranslate(); const [productLanguage] = useProductLanguage();
  const [language] = useUiLanguage();
  const [visited, setVisited] = useState(false), [busy, setBusy] = useState(false);
  const details = useRef<HTMLDetailsElement>(null);
  const dpt = effectiveDpt(com);
  return <Fragment>
    <tr className={flat ? "com-object-summary" : "com-object-summary com-object-summary--grouped"} data-object-id={com.id} data-channel-key={channelKey(com)} data-group-key={groupKey ?? undefined} data-activation={com.activation} hidden={hidden}>
      <td className="mono"><details className="com-object-detail" ref={details} open={open} onToggle={event => {
        const next = event.currentTarget.open;
        if (!next && busy) { event.currentTarget.open = true; return; }
        if (next) setVisited(true);
        if (next !== open) onOpen(com.id, next);
      }}><summary aria-label={t("comTable.edit", { number: com.number })}>{com.number}</summary></details></td>
      <td><strong className="com-object-name" {...fillProps(com.name === null ? undefined : com.name_fill, t)}>{com.name ?? t("inspector.unnamed")}</strong>
        <ProjectFolderNote com={com} />
        {descriptionMatch && <small className="com-description-match">{t("comTable.descriptionMatch", { value: com.description ?? "" })}</small>}
        {!matches && open && <small className="com-edit-exception">{t("comTable.editException")}</small>}</td>
      <td>{com.function_text || "—"}</td>
      <td className="com-object-effective-dpt" title={dpt ?? undefined}><span>{dptDisplay(dpt, language)}</span>
        {com.dpt === null && com.program_dpt !== null && <small className="com-object-dpt-origin">{t("inspector.programDefault")}</small>}
        {com.dpt_text && <small>{t("dpt.masterText", { value: com.dpt_text })}</small>}
        {com.size && <small className="com-object-size">{t("comTable.size", { value: com.size })}</small>}
        {productLanguage !== null && fellBack(com.dpt_text, com.dpt_text_language) && <LanguageFallbackBadge selected={productLanguage} source={null} />}</td>
      <td className="mono ga-address">{com.links.length ? com.links.map((link, i) => <span key={`${link.ga_id}-${link.direction}-${i}`} className="com-address-value">
        {link.address === null ? t("comTable.unresolvedLink", { id: link.ga_id }) : formatGroupAddress(link.address)}</span>) : "—"}</td>
      <td><span className="com-object-status" data-activation={com.activation}>{t(STATUS_KEYS[com.activation] ?? "inspector.activation.unknown")}</span>
        {com.activation === "Active" && !com.is_active && <small className="com-object-stored-status">{t("inspector.storedInactive")}</small>}
        {com.activation === "Inactive" && com.is_active && <small className="com-object-stored-status">{t("inspector.storedActive")}</small>}</td>
      {priority && <PriorityCell com={com} />}
      {flat && <td><ChannelText channel={evaluatedChannel(com)} /></td>}
    </tr>
    <tr className="com-object-editor-row" data-editor-id={com.id} hidden={hidden || !open}><td colSpan={span}>
      {/* Once opened, keep this subtree mounted even when closed. Row keys are
          direct siblings under one tbody in every view: keys cannot preserve
          state if a channel/folder/flat switch reparents a component. */}
      {(visited || open) && <ComMutationBoundary onBusy={setBusy}>{editor(com)}</ComMutationBoundary>}
    </td></tr>
  </Fragment>;
}
export default function ComObjectTable({ objects, editor, folderCurrency }: { objects: ComObjectNode[]; editor: (com: ComObjectNode) => ReactNode;
  /** Server-only: whether the saved folders still describe this device. */
  folderCurrency?: ProjectFolderCurrency }) {
  const t = useTranslate(); const [language] = useUiLanguage();
  const [filters, setFilters] = useState<ComFilters>({ ...DEFAULT_FILTERS });
  const [sort, setSort] = useState<ComSort>({ column: null, descending: false });
  // Channel and folder groups use disjoint key namespaces, so one expansion set serves both.
  const [mode, setMode] = useState<ComGrouping | "flat">("channel"), [openObjects, setOpenObjects] = useState(new Set<number>());
  const flat = mode === "flat", by: ComGrouping = mode === "folder" ? "folder" : "channel";
  useSettingsRevision();
  const showPriority = getSetting(PRIORITY_COLUMN_SETTING) === true;
  // However the column disappears (this toggle, another window, a settings
  // refresh), its filter and sort go with it, so nothing invisible narrows the table.
  useEffect(() => {
    if (showPriority) return;
    setFilters(previous => previous.priority === "all" ? previous : { ...previous, priority: "all" });
    setSort(previous => previous.column === "priority" ? { column: null, descending: false } : previous);
  }, [showPriority]);
  const columns = [...COLUMNS, ...(showPriority ? [PRIORITY_COLUMN] : []),
    ...(flat ? [{ column: "channel" as const, label: "comTable.channel" as const }] : [])];
  const [expanded, setExpanded] = useState(new Set<string>());
  const [removed, setRemoved] = useState(0);
  // Filter-collapse overrides belong to the exact filter signature, not the
  // original expansion set. Clearing filters therefore restores it exactly.
  const signature = JSON.stringify(filters);
  const [filteredCollapse, setFilteredCollapse] = useState<{ signature: string; keys: Set<string> }>({ signature: "", keys: new Set() });
  const filtered = hasFilters(filters);
  const view = useMemo(() => deriveComObjectView(objects, filters, sort, language, !flat, by), [objects, filters, sort, language, flat, by]);
  const all = useMemo(() => deriveComObjectView(objects, DEFAULT_FILTERS, sort, language, !flat, by), [objects, sort, language, flat, by]);
  const matchedIds = new Set(view.objects.map(o => o.id));
  const groupMatches = new Map(view.groups.map(g => [g.key, g.objects.length]));
  const options = dptOptions(objects);
  if (filters.dpt !== "all" && !options.includes(filters.dpt)) options.push(filters.dpt);
  useEffect(() => {
    const ids = new Set(objects.map(o => o.id));
    const missing = [...openObjects].filter(id => !ids.has(id));
    if (missing.length) {
      setRemoved(missing.length);
      setOpenObjects(new Set([...openObjects].filter(id => ids.has(id))));
    }
  }, [objects, openObjects]);
  const resetCollapse = () => setFilteredCollapse({ signature: "", keys: new Set() });
  const change = <K extends keyof ComFilters>(key: K, value: ComFilters[K]) => {
    resetCollapse(); setFilters(previous => ({ ...previous, [key]: value }));
  };
  const isExpanded = (key: string) => filtered ? !(filteredCollapse.signature === signature && filteredCollapse.keys.has(key)) : expanded.has(key);
  const toggle = (key: string) => {
    if (filtered) setFilteredCollapse(previous => {
      const keys = new Set(previous.signature === signature ? previous.keys : []);
      if (keys.has(key)) keys.delete(key); else keys.add(key);
      return { signature, keys };
    });
    else setExpanded(previous => { const keys = new Set(previous); if (keys.has(key)) keys.delete(key); else keys.add(key); return keys; });
  };
  const onOpen = (id: number, open: boolean) => setOpenObjects(previous => {
    const keys = new Set(previous); if (open) keys.add(id); else keys.delete(id); return keys;
  });
  const rows: ReactNode[] = [];
  const orderedGroups = flat ? [{ key: "flat", channel: null, folder: null, objects: all.objects, total: objects.length }] : all.groups;
  for (const group of orderedGroups) {
    const count = groupMatches.get(group.key) ?? 0;
    const hasEditor = group.objects.some(o => openObjects.has(o.id));
    const groupVisible = flat || !filtered || count > 0 || hasEditor;
    if (!flat) rows.push(<tr className="com-object-channel" key={`group:${group.key}`} data-group-key={group.key}
      data-channel-key={by === "channel" ? group.key : undefined} hidden={!groupVisible}>
      <th colSpan={columns.length} scope="row"><button type="button" className="com-object-channel-summary" aria-expanded={isExpanded(group.key)} onClick={() => toggle(group.key)}>
        <span aria-hidden="true">{isExpanded(group.key) ? "▾" : "▸"}</span>
        {by === "folder" ? <FolderText folder={group.folder} /> : <ChannelText channel={group.channel} />}
        <span aria-hidden="true" className="com-object-channel-separator">·</span><span className="com-object-channel-count">{t("comTable.count", { shown: count, total: group.total })}</span>
      </button></th></tr>);
    for (const com of group.objects) {
      const open = openObjects.has(com.id), matches = matchedIds.has(com.id);
      // Issue #16: a collapsed group hides every child, open editors included.
      // Inside an expanded group (and in the flat view) an open editor stays
      // visible even outside the filter. Hidden rows keep their editors mounted.
      const visible = (open || matches) && (flat || groupVisible && isExpanded(group.key));
      rows.push(<ObjectRow key={`object:${com.id}`} com={com} flat={flat} priority={showPriority} span={columns.length} groupKey={flat ? null : group.key} hidden={!visible} matches={matches} open={open}
        descriptionMatch={view.descriptionMatches.has(com.id)} editor={editor} onOpen={onOpen} />);
    }
  }
  return <section className="com-table-panel" aria-label={t("inspector.communicationObjects")}>
    <div className="com-table-toolbar">
      <input type="search" aria-label={t("comTable.search")} placeholder={t("comTable.search")} value={filters.query} onChange={e => change("query", e.target.value)} />
      <label>{t("comTable.status")}<select aria-label={t("comTable.statusFilter")} value={filters.status} onChange={e => change("status", e.target.value)}>
        <option value="all">{t("comTable.allStates")}</option>{ACTIVATION_ORDER.map(s => <option key={s} value={s}>{t(STATUS_KEYS[s])}</option>)}
        <option value="unknown">{t("inspector.activation.unknown")}</option></select></label>
      <label>{t("comTable.links")}<select aria-label={t("comTable.linkFilter")} value={filters.link} onChange={e => change("link", e.target.value as ComFilters["link"])}>
        <option value="all">{t("comTable.allLinks")}</option><option value="linked">{t("comTable.linked")}</option><option value="unlinked">{t("comTable.unlinked")}</option></select></label>
      <label>{t("inspector.dpt")}<select aria-label={t("comTable.dptFilter")} value={filters.dpt} onChange={e => change("dpt", e.target.value)}>
        <option value="all">{t("comTable.allDpts")}</option>{options.map(value => <option key={value} value={value}>{value === "missing" ? t("comTable.noDpt")
          : value.startsWith("family:") ? t("comTable.dptFamily", { number: value.slice(7) }) : dptDisplay(value.slice(6), language)}</option>)}</select></label>
      {showPriority && <label>{t("comTable.priority")}<select aria-label={t("comTable.priorityFilter")} value={filters.priority} onChange={e => change("priority", e.target.value)}>
        <option value="all">{t("comTable.allPriorities")}</option>{PRIORITY_ORDER.map(token => <option key={token} value={token}>{token}</option>)}
        <option value="unresolved">{t("comTable.unresolvedPriority")}</option></select></label>}
      <button type="button" onClick={() => { resetCollapse(); setFilters({ ...DEFAULT_FILTERS }); }}>{t("comTable.resetFilters")}</button>
      <button type="button" onClick={() => setSort({ column: null, descending: false })}>{t("comTable.originalOrder")}</button>
      <div className="com-table-views" role="group" aria-label={t("comTable.view")}>
        <button type="button" aria-pressed={mode === "channel"} onClick={() => setMode("channel")}>{t("comTable.grouped")}</button>
        <button type="button" aria-pressed={mode === "folder"} onClick={() => setMode("folder")}>{t("comTable.byFolder")}</button>
        <button type="button" aria-pressed={flat} onClick={() => setMode("flat")}>{t("comTable.flat")}</button>
      </div>
      <button type="button" className="com-table-priority-toggle" aria-pressed={showPriority}
        onClick={() => setSetting(PRIORITY_COLUMN_SETTING, !showPriority)}>{t("comTable.priorityColumn")}</button>
    </div>
    {removed > 0 && <p className="com-object-removal" role="status">{t("comTable.removed", { count: removed })}</p>}
    <p className="com-table-count" role="status">{t("comTable.count", { shown: view.objects.length, total: objects.length })}</p>
    {mode === "folder" && <p className="com-folder-view-note">{t("comTable.folderViewNote")}</p>}
    {(mode === "folder" || objects.some(o => o.project_folder?.status === "Assigned")) && <FolderCurrencyNotice currency={folderCurrency} />}
      <ResizableTable scrollClassName="workspace-table-wrap com-table-scroll" scrollLabel={t("comTable.table")} tableId="communication-objects"
        columns={columns.map(c => ({ id: c.column, width: c.column === "number" ? 120 : (flat ? 114 : 136), min: c.column === "number" ? 64 : 100 }))} className="workspace-table com-table"><thead><tr>{columns.map(c => <th scope="col" key={c.column} data-column={c.column}
        aria-sort={sort.column === c.column ? sort.descending ? "descending" : "ascending" : "none"}>
        <button type="button" onClick={() => setSort({ column: c.column, descending: sort.column === c.column && !sort.descending })}><span className="table-column-label">{t(c.label)}</span>
          <span className="table-sort-indicator" aria-hidden="true" style={{ visibility: sort.column === c.column ? "visible" : "hidden" }}>{sort.column === c.column ? sort.descending ? "↓" : "↑" : ""}</span></button></th>)}</tr></thead><tbody>{rows}</tbody></ResizableTable>
    {view.objects.length === 0 && <p role="status">{t(objects.length === 0 ? "comTable.empty" : "comTable.noMatches")}</p>}
  </section>;
}
