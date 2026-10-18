/** Exercises real device and comparison tables with fictional offline data. */
import { useState } from "react";
import { createRoot } from "react-dom/client";
import DevicesWorkspace from "../src/DevicesWorkspace";
import ComparisonTable from "../src/ComparisonTable";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import { setSetting } from "../src/settingsStore";
import { useThemeId } from "../src/theme";
import { useAppearance } from "../src/appearance";
import "../src/styles.css";
import "../src/deviceComparisons.css";

const params = new URLSearchParams(location.search);
setSetting("uiLanguage", params.get("lang") === "de" ? "de" : "en");
setSetting("theme", params.get("theme") ?? "porcelain");
const tree: ProjectTree = { schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false,
  is_modified: false, group_address_style: "ThreeLevel", server_incarnation: "table-columns", snapshot_revision: 1,
  installations: [{ id: 1, name: "Fictional house", topology: [], buildings: [], group_addresses: [], group_ranges: [],
    unassigned: Array.from({ length: 80 }, (_, index) => ({ id: index + 1, name: `Device ${index + 1}`, address: `1.1.${index + 1}`, description: "Synthetic device", com_object_count: 0 })) }] };
function Fixture() {
  useThemeId(); useAppearance();
  const [clicks, setClicks] = useState(0);
  return <main className="workbench" style={{ display: "block", padding: 12, maxWidth: "100%", boxSizing: "border-box" }}>
    <DevicesWorkspace tree={tree} active selection={null} multiSelection={null} onItemClick={() => setClicks(value => value + 1)} onCatalogue={() => {}} />
    <output data-row-clicks>{clicks}</output>
    <ComparisonTable rows={[{ kind: "raw", label: "Synthetic memory", address: 0x4000, project: "01 02 03", device: "01 02 04", reason: null }]} />
  </main>;
}
createRoot(document.getElementById("root")!).render(<Fixture />);
