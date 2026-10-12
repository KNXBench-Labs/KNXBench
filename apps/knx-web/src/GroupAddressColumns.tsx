/** Stored-range columns with shared navigation, selection and address actions. */
import { useTranslate } from "./i18n";
import { groupAddressGroupLabel, type GroupAddressWorkspaceViewProps } from "./groupAddressWorkspaceView";
import GroupAddressTreeTable from "./GroupAddressTreeTable";

export default function GroupAddressColumns(props: GroupAddressWorkspaceViewProps) {
  const t = useTranslate();
  const path: string[] = [];
  let current = props.scope;
  while (current !== null) {
    path.unshift(current);
    current = props.index.groups.get(current)?.parent ?? null;
  }
  const levels = [props.index.roots, ...path.map(key => props.index.groups.get(key)!.children)];
  return <div className="ga-columns" data-ga-columns>
    <div className="ga-column-path">{levels.filter(level => level.length > 0).map((level, depth) =>
      <section className="ga-column" key={depth} aria-label={t("gaWorkspace.level", { level: depth + 1 })}>
        <h3>{depth === 0 ? t("addressTable.range") : groupAddressGroupLabel(t, props.index.groups.get(path[depth - 1])!)}</h3>
        <ul>{level.map(key => {
          const group = props.index.groups.get(key)!;
          return <li key={key}><button type="button" data-ga-scope={key} data-ga-nav=""
            aria-pressed={path.includes(key)} onClick={() => props.onScope(key)}>
            <strong>{groupAddressGroupLabel(t, group)}</strong><small>{group.addressCount}</small>
          </button></li>;
        })}</ul>
      </section>,
    )}</div>
    <GroupAddressTreeTable {...props} flat includeEmpty={false} collapsed={new Set()} onToggle={() => {}} />
  </div>;
}
