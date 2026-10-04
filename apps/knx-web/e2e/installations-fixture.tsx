/** Real Explorer and project Inspector over a served tree; Playwright mocks every API call. */
import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import * as api from "../src/api";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import Inspector from "../src/Inspector";
import ProjectExplorer from "../src/ProjectExplorer";
import "../src/styles.css";

function Fixture() {
  const [tree, setTree] = useState<ProjectTree | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => { void api.currentProject().then(setTree, (e) => setError(api.errorMessage(e))); }, []);
  if (error) return <p role="alert">{error}</p>;
  if (!tree) return <p>Loading</p>;
  return (
    <main className="workbench" style={{ display: "grid", gridTemplateColumns: "1fr 1fr" }}>
      <ProjectExplorer tree={tree} onTreeUpdate={setTree} onSummary={() => {}}
        onError={(e) => setError(api.errorMessage(e))} selection={{ kind: "project", id: 0 }}
        onSelect={() => {}} multiSelection={null} onItemClick={() => {}} />
      <Inspector selection={{ kind: "project", id: 0 }} tree={tree} deviceDetail={null}
        onApplied={setTree} onDeleted={setTree} />
    </main>
  );
}

createRoot(document.getElementById("root")!).render(<Fixture />);
