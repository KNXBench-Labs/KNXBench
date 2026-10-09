/** Communication-object table with stable editor ownership across all view changes. */
import { Fragment, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import type { ComObjectNode } from "./bindings/ComObjectNode";
import type { ComObjectChannel } from "./bindings/ComObjectChannel";
import { useTranslate, type MessageKey, type Translate } from "./i18n";
import { useUiLanguage } from "./uiLanguage";
import { useProductLanguage } from "./productLanguage";
import { LanguageFallbackBadge, fellBack } from "./languageFallback";
import { formatGroupAddress } from "./gaNotation";
import { ACTIVATION_ORDER, DEFAULT_FILTERS, channelKey, deriveComObjectView, dptOptions, evaluatedChannel,
  effectiveDpt, hasFilters, type ComFilters, type ComSort, type ComSortColumn } from "./comObjectView";
import ComMutationBoundary from "./ComMutationBoundary";
const COLUMNS: { column: ComSortColumn; label: MessageKey }[] = [
  { column: "number", label: "comTable.number" }, { column: "name", label: "workbench.name" },
  { column: "function", label: "comTable.function" }, { column: "dpt", label: "inspector.dpt" },
  { column: "addresses", label: "comTable.addresses" }, { column: "status", label: "comTable.status" },
];
const STATUS_KEYS: Record<string, MessageKey> = { Active: "inspector.activation.active", Inactive: "inspector.activation.inactive",
  Undetermined: "inspector.activation.undetermined", NotEvaluated: "inspector.activation.notEvaluated" };
function channelLabel(c: ComObjectChannel | null, t: Translate): string {
  return c === null ? t("inspector.noEvaluatedChannel") : c.kind === "ChannelIndependentBlock"
    ? t("inspector.channelIndependent") : c.text || c.name || t("inspector.untitledChannel");
}
function ChannelText({ channel }: { channel: ComObjectChannel | null }) {
  const t = useTranslate();
  return <><strong>{channelLabel(channel, t)}</strong>
    {channel?.kind === "Channel" && channel.text && channel.name && <span className="com-object-channel-name">{t("inspector.channelName", { value: channel.name })}</span>}
    {channel?.kind === "Channel" && channel.number && <span className="com-object-channel-number">{t("inspector.channelNumber", { value: channel.number })}</span>}</>;
}
interface RowProps { com: ComObjectNode; flat: boolean; hidden: boolean; matches: boolean; open: boolean;
  descriptionMatch: boolean; editor: (com: ComObjectNode) => ReactNode; onOpen: (id: number, open: boolean) => void }
function ObjectRow({ com, flat, hidden, matches, open, descriptionMatch, editor, onOpen }: RowProps) {
  const t = useTranslate(); const [productLanguage] = useProductLanguage();
  const [visited, setVisited] = useState(false), [busy, setBusy] = useState(false);
  const details = useRef<HTMLDetailsElement>(null);
  const dpt = effectiveDpt(com);
  return <Fragment>
    <tr className="com-object-summary" data-object-id={com.id} data-channel-key={channelKey(com)} data-activation={com.activation} hidden={hidden}>
      <td className="mono"><details className="com-object-detail" ref={details} open={open} onToggle={event => {
        const next = event.currentTarget.open;
        if (!next && busy) { event.currentTarget.open = true; return; }
        if (next) setVisited(true);
        if (next !== open) onOpen(com.id, next);
      }}><summary aria-label={t("comTable.edit", { number: com.number })}>{com.number}</summary></details></td>
      <td><strong>{com.name ?? t("inspector.unnamed")}</strong>
        {descriptionMatch && <small className="com-description-match">{t("comTable.descriptionMatch", { value: com.description ?? "" })}</small>}
        {!matches && open && <small className="com-edit-exception">{t("comTable.editException")}</small>}</td>
      <td>{com.function_text || "—"}</td>
      <td className="com-object-effective-dpt"><span className="mono">{dpt ?? "—"}</span>
        {com.dpt === null && com.program_dpt !== null && <small className="com-object-dpt-origin">{t("inspector.programDefault")}</small>}
        {com.dpt_text && <small>{com.dpt_text}</small>}
        {productLanguage !== null && fellBack(com.dpt_text, com.dpt_text_language) && <LanguageFallbackBadge selected={productLanguage} source={null} />}</td>
      <td className="mono ga-address">{com.links.length ? com.links.map((link, i) => <span key={`${link.ga_id}-${link.direction}-${i}`} className="com-address-value">
        {link.address === null ? t("comTable.unresolvedLink", { id: link.ga_id }) : formatGroupAddress(link.address)}</span>) : "—"}</td>
      <td><span className="com-object-status" data-activation={com.activation}>{t(STATUS_KEYS[com.activation] ?? "inspector.activation.unknown")}</span>
        {com.activation === "Active" && !com.is_active && <small className="com-object-stored-status">{t("inspector.storedInactive")}</small>}
        {com.activation === "Inactive" && com.is_active && <small className="com-object-stored-status">{t("inspector.storedActive")}</small>}</td>
      {flat && <td><ChannelText channel={evaluatedChannel(com)} /></td>}
    </tr>
    <tr className="com-object-editor-row" data-editor-id={com.id} hidden={hidden || !open}><td colSpan={flat ? 7 : 6}>
      {/* Once opened, keep this subtree mounted even when closed. Row keys are
          direct siblings under one tbody in every view: keys cannot preserve
          state if a channel/flat switch reparents a component. */}
      {(visited || open) && <ComMutationBoundary onBusy={setBusy}>{editor(com)}</ComMutationBoundary>}
    </td></tr>
  </Fragment>;
}
export default function ComObjectTable({ objects, editor }: { objects: ComObjectNode[]; editor: (com: ComObjectNode) => ReactNode }) {
  const t = useTranslate(); const [language] = useUiLanguage();
  const [filters, setFilters] = useState<ComFilters>({ ...DEFAULT_FILTERS });
  const [sort, setSort] = useState<ComSort>({ column: null, descending: false });
  const [flat, setFlat] = useState(false), [openObjects, setOpenObjects] = useState(new Set<number>());
  const [expanded, setExpanded] = useState(new Set<string>());
  const [removed, setRemoved] = useState(0);
  // Filter-collapse overrides belong to the exact filter signature, not the
  // original expansion set. Clearing filters therefore restores it exactly.
  const signature = JSON.stringify(filters);
  const [filteredCollapse, setFilteredCollapse] = useState<{ signature: string; keys: Set<string> }>({ signature: "", keys: new Set() });
  const filtered = hasFilters(filters);
  const view = useMemo(() => deriveComObjectView(objects, filters, sort, language, !flat), [objects, filters, sort, language, flat]);
  const all = useMemo(() => deriveComObjectView(objects, DEFAULT_FILTERS, sort, language, !flat), [objects, sort, language, flat]);
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
  const orderedGroups = flat ? [{ key: "flat", channel: null, objects: all.objects, total: objects.length }] : all.groups;
  for (const group of orderedGroups) {
    const count = groupMatches.get(group.key) ?? 0;
    const hasEditor = group.objects.some(o => openObjects.has(o.id));
    const groupVisible = flat || !filtered || count > 0 || hasEditor;
    if (!flat) rows.push(<tr className="com-object-channel" key={`group:${group.key}`} data-channel-key={group.key} hidden={!groupVisible}>
      <th colSpan={6} scope="row"><button type="button" className="com-object-channel-summary" aria-expanded={isExpanded(group.key)} onClick={() => toggle(group.key)}>
        <span aria-hidden="true">{isExpanded(group.key) ? "▾" : "▸"}</span><ChannelText channel={group.channel} />
        <span className="com-object-channel-count">{t("comTable.count", { shown: count, total: group.total })}</span>
      </button></th></tr>);
    for (const com of group.objects) {
      const open = openObjects.has(com.id), matches = matchedIds.has(com.id);
      const visible = open || matches && (flat || groupVisible && isExpanded(channelKey(com)));
      rows.push(<ObjectRow key={`object:${com.id}`} com={com} flat={flat} hidden={!visible} matches={matches} open={open}
        descriptionMatch={view.descriptionMatches.has(com.id)} editor={editor} onOpen={onOpen} />);
    }
  }
  const columns = flat ? [...COLUMNS, { column: "channel" as const, label: "comTable.channel" as const }] : COLUMNS;
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
          : value.startsWith("family:") ? t("comTable.dptFamily", { number: value.slice(7) }) : value.slice(6)}</option>)}</select></label>
      <button type="button" onClick={() => { resetCollapse(); setFilters({ ...DEFAULT_FILTERS }); }}>{t("comTable.resetFilters")}</button>
      <button type="button" onClick={() => setSort({ column: null, descending: false })}>{t("comTable.originalOrder")}</button>
      <div className="com-table-views" role="group" aria-label={t("comTable.view")}>
        <button type="button" aria-pressed={!flat} onClick={() => setFlat(false)}>{t("comTable.grouped")}</button>
        <button type="button" aria-pressed={flat} onClick={() => setFlat(true)}>{t("comTable.flat")}</button>
      </div>
    </div>
    {removed > 0 && <p className="com-object-removal" role="status">{t("comTable.removed", { count: removed })}</p>}
    <p className="com-table-count" role="status">{t("comTable.count", { shown: view.objects.length, total: objects.length })}</p>
    <div className="workspace-table-wrap com-table-scroll" tabIndex={0} aria-label={t("comTable.table")}>
      <table className="workspace-table com-table"><thead><tr>{columns.map(c => <th scope="col" key={c.column} data-column={c.column}
        aria-sort={sort.column === c.column ? sort.descending ? "descending" : "ascending" : "none"}>
        <button type="button" onClick={() => setSort({ column: c.column, descending: sort.column === c.column && !sort.descending })}>{t(c.label)}
          {sort.column === c.column && <span aria-hidden="true">{sort.descending ? " ↓" : " ↑"}</span>}</button></th>)}</tr></thead><tbody>{rows}</tbody></table>
    </div>
    {view.objects.length === 0 && <p role="status">{t(objects.length === 0 ? "comTable.empty" : "comTable.noMatches")}</p>}
  </section>;
}
