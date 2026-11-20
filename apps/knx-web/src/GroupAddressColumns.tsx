/** Stored-range columns with shared navigation, selection and address actions. */
import { useId } from "react";
import { useTranslate } from "./i18n";
import { groupAddressGroupLabel, type GroupAddressWorkspaceViewProps } from "./groupAddressWorkspaceView";
import GroupAddressTreeTable from "./GroupAddressTreeTable";

export default function GroupAddressColumns(props: GroupAddressWorkspaceViewProps) {
  const t = useTranslate();
  const noteId = useId();
  const path: string[] = [];
  let current = props.scope;
  while (current !== null) {
    path.unshift(current);
    current = props.index.groups.get(current)?.parent ?? null;
  }
  const levels = [props.index.roots, ...path.map(key => props.index.groups.get(key)!.children)];
  // A column entry differs when any address in its subtree differs.
  const groupDiffers = (key: string): boolean => {
    if (props.differingRowKeys === undefined) return false;
    const subtree = new Set<string>();
    const queue = [key];
    for (let i = 0; i < queue.length; i += 1) {
      const currentKey = queue[i];
      subtree.add(currentKey);
      for (const child of props.index.groups.get(currentKey)?.children ?? []) queue.push(child);
    }
    for (const row of props.index.rows) if (subtree.has(row.groupKey) && props.differingRowKeys!.has(row.key)) return true;
    return false;
  };
  return <div className="ga-columns" data-ga-columns>
    <div className="ga-column-path">{levels.filter(level => level.length > 0).map((level, depth) =>
      <section className="ga-column" key={depth} aria-label={t("gaWorkspace.level", { level: depth + 1 })}>
        <h3>{depth === 0 ? t("addressTable.range") : groupAddressGroupLabel(t, props.index.groups.get(path[depth - 1])!)}</h3>
        <ul>{level.map(key => {
          const group = props.index.groups.get(key)!;
          const differs = groupDiffers(key);
          return <li key={key}><button type="button" data-ga-scope={key} data-ga-nav=""
            aria-pressed={path.includes(key)} onClick={() => props.onScope(key)}
            aria-describedby={differs ? noteId : undefined} data-differs-from-download={differs ? "true" : undefined}>
            <strong>{groupAddressGroupLabel(t, group)}</strong><small>{group.addressCount}</small>
            {differs && <span className="ga-download-mark" title={t("gaWorkspace.lastDownload.group")} aria-hidden="true" />}
          </button></li>;
        })}</ul>
      </section>,
    )}</div>
    <GroupAddressTreeTable {...props} flat includeEmpty={false} expanded={new Set()} onToggle={() => {}} />
    <span id={noteId} hidden>{t("gaWorkspace.lastDownload.group")}</span>
  </div>;
}
