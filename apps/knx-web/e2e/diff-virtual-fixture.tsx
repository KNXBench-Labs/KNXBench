/** Renders the real ProjectDiffDetails for a synthetic 3,300-entry group-address diff. */
import { createRoot } from "react-dom/client";
import type { EntityChange, GroupAddressFields, GroupAddressKey, ProjectDiffReport } from "../src/api";
import ProjectDiffDetails from "../src/ProjectDiffDetails";
import "../src/styles.css";

// 3,000 one-line added rows and 300 changed rows carrying a field table of
// one to three lines, so row heights really vary.
const fields = (name: string): GroupAddressFields => ({ name, central: false, unfiltered: false, range: null });
const added: [GroupAddressKey, GroupAddressFields][] = Array.from({ length: 3000 }, (_, i) => [
  { etsId: null, address: `1/${Math.floor(i / 250)}/${i % 250}` },
  fields(`GA ${i}`),
]);
const changed: EntityChange<GroupAddressKey, GroupAddressFields>[] = Array.from({ length: 300 }, (_, i) => ({
  key: { etsId: null, address: `3/${Math.floor(i / 100)}/${i % 100}` },
  matchedBy: "naturalKey",
  left: fields(`Old ${i}`),
  right: fields(`Changed ${i}`),
  changedFields: ["name"],
  fieldChanges: Array.from({ length: 1 + (i % 3) }, (_, f) => ({ field: `field${f}`, left: `before ${i}`, right: `after ${i}` })),
}));
const empty = { added: [], removed: [], changed: [], ambiguous: [] };
const report: ProjectDiffReport = {
  inputKind: "knxdb",
  importReport: null,
  importDiagnostics: [],
  infoChanges: [],
  installations: [{
    id: 0, status: "matched", fieldChanges: [], areas: empty, lines: empty, devices: empty,
    groupRanges: empty, groupAddresses: { added, removed: [], changed, ambiguous: [] }, buildings: empty,
  }],
};

createRoot(document.getElementById("root")!).render(
  <main className="workbench" style={{ padding: "1rem", maxWidth: "60rem" }}>
    <ProjectDiffDetails report={report} />
  </main>,
);
