import { useState } from "react";
import * as api from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { MultiSelection } from "./selection";
import { flattenBuildingParts } from "./treeUtils";
import { useTranslate } from "./i18n";

// The bulk-action counterpart of `Inspector.tsx`'s single-entity Delete
// buttons/`LineMoveField`/`BuildingPartMoveField` — same "act immediately,
// no `confirm()`" behavior as those (see this task's brief: the premise
// that Inspector.tsx confirms before deleting is false; undo (Ctrl+Z) is
// the only safety net there, and this toolbar matches that). Rendered by
// `ProjectExplorer` above the tree whenever a `MultiSelection` is
// non-empty (T9, GAP_ANALYSIS_ETS.md B9).
export default function BulkActionToolbar(props: {
  multiSelection: MultiSelection;
  tree: ProjectTree;
  onTreeUpdate: (tree: ProjectTree) => void;
  onDone: () => void;
}) {
  const { multiSelection, tree, onTreeUpdate, onDone } = props;
  const t = useTranslate();
  const [error, setError] = useState<string | null>(null);
  const ids = Array.from(multiSelection.ids);
  const count = ids.length;

  async function run(action: Promise<ProjectTree>) {
    setError(null);
    try {
      const nextTree = await action;
      onTreeUpdate(nextTree);
      onDone();
    } catch (e) {
      setError(api.errorMessage(e));
    }
  }

  function deleteSelected() {
    void run(
      multiSelection.kind === "device"
        ? api.batchDeleteDevices(ids)
        : api.batchDeleteGroupAddresses(ids),
    );
  }

  function moveToLine(lineId: number | null) {
    void run(api.batchMoveDevicesToLine(ids, lineId));
  }

  function moveToBuildingPart(partId: number | null) {
    void run(api.batchMoveDevicesToBuildingPart(ids, partId));
  }

  const label =
    multiSelection.kind === "device"
      ? t("bulkAction.deviceLabel", { count })
      : t("bulkAction.groupAddressLabel", { count });

  const parts = flattenBuildingParts(tree.installations[0]?.buildings ?? [], []);

  return (
    <div className="bulk-action-toolbar">
      <span className="bulk-action-label">{label}</span>
      <button onClick={deleteSelected}>{t("bulkAction.delete")}</button>
      {multiSelection.kind === "device" && (
        <>
          <select
            data-role="move-line"
            value=""
            onChange={(e) => {
              const value = e.target.value;
              if (value === "") return;
              moveToLine(value === "unassigned" ? null : Number(value));
            }}
          >
            <option value="" disabled>
              {t("bulkAction.moveToLine")}
            </option>
            <option value="unassigned">{t("bulkAction.unassigned")}</option>
            {tree.installations[0]?.topology.map((area) => (
              <optgroup
                key={area.id}
                label={t("explorer.areaLabel", { address: area.address, name: area.name })}
              >
                {area.lines.map((line) => (
                  <option key={line.id} value={line.id}>
                    {t("explorer.lineLabel", { address: line.address, name: line.name })}
                  </option>
                ))}
              </optgroup>
            ))}
          </select>
          <select
            data-role="move-building-part"
            value=""
            onChange={(e) => {
              const value = e.target.value;
              if (value === "") return;
              moveToBuildingPart(value === "none" ? null : Number(value));
            }}
          >
            <option value="" disabled>
              {t("bulkAction.moveToBuildingPart")}
            </option>
            <option value="none">{t("bulkAction.none")}</option>
            {parts.map(({ node, path }) => (
              <option key={node.id} value={node.id}>
                {path}
              </option>
            ))}
          </select>
        </>
      )}
      <button aria-label={t("bulkAction.dismissSelection")} onClick={onDone}>
        ✕
      </button>
      {error && <span className="field-error">{error}</span>}
    </div>
  );
}
