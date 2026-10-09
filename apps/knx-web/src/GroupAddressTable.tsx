/** Group-address table with range context, resolved DPT and linked com objects per direction. */
import { useMemo, useState } from "react";
import * as api from "./api";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { GroupAddressLinkNode } from "./bindings/GroupAddressLinkNode";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { MultiSelection, Selection } from "./selection";
import type { ItemClickHandler } from "./multiSelection";
import {
  directionLabel,
  dptText,
  hasDptConflict,
  linkDirectionCounts,
  rangePath,
  rangeWithDescendants,
} from "./groupAddressView";
import { groupAddressMatches, useGroupAddressFormat } from "./gaNotation";
import { useTranslate } from "./i18n";
import HelpTip from "./HelpTip";
import DeviceLink from "./DeviceLink";

// The real address table the workbench's group-address view was missing:
// the tree branch already carried ranges, DPTs and link directions, while
// the workspace showed a two-column address/name list (stage 4 brief,
// item 1). Range context, the resolved DPT and the linked communication
// objects all come straight from `GroupAddressNode` — extended in
// `knx-projection` for this view rather than reconstructed here out of
// per-device detail requests, since which objects reference an address is
// a question about the project, not about the screen.
//
// Multi-select goes through the shared `useMultiSelection` handler, the
// same one `ProjectExplorer` uses, so the single `BulkActionToolbar` in
// `App` acts on whatever was last selected in either view. Nothing about
// the command set is restated here.

// A checkbox toggle is a ctrl-click in everything but name: it adds or
// removes exactly one id and leaves the rest of the selection alone. The
// shared handler takes the few `MouseEvent` fields it reads as a
// structural type precisely so this synthetic one needs no cast.
const CHECKBOX_CLICK = {
  shiftKey: false,
  ctrlKey: true,
  metaKey: false,
  preventDefault: () => {},
};

function LinkRow(props: {
  ga: GroupAddressNode;
  link: GroupAddressLinkNode;
  onTreeUpdate: (tree: ProjectTree) => void;
}) {
  const { ga, link, onTreeUpdate } = props;
  const formatGa = useGroupAddressFormat();
  const t = useTranslate();
  const [error, setError] = useState<string | null>(null);

  // `Command::UnlinkComObject` carries no `installations[0]`-only
  // restriction (see `Inspector.tsx`'s `GroupLinkRow`, which calls the
  // same command from the communication object's side), so this is offered
  // for every address. Linking from this side is not: choosing a
  // communication object needs a device/object picker that does not exist
  // yet, and the Inspector's existing `NewGroupLinkRow` remains the one
  // place a link is created.
  async function unlink() {
    setError(null);
    try {
      onTreeUpdate(await api.unlinkComObject(link.com_object_id, ga.id, link.direction));
    } catch (e) {
      setError(api.errorMessage(e));
    }
  }

  return (
    <tr>
      <td>
        <DeviceLink deviceId={link.device_id}>{link.device_name ?? t("addressTable.unknownDevice", { id: link.device_id })}</DeviceLink>
        {link.device_address && <small className="mono"> {link.device_address}</small>}
      </td>
      <td>
        {link.com_object_name ?? t("addressTable.unnamedObject")}
        <small className="mono"> #{link.com_object_number}</small>
      </td>
      <td>{directionLabel(t, link.direction)}</td>
      <td>
        <button
          onClick={unlink}
          aria-label={t("addressTable.unlinkFrom", {
            object: link.com_object_name ?? `#${link.com_object_number}`,
            address: formatGa(ga.address),
          })}
        >
          {t("inspector.unlink")}
        </button>
        {error && <span className="field-error">{error}</span>}
      </td>
    </tr>
  );
}

export default function GroupAddressTable(props: {
  installation: InstallationNode;
  selection: Selection | null;
  multiSelection: MultiSelection | null;
  onItemClick: ItemClickHandler;
  onTreeUpdate: (tree: ProjectTree) => void;
  // The range the view is scoped to, owned by `App` alongside
  // `buildingScope` — set when a range is selected anywhere, so the tree,
  // the breadcrumb and this table always agree on the scope.
  rangeScope: number | null;
}) {
  const { installation, selection, multiSelection, onItemClick, onTreeUpdate, rangeScope } = props;
  const t = useTranslate();
  const formatGa = useGroupAddressFormat();
  const [query, setQuery] = useState("");

  const scopedIds = useMemo(
    () => (rangeScope === null ? null : rangeWithDescendants(installation.group_ranges, rangeScope)),
    [installation.group_ranges, rangeScope],
  );

  const needle = query.trim().toLowerCase();
  const rows = installation.group_addresses.filter((ga) => {
    if (scopedIds && (ga.range === null || !scopedIds.has(ga.range))) return false;
    if (needle === "") return true;
    return (
      // Both notations, always: the filter must not depend on which one
      // is on screen (`gaNotation.ts`'s `groupAddressSpellings`).
      groupAddressMatches(ga.address, needle) ||
      ga.name.toLowerCase().includes(needle) ||
      ga.dpts.some((dpt) => dpt.toLowerCase().includes(needle))
    );
  });
  // A shift-click range spans the rows the user can actually see, never
  // the hidden ones a filter or a range scope just removed.
  const visibleOrder = rows.map((ga) => ga.id);

  const selected =
    selection?.kind === "group_address"
      ? (installation.group_addresses.find((ga) => ga.id === selection.id) ?? null)
      : null;

  function checked(ga: GroupAddressNode): boolean {
    return multiSelection?.kind === "group_address" && multiSelection.ids.has(ga.id);
  }

  return (
    <div className="address-table-view">
      <div className="address-table-toolbar">
        <input
          type="search"
          className="address-filter"
          aria-label={t("addressTable.filterLabel")}
          placeholder={t("addressTable.filterPlaceholder")}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <HelpTip labelKey="help.tip.addressTable.label" textKey="help.tip.addressTable.text" topicId="groupAddresses" />
      </div>
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
            {rows.map((ga) => {
              const counts = linkDirectionCounts(ga);
              const path = rangePath(installation.group_ranges, ga.range);
              return (
                <tr
                  key={ga.id}
                  data-rename-kind="group_address" data-rename-id={ga.id}
                  data-crt-surface="row"
                  aria-selected={selection?.kind === "group_address" && selection.id === ga.id}
                  className={checked(ga) ? "row-multi-selected" : undefined}
                >
                  <td className="address-table-check">
                    <input
                      type="checkbox"
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
                    {dptText(t, ga)}
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
      </div>
      {selected && (
        <section className="address-links-panel" aria-label={t("addressTable.links")}>
          <h2>{t("addressTable.linksFor", { address: formatGa(selected.address) })}</h2>
          {selected.links.length === 0 ? (
            <p role="status">{t("addressTable.noLinksYet")}</p>
          ) : (
            <table className="workspace-table">
              <thead>
                <tr>
                  <th>{t("addressTable.participant")}</th>
                  <th>{t("addressTable.function")}</th>
                  <th>{t("addressTable.direction")}</th>
                  <th>
                    <span className="sr-only">{t("inspector.unlink")}</span>
                  </th>
                </tr>
              </thead>
              <tbody>
                {selected.links.map((link) => (
                  <LinkRow
                    key={`${link.com_object_id}-${link.direction}`}
                    ga={selected}
                    link={link}
                    onTreeUpdate={onTreeUpdate}
                  />
                ))}
              </tbody>
            </table>
          )}
        </section>
      )}
    </div>
  );
}
