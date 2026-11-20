/** Semantic range drill-down with shared group-address selection and inspection. */
import { useTranslate } from "./i18n";
import { useUiLanguage } from "./uiLanguage";
import { useGroupAddressFormat } from "./gaNotation";
import { dptText, hasDptConflict } from "./groupAddressView";
import GroupAddressWindowNotice from "./GroupAddressWindowNotice";
import { groupAddressGroupLabel, type GroupAddressWorkspaceViewProps } from "./groupAddressWorkspaceView";
import { addressDiffers } from "./groupAddressLastDownload";
import { rawGroupAddress } from "./rawGroupAddress";

export default function GroupAddressAtlas(props: GroupAddressWorkspaceViewProps) {
  const t = useTranslate(); const [language] = useUiLanguage(); const formatGa = useGroupAddressFormat();
  const differing = props.differing ?? new Map();
  const style = props.addressStyle ?? "ThreeLevel";
  const rowDiffers = (row: { key: string; node: { address: string } }) =>
    props.differingRowKeys !== undefined ? props.differingRowKeys.has(row.key)
      : (rawGroupAddress(row.node.address, style) !== null && addressDiffers(differing, rawGroupAddress(row.node.address, style)!));
  const groups = props.scope === null ? props.index.roots : props.index.groups.get(props.scope)!.children;
  const matchingRows = props.scope === null ? [] : props.rows.filter(row => row.groupKey === props.scope);
  const directRows = Number.isFinite(props.limit) ? matchingRows.slice(0, props.limit) : matchingRows;
  const visibleOrder = directRows.filter(row => row.selectable).map(row => row.node.id);
  // A range heading differs when any address in its subtree differs.
  const rangeDiffers = (key: string): boolean => {
    if (props.differingRowKeys !== undefined) {
      const subtree = new Set<string>();
      const queue = [key];
      for (let i = 0; i < queue.length; i += 1) {
        const current = queue[i];
        subtree.add(current);
        for (const child of props.index.groups.get(current)?.children ?? []) queue.push(child);
      }
      for (const row of props.index.rows) if (subtree.has(row.groupKey) && props.differingRowKeys.has(row.key)) return true;
      return false;
    }
    for (const row of props.index.rows) {
      let current: string | null = row.groupKey;
      while (current !== null) {
        if (current === key) { if (rowDiffers(row)) return true; break; }
        current = props.index.groups.get(current)?.parent ?? null;
      }
    }
    return false;
  };
  return <section data-ga-atlas className="ga-atlas" aria-label={t("gaWorkspace.view.atlas")}>
    <p className="ga-evidence-note">{t("gaWorkspace.atlasMeaning")}</p>
    <ul className="ga-atlas-ranges">{groups.map(key => {
      const group = props.index.groups.get(key)!;
      const differs = rangeDiffers(key);
      return <li key={key}><button type="button" data-ga-scope={key} data-ga-nav="" onClick={() => props.onScope(key)}
        aria-describedby={differs ? "ga-atlas-download-note" : undefined}
        data-differs-from-download={differs ? "true" : undefined}>
        <strong>{groupAddressGroupLabel(t, group)}</strong>
        {group.range && <span className="mono">{group.range.start} — {group.range.end}</span>}
        <small>{t("gaWorkspace.storedCount", { count: group.addressCount })}</small>
        {differs && <span className="ga-download-mark" title={t("gaWorkspace.lastDownload.group")} aria-hidden="true" />}
      </button></li>;
    })}</ul>
    <ul className="ga-atlas-addresses">{directRows.map(row => {
      const ga = row.node;
      const checked = props.multiSelection?.kind === "group_address" && props.multiSelection.ids.has(ga.id);
      const selected = props.selection?.kind === "group_address" && props.selection.id === ga.id;
      const differs = rowDiffers(row);
      return <li key={row.key} className={checked || selected ? "ga-atlas-selected" : undefined}
        data-rename-kind={row.selectable ? "group_address" : undefined} data-rename-id={row.selectable ? ga.id : undefined}
        data-differs-from-download={differs ? "true" : undefined}>
        <label className="ga-card-check"><input type="checkbox" disabled={!row.selectable} checked={!!checked}
          aria-label={`${formatGa(ga.address)} ${ga.name}`} onChange={() => props.onItemClick(
            { shiftKey: false, ctrlKey: true, metaKey: false, preventDefault: () => {} },
            "group_address", ga.id, { kind: "group_address", id: ga.id }, visibleOrder,
          )} /></label>
        <button type="button" data-ga-select={ga.id} data-ga-nav="" data-crt-activate="" disabled={!row.selectable}
          aria-pressed={selected} aria-describedby={differs ? "ga-atlas-download-row-note" : undefined}
          onClick={e => props.onItemClick(e, "group_address", ga.id, { kind: "group_address", id: ga.id }, visibleOrder)}>
          <span className="mono">{formatGa(ga.address)}{differs && <span className="ga-download-mark" title={t("gaWorkspace.lastDownload.row")} aria-hidden="true" />}</span><strong>{ga.name}</strong>
          <small className={hasDptConflict(ga) ? "dpt-conflict" : undefined}>{dptText(t, ga, language)}</small>
          <small>{t("gaWorkspace.projectLinks", { count: ga.links.length })}</small>
        </button>
      </li>;
    })}</ul>
    <GroupAddressWindowNotice shown={directRows.length} total={matchingRows.length} onLimit={props.onLimit} />
    {props.scope !== null && directRows.length === 0 && <p role="status">{t("gaWorkspace.noDirectRows")}</p>}
    <span id="ga-atlas-download-note" hidden>{t("gaWorkspace.lastDownload.group")}</span>
    <span id="ga-atlas-download-row-note" hidden>{t("gaWorkspace.lastDownload.row")}</span>
  </section>;
}
