/** Shared group-address workspace controller; renderers never own project edits. */
import { useEffect, useMemo, useState } from "react";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { MultiSelection, Selection } from "./selection";
import type { ItemClickHandler } from "./multiSelection";
import { useTranslate } from "./i18n";
import { useUiLanguage } from "./uiLanguage";
import { dptDisplay } from "./dptLabels";
import { buildGroupAddressPresentation, selectGroupAddressRows, type GroupAddressPresentationFilter } from "./groupAddressPresentation";
import { GROUP_ADDRESS_VIEWS, useGroupAddressView, type GroupAddressPresentationView } from "./groupAddressViewPreference";
import { groupAddressGroupLabel, type GroupAddressWorkspaceViewProps } from "./groupAddressWorkspaceView";
import GroupAddressTreeTable from "./GroupAddressTreeTable";
import GroupAddressColumns from "./GroupAddressColumns";
import GroupAddressAtlas from "./GroupAddressAtlas";
import GroupAddressMatrix from "./GroupAddressMatrix";
import { buildGroupAddressMatrix } from "./groupAddressMatrix";
import { useGroupAddressObjectInventory } from "./groupAddressObjectInventory";
import GroupAddressLinks from "./GroupAddressLinks";
import NewGroupAddressInRangeDialog from "./NewGroupAddressInRangeDialog";
import type { GroupRangeNode } from "./bindings/GroupRangeNode";
import HelpTip from "./HelpTip";
import { GROUP_ADDRESS_WINDOW } from "./GroupAddressWindowNotice";
import {
  defaultOpenGroups, effectiveOpenGroups, groupWithAncestors, groupsRevealing, groupsWithMatches, pruneGroups, readOpenGroups, writeOpenGroups,
} from "./groupAddressTreeState";
import "./groupAddressWorkspace.css";

/** Local view memory; absent (private mode, tests without storage) means this session only. */
function viewStorage(): Storage | null {
  try { return typeof window === "undefined" ? null : window.localStorage; } catch { return null; }
}

export default function GroupAddressTable(props: {
  installation: InstallationNode; selection: Selection | null; multiSelection: MultiSelection | null;
  /** The whole project: adding members follows the core's cross-installation link rule. */
  tree: ProjectTree;
  onItemClick: ItemClickHandler; onTreeUpdate: (tree: ProjectTree) => void;
  rangeScope: number | null;
  projectContextKey?: string;
  showColumnReset?: boolean;
}) {
  const { installation, selection, multiSelection, onItemClick, onTreeUpdate, rangeScope } = props;
  const t = useTranslate(); const [language] = useUiLanguage();
  const [view, setView] = useGroupAddressView();
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<GroupAddressPresentationFilter>("all");
  const [scope, setScope] = useState<string | null>(null);
  const index = useMemo(() => buildGroupAddressPresentation(installation), [installation]);
  // Issue #11: the tree remembers which groups are *open*, per project on this machine.
  const viewKey = props.tree.view_key ?? null;
  const initialOpen = () => {
    const storage = viewStorage();
    const stored = viewKey === null || storage === null ? null : readOpenGroups(storage, viewKey, installation.id);
    return stored === null ? defaultOpenGroups() : pruneGroups(stored, index);
  };
  // The open set belongs to one project and installation; a reply for the next
  // one must never store the previous view under its key.
  const owner = `${props.projectContextKey ?? ""}:${installation.id}`;
  const [openState, setOpenState] = useState(() => ({ owner, keys: initialOpen() }));
  const open = openState.owner === owner ? openState.keys : initialOpen();
  const setOpen = (update: (previous: Set<string>) => Set<string>) => setOpenState(previous =>
    ({ owner, keys: update(previous.owner === owner ? previous.keys : initialOpen()) }));
  // Groups the user closed while a search or filter forced them open; reset with the search.
  const [closedDuringSearch, setClosedDuringSearch] = useState<Set<string>>(() => new Set());
  const inventory = useGroupAddressObjectInventory(installation, `${props.projectContextKey ?? ""}:${installation.id}`);
  const matrix = useMemo(() => buildGroupAddressMatrix(index, inventory.current), [index, inventory.current]);
  const [columnKeys, setColumnKeys] = useState<Set<string> | null>(null);
  const [columnQuery, setColumnQuery] = useState("");
  const [limit, setLimit] = useState(GROUP_ADDRESS_WINDOW);
  // Issue #20: the range a `+` asked to create an address in.
  const [createIn, setCreateIn] = useState<{ range: GroupRangeNode; label: string; groupKey: string } | null>(null);
  useEffect(() => {
    setQuery(""); setFilter("all"); setScope(null); setClosedDuringSearch(new Set()); setCreateIn(null);
    setOpenState(previous => previous.owner === owner ? previous : { owner, keys: initialOpen() });
    setColumnKeys(null); setColumnQuery("");
  }, [props.projectContextKey, installation.id]);
  useEffect(() => {
    const present = new Set(matrix.columns.map(column => column.key));
    // A device being re-read keeps its chosen columns until its reply settles.
    for (const key of inventory.pendingKeys) present.add(key);
    setColumnKeys(previous => {
      if (previous === null) return previous;
      const next = new Set([...previous].filter(key => present.has(key)));
      return next.size === previous.size ? previous : next;
    });
  }, [matrix, inventory.pendingKeys]);
  const validScope = scope !== null && index.groups.has(scope) ? scope : null;
  useEffect(() => {
    if (scope !== validScope) setScope(validScope);
    setOpen(previous => pruneGroups(previous, index));
  }, [index, scope, validScope]);
  useEffect(() => { setScope(null); }, [rangeScope]);
  // A new result set starts from a bounded window again; selection is unaffected.
  useEffect(() => { setLimit(GROUP_ADDRESS_WINDOW); }, [query, filter, scope, rangeScope, view]);
  const externalGroups = [...index.groups.values()].filter(group => group.range?.id === rangeScope);
  const externalScope = rangeScope !== null && externalGroups.length === 1 ? externalGroups[0].key : null;
  const baseResult = selectGroupAddressRows(index, { query, filter, scope: externalScope }, dpt => dptDisplay(dpt, language));
  const baseRows = baseResult.rows;
  const allowed = new Set(baseRows.map(row => row.key));
  const rows = selectGroupAddressRows(index, { query, filter, scope: validScope }, dpt => dptDisplay(dpt, language)).rows.filter(row => allowed.has(row.key));
  const selectedRows = index.rows.filter(row => selection?.kind === "group_address" && row.node.id === selection.id);
  const selected = selectedRows.length === 1 ? selectedRows[0].node : null;
  const shared: GroupAddressWorkspaceViewProps = { installation, index, rows, baseRows, scope: validScope,
    onScope: setScope, selection, multiSelection, onItemClick, limit, onLimit: setLimit, showColumnReset: props.showColumnReset };
  const searching = query.trim() !== "" || filter !== "all";
  useEffect(() => { setClosedDuringSearch(new Set()); }, [query, filter]);
  const matches = searching ? groupsWithMatches(index, new Set(rows.map(row => row.key))) : null;
  const expanded = effectiveOpenGroups(open, matches, closedDuringSearch);
  // Whatever selects an address (a link, the search, the explorer) opens the groups above it.
  const selectedId = selection?.kind === "group_address" ? selection.id : null;
  useEffect(() => {
    if (selectedId === null) return;
    const reveal = groupsRevealing(index, selectedId);
    if (reveal.size === 0) return;
    setOpen(previous => [...reveal].every(key => previous.has(key)) ? previous : new Set([...previous, ...reveal]));
    setClosedDuringSearch(previous => [...reveal].some(key => previous.has(key)) ? new Set([...previous].filter(key => !reveal.has(key))) : previous);
    // Only a new selection reveals; an edit that re-reads the tree must not reopen what the user closed.
  }, [selectedId, owner]);
  // Scoping to a range (explorer or breadcrumb) is navigation too: that range opens.
  const scopeKey = validScope ?? externalScope;
  useEffect(() => {
    if (scopeKey === null) return;
    const reveal = groupWithAncestors(index, scopeKey);
    setOpen(previous => reveal.every(key => previous.has(key)) ? previous : new Set([...previous, ...reveal]));
  }, [scopeKey, owner]);
  // Stores what is open (not search-forced groups). Save As changes the key and
  // takes the current view along; an unsaved new project stores nothing.
  useEffect(() => {
    const storage = viewStorage();
    if (viewKey === null || storage === null || openState.owner !== owner) return;
    writeOpenGroups(storage, viewKey, installation.id, openState.keys);
  }, [openState, owner, viewKey, installation.id]);
  const toggle = (key: string) => {
    if (expanded.has(key)) {
      setOpen(previous => { const next = new Set(previous); next.delete(key); return next; });
      if (matches?.has(key)) setClosedDuringSearch(previous => new Set([...previous, key]));
    } else {
      setOpen(previous => new Set([...previous, key]));
      setClosedDuringSearch(previous => { const next = new Set(previous); next.delete(key); return next; });
    }
  };
  const expandAll = () => { setOpen(() => new Set(index.groups.keys())); setClosedDuringSearch(new Set()); };
  const collapseAll = () => { setOpen(() => new Set()); setClosedDuringSearch(new Set(index.groups.keys())); };
  return <div className="address-table-view ga-workspace" onKeyDown={e => {
    if (!(e.target instanceof HTMLButtonElement) || !e.target.hasAttribute("data-ga-nav")) return;
    if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(e.key)) return;
    e.preventDefault(); e.stopPropagation();
    const buttons = [...e.currentTarget.querySelectorAll<HTMLButtonElement>("button[data-ga-nav]:not(:disabled)")];
    const position = buttons.indexOf(e.target);
    const target = buttons[e.key === "Home" ? 0 : e.key === "End" ? buttons.length - 1 : Math.max(0, Math.min(buttons.length - 1, position + (e.key === "ArrowDown" ? 1 : -1)))];
    target?.focus();
    if (e.shiftKey && target?.hasAttribute("data-ga-select")) {
      const id = Number(target.dataset.gaSelect);
      const order = [...e.currentTarget.querySelectorAll<HTMLButtonElement>("button[data-ga-select]:not(:disabled)")].map(button => Number(button.dataset.gaSelect));
      onItemClick(e, "group_address", id, { kind: "group_address", id }, order);
    }
  }}>
    <div className="address-table-toolbar">
      <input type="search" className="address-filter" aria-label={t("addressTable.filterLabel")}
        placeholder={t("addressTable.filterPlaceholder")} value={query} onChange={e => setQuery(e.target.value)} />
      <label>{t("gaWorkspace.presentation")}<select data-ga-view value={view}
        onChange={e => setView(e.target.value as GroupAddressPresentationView)}>
        {GROUP_ADDRESS_VIEWS.map(choice => <option key={choice} value={choice}>{t(`gaWorkspace.view.${choice}`)}</option>)}
      </select></label>
      <label>{t("gaWorkspace.filter")}<select value={filter} onChange={e => setFilter(e.target.value as GroupAddressPresentationFilter)}>
        {(["all", "untyped", "conflict", "linked", "unlinked"] as const).map(choice => <option key={choice} value={choice}>{t(`gaWorkspace.filter.${choice}`)}</option>)}
      </select></label>
      {view === "tree" && <span className="ga-tree-actions">
        <button type="button" className="ga-tree-action" data-ga-expand-all="" onClick={expandAll}>{t("gaWorkspace.expandAll")}</button>
        <button type="button" className="ga-tree-action" data-ga-collapse-all="" onClick={collapseAll}>{t("gaWorkspace.collapseAll")}</button>
      </span>}
      <HelpTip labelKey="help.tip.addressTable.label" textKey="help.tip.addressTable.text" topicId="groupAddresses" />
    </div>
    <nav className="ga-breadcrumb" aria-label={t("addressTable.range")}>
      {validScope && <button type="button" onClick={() => setScope(null)}>{t("gaWorkspace.all")}</button>}
      <span data-ga-current-scope aria-current="location">{validScope ? groupAddressGroupLabel(t, index.groups.get(validScope)!) : t("gaWorkspace.all")}</span>
    </nav>
    {index.issues.length > 0 && <details className="ga-issues"><summary>{t("gaWorkspace.problems", { count: index.issues.length })}</summary>
      <ul>{index.issues.map((issue, i) => <li key={i}>{t(`gaWorkspace.issue.${issue.code}`)} · {issue.key} → #{issue.reference}</li>)}</ul>
    </details>}
    {rangeScope !== null && externalGroups.length !== 1 && <p role="status">{t("gaWorkspace.scopeUnavailable")}</p>}
    {view === "columns" ? <GroupAddressColumns {...shared} /> : view === "atlas" ? <GroupAddressAtlas {...shared} /> : view === "matrix" ? <GroupAddressMatrix {...shared}
      matrix={matrix} inventory={inventory} columnKeys={columnKeys} onColumns={setColumnKeys} columnQuery={columnQuery} onColumnQuery={setColumnQuery} /> : <GroupAddressTreeTable {...shared}
      expanded={expanded} onToggle={toggle} includeEmpty={query.trim() === "" && filter === "all" && validScope === null && externalScope === null}
      onCreateInRange={(range, label, groupKey) => setCreateIn({ range, label, groupKey })} />}
    {createIn && <NewGroupAddressInRangeDialog range={createIn.range} rangeLabel={createIn.label}
      usedAddresses={installation.group_addresses.map(ga => ga.address)} style={props.tree.group_address_style}
      onCreated={created => {
        // The new address shows where it was asked for: its range opens.
        const reveal = groupWithAncestors(index, createIn.groupKey);
        setOpen(previous => new Set([...previous, ...reveal]));
        onTreeUpdate(created);
      }}
      onClose={() => setCreateIn(null)} />}
    {selected && <GroupAddressLinks ga={selected} tree={props.tree} onTreeUpdate={onTreeUpdate} />}
  </div>;
}
