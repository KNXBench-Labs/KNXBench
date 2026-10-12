/** Semantic range drill-down with shared group-address selection and inspection. */
import { useTranslate } from "./i18n";
import { useUiLanguage } from "./uiLanguage";
import { useGroupAddressFormat } from "./gaNotation";
import { dptText, hasDptConflict } from "./groupAddressView";
import GroupAddressWindowNotice from "./GroupAddressWindowNotice";
import { groupAddressGroupLabel, type GroupAddressWorkspaceViewProps } from "./groupAddressWorkspaceView";

export default function GroupAddressAtlas(props: GroupAddressWorkspaceViewProps) {
  const t = useTranslate(); const [language] = useUiLanguage(); const formatGa = useGroupAddressFormat();
  const groups = props.scope === null ? props.index.roots : props.index.groups.get(props.scope)!.children;
  const matchingRows = props.scope === null ? [] : props.rows.filter(row => row.groupKey === props.scope);
  const directRows = Number.isFinite(props.limit) ? matchingRows.slice(0, props.limit) : matchingRows;
  const visibleOrder = directRows.filter(row => row.selectable).map(row => row.node.id);
  return <section data-ga-atlas className="ga-atlas" aria-label={t("gaWorkspace.view.atlas")}>
    <p className="ga-evidence-note">{t("gaWorkspace.atlasMeaning")}</p>
    <ul className="ga-atlas-ranges">{groups.map(key => {
      const group = props.index.groups.get(key)!;
      return <li key={key}><button type="button" data-ga-scope={key} data-ga-nav="" onClick={() => props.onScope(key)}>
        <strong>{groupAddressGroupLabel(t, group)}</strong>
        {group.range && <span className="mono">{group.range.start} — {group.range.end}</span>}
        <small>{t("gaWorkspace.storedCount", { count: group.addressCount })}</small>
      </button></li>;
    })}</ul>
    <ul className="ga-atlas-addresses">{directRows.map(row => {
      const ga = row.node;
      const checked = props.multiSelection?.kind === "group_address" && props.multiSelection.ids.has(ga.id);
      const selected = props.selection?.kind === "group_address" && props.selection.id === ga.id;
      return <li key={row.key} className={checked || selected ? "ga-atlas-selected" : undefined}
        data-rename-kind={row.selectable ? "group_address" : undefined} data-rename-id={row.selectable ? ga.id : undefined}>
        <label className="ga-card-check"><input type="checkbox" disabled={!row.selectable} checked={!!checked}
          aria-label={`${formatGa(ga.address)} ${ga.name}`} onChange={() => props.onItemClick(
            { shiftKey: false, ctrlKey: true, metaKey: false, preventDefault: () => {} },
            "group_address", ga.id, { kind: "group_address", id: ga.id }, visibleOrder,
          )} /></label>
        <button type="button" data-ga-select={ga.id} data-ga-nav="" data-crt-activate="" disabled={!row.selectable}
          aria-pressed={selected} onClick={e => props.onItemClick(e, "group_address", ga.id, { kind: "group_address", id: ga.id }, visibleOrder)}>
          <span className="mono">{formatGa(ga.address)}</span><strong>{ga.name}</strong>
          <small className={hasDptConflict(ga) ? "dpt-conflict" : undefined}>{dptText(t, ga, language)}</small>
          <small>{t("gaWorkspace.projectLinks", { count: ga.links.length })}</small>
        </button>
      </li>;
    })}</ul>
    <GroupAddressWindowNotice shown={directRows.length} total={matchingRows.length} onLimit={props.onLimit} />
    {props.scope !== null && directRows.length === 0 && <p role="status">{t("gaWorkspace.noDirectRows")}</p>}
  </section>;
}
