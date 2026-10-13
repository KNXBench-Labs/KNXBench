/** Native hierarchical address table over the shared presentation index. */
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { GroupAddressWorkspaceViewProps } from "./groupAddressWorkspaceView";
import { useTranslate } from "./i18n";
import { useUiLanguage } from "./uiLanguage";
import { useGroupAddressFormat } from "./gaNotation";
import { dptText, hasDptConflict, linkDirectionCounts, rangePath } from "./groupAddressView";
import { groupAddressTreeEntries, limitGroupAddressEntries, type GroupAddressTreeEntry } from "./groupAddressPresentation";
import HelpTip from "./HelpTip";
import GroupAddressWindowNotice from "./GroupAddressWindowNotice";
const CHECKBOX_CLICK = { shiftKey: false, ctrlKey: true, metaKey: false, preventDefault: () => {} };
/** One step per tree level, capped so imported deep trees cannot push the table off screen. */
const treeIndent = (depth: number) => `${Math.min(depth, 12) * 1.2}rem`;

export default function GroupAddressTreeTable(props: GroupAddressWorkspaceViewProps & {
  /** Open groups (issue #11: the tree remembers what is open, not what is closed). */
  expanded: ReadonlySet<string>; onToggle: (key: string) => void; includeEmpty: boolean; flat?: boolean;
}) {
  const { installation, index, rows, selection, multiSelection, onItemClick, expanded, onToggle } = props;
  const t = useTranslate(); const [language] = useUiLanguage(); const formatGa = useGroupAddressFormat();
  const allEntries: GroupAddressTreeEntry[] = props.flat
    ? rows.map(row => ({ kind: "address", key: row.key, depth: 0, row }))
    : groupAddressTreeEntries(index, rows, expanded, props.includeEmpty);
  const totalAddresses = allEntries.filter(entry => entry.kind === "address").length;
  const entries = limitGroupAddressEntries(allEntries, props.limit);
  const shownAddresses = Math.min(totalAddresses, props.limit);
  const visibleOrder = entries.flatMap(entry => entry.kind === "address" && entry.row.selectable ? [entry.row.node.id] : []);
  const checked = (ga: GroupAddressNode) => multiSelection?.kind === "group_address" && multiSelection.ids.has(ga.id);
  return (
      <div className="workspace-table-wrap">
        <table className="workspace-table address-table">
          <thead>
            <tr>
              <th className="address-table-check">
                <span className="sr-only">{t("addressTable.selectColumn")}</span>
              </th>
              <th>{t("workbench.address")}</th>
              <th>{t("workbench.name")}</th>
              <th>{t("addressTable.range")} <HelpTip labelKey="help.tip.groupRange.label" textKey="help.tip.groupRange.text" topicId="groupRanges" /></th>
              <th>{t("addressTable.dpt")}</th>
              <th>{t("addressTable.links")}</th>
            </tr>
          </thead>
          <tbody>
            {entries.map((entry) => {
              if (entry.kind === "group") {
                const group = entry.group;
                const label = group.range ? (group.range.name || t("gaWorkspace.unnamedRange", { id: group.range.id }))
                  : t(group.kind === "unassigned" ? "addressTable.noRange" : "gaWorkspace.unresolved");
                return <tr key={entry.key} className="ga-range-row"><td colSpan={6}>
                  <button type="button" data-ga-expand={entry.key} data-ga-nav=""
                    style={{ paddingInlineStart: treeIndent(entry.depth) }}
                    aria-expanded={expanded.has(entry.key)} onClick={() => onToggle(entry.key)}>{expanded.has(entry.key) ? "▾" : "▸"} {label} <small>({entry.count})</small></button>
                </td></tr>;
              }
              const row = entry.row;
              const ga = row.node;
              const counts = linkDirectionCounts(ga);
              const path = rangePath(installation.group_ranges, ga.range);
              return (
                <tr
                  key={row.key}
                  data-rename-kind={row.selectable ? "group_address" : undefined} data-rename-id={row.selectable ? ga.id : undefined}
                  data-crt-surface="row"
                  aria-selected={selection?.kind === "group_address" && selection.id === ga.id}
                  className={checked(ga) ? "row-multi-selected" : undefined}
                >
                  <td className={props.flat ? "address-table-check" : "address-table-check address-table-check--tree"}>
                    {/* Issue #9: the checkbox leads the row, so it carries the row's tree level. */}
                    <input
                      type="checkbox"
                      style={props.flat ? undefined : { marginInlineStart: treeIndent(entry.depth) }}
                      disabled={!row.selectable}
                      checked={checked(ga)}
                      aria-label={`${formatGa(ga.address)} ${ga.name}`}
                      onChange={() =>
                        onItemClick(
                          CHECKBOX_CLICK,
                          "group_address",
                          ga.id,
                          { kind: "group_address", id: ga.id },
                          visibleOrder,
                        )
                      }
                    />
                  </td>
                  <td className="mono ga-address">
                    <button
                      className="table-select"
                      data-ga-select={ga.id} data-ga-nav="" disabled={!row.selectable}
                      data-crt-activate=""
                      onClick={(e) =>
                        onItemClick(
                          e,
                          "group_address",
                          ga.id,
                          { kind: "group_address", id: ga.id },
                          visibleOrder,
                        )
                      }
                    >
                      {formatGa(ga.address)}
                    </button>
                  </td>
                  <td>{ga.name}</td>
                  <td>{path ?? t("addressTable.noRange")}</td>
                  <td className={hasDptConflict(ga) ? "mono dpt-conflict" : "mono"}>
                    {dptText(t, ga, language)}
                    {hasDptConflict(ga) && (
                      <small className="dpt-conflict-note"> {t("addressTable.dptConflict")}</small>
                    )}
                  </td>
                  <td>
                    {counts.total === 0
                      ? t("addressTable.noLinks")
                      : t("addressTable.linkCounts", {
                          senders: counts.senders,
                          receivers: counts.receivers,
                        })}
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
        {rows.length === 0 && (
          <p role="status">
            {installation.group_addresses.length === 0
              ? t("workbench.emptyStructure")
              : t("addressTable.noMatches")}
          </p>
        )}
        <GroupAddressWindowNotice shown={shownAddresses} total={totalAddresses} onLimit={props.onLimit} />
      </div>
  );
}
