/** The one ctrl/shift-click multi-selection state machine, shared by tree and address table. */
import { useEffect, useMemo, useState } from "react";
import type React from "react";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { BuildingNode } from "./bindings/BuildingNode";
import type { DeviceNode } from "./bindings/DeviceNode";
import type { MultiSelection, MultiSelectionKind, Selection } from "./selection";

// Render order of every device/group-address id across the tree, matching
// `ProjectExplorer`'s `InstallationItem` JSX order (topology, then
// buildings, then unassigned; group addresses are already a flat
// per-installation list) — the anchor/target pair a shift-click range is
// computed over. A device reachable from both topology and a building
// keeps only its first occurrence (a `Set` can't select "the same id
// twice" anyway).
//
// `GroupAddressTable` renders the same flat group-address list in the same
// order, so one order serves both views. A table filtered down to a few
// rows is the one case where the two diverge: see `onItemClick`'s
// `visibleOrder` argument.
export function deviceRenderOrder(tree: ProjectTree): number[] {
  const seen = new Set<number>();
  const order: number[] = [];
  const addAll = (devices: DeviceNode[]) => {
    for (const d of devices) {
      if (!seen.has(d.id)) {
        seen.add(d.id);
        order.push(d.id);
      }
    }
  };
  function addBuildings(nodes: BuildingNode[]) {
    for (const node of nodes) {
      addAll(node.devices);
      addBuildings(node.children);
    }
  }
  for (const inst of tree.installations) {
    for (const area of inst.topology) {
      for (const line of area.lines) addAll(line.devices);
    }
    addBuildings(inst.buildings);
    addAll(inst.unassigned);
  }
  return order;
}

export function groupAddressRenderOrder(tree: ProjectTree): number[] {
  return tree.installations.flatMap((inst) => inst.group_addresses.map((ga) => ga.id));
}

export type ItemClickHandler = (
  e: Pick<React.MouseEvent, "shiftKey" | "ctrlKey" | "metaKey" | "preventDefault">,
  kind: MultiSelectionKind,
  id: number,
  sel: Selection,
  // The ids the clicked view currently shows, in the order it shows them,
  // when that differs from the whole tree's order — a filtered or
  // range-scoped address table. A shift-click then spans what the user can
  // see, never silently selecting rows hidden by the filter. Omitted by
  // the tree, which always renders every id.
  visibleOrder?: number[],
  preserveMultiSelection?: boolean,
) => void;

export interface MultiSelectionState {
  multiSelection: MultiSelection | null;
  onItemClick: ItemClickHandler;
  clear: () => void;
}

// Ctrl/shift-click multi-select (T9, GAP_ANALYSIS_ETS.md B9). Lifted out
// of `ProjectExplorer` in stage 4 so the group-address table can feed the
// same `BulkActionToolbar` commands: two hook instances would be two
// states agreeing only by luck, and two toolbars could disagree about
// what is selected. `App` holds exactly one instance and passes it to
// both views.
//
// `tree` is nullable because `App`'s is: with no project open there is
// nothing to select and every order is empty.
export function useMultiSelection(
  tree: ProjectTree | null,
  onSelect: (sel: Selection) => void,
): MultiSelectionState {
  const [multiSelection, setMultiSelection] = useState<MultiSelection | null>(null);
  // The anchor a shift-click range is computed from — updated on every
  // click (plain, ctrl, or shift) so a shift-click after a plain click
  // extends from that plain selection too, the usual file-explorer rule.
  const [lastClicked, setLastClicked] = useState<{ kind: MultiSelectionKind; id: number } | null>(
    null,
  );

  const deviceOrder = useMemo(() => (tree ? deviceRenderOrder(tree) : []), [tree]);
  const gaOrder = useMemo(() => (tree ? groupAddressRenderOrder(tree) : []), [tree]);

  // New authoritative snapshots invalidate deleted targets, but view changes
  // do not: the device list and editor share one preserved selection.
  useEffect(() => {
    setMultiSelection((previous) => {
      if (!previous) return previous;
      const present = new Set(previous.kind === "device" ? deviceOrder : gaOrder);
      const ids = new Set([...previous.ids].filter((id) => present.has(id)));
      return ids.size === previous.ids.size ? previous : ids.size ? { kind: previous.kind, ids } : null;
    });
  }, [tree, deviceOrder, gaOrder]);

  useEffect(() => {
    if (!multiSelection) return;
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") setMultiSelection(null);
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [multiSelection]);

  const onItemClick: ItemClickHandler = (e, kind, id, sel, visibleOrder, preserveMultiSelection = false) => {
    const order = visibleOrder ?? (kind === "device" ? deviceOrder : gaOrder);
    if (e.shiftKey) {
      e.preventDefault();
      const anchorId = lastClicked && lastClicked.kind === kind ? lastClicked.id : null;
      const from = anchorId !== null ? order.indexOf(anchorId) : -1;
      const to = order.indexOf(id);
      if (from !== -1 && to !== -1) {
        const [lo, hi] = from <= to ? [from, to] : [to, from];
        setMultiSelection({ kind, ids: new Set(order.slice(lo, hi + 1)) });
      } else {
        setMultiSelection({ kind, ids: new Set([id]) });
      }
      setLastClicked({ kind, id });
      return;
    }
    if (e.ctrlKey || e.metaKey) {
      e.preventDefault();
      setMultiSelection((prev) => {
        if (!prev || prev.kind !== kind) return { kind, ids: new Set([id]) };
        const ids = new Set(prev.ids);
        if (ids.has(id)) ids.delete(id);
        else ids.add(id);
        return { kind, ids };
      });
      setLastClicked({ kind, id });
      return;
    }
    // Plain click — untouched contract: clears any multi-selection, sets
    // the single `Selection` exactly as before this feature existed.
    if (!preserveMultiSelection) setMultiSelection(null);
    setLastClicked({ kind, id });
    onSelect(sel);
  };

  return { multiSelection, onItemClick, clear: () => setMultiSelection(null) };
}
