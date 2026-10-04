/** Real Explorer and project Inspector over a served tree; Playwright mocks every API call. */
import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import * as api from "../src/api";
import type { DeviceDetail } from "../src/bindings/DeviceDetail";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import type { Selection } from "../src/selection";
import Inspector from "../src/Inspector";
import ProjectExplorer from "../src/ProjectExplorer";
import "../src/styles.css";

// `?workspace` renders the device workspace (communication objects) too.
const showWorkspace = new URLSearchParams(location.search).has("workspace");

function Fixture() {
  const [tree, setTree] = useState<ProjectTree | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selection, setSelection] = useState<Selection>({ kind: "project", id: 0 });
  const [detail, setDetail] = useState<DeviceDetail | null>(null);
  useEffect(() => {
    setDetail(null);
    if (selection.kind === "device") void api.deviceDetail(selection.id).then(setDetail, (e) => setError(api.errorMessage(e)));
  }, [selection]);
  useEffect(() => { void api.currentProject().then(setTree, (e) => setError(api.errorMessage(e))); }, []);
  if (error) return <p role="alert">{error}</p>;
  if (!tree) return <p>Loading</p>;
  return (
    <main className="workbench" style={{ display: "grid", gridTemplateColumns: "1fr 1fr" }}>
      <ProjectExplorer tree={tree} onTreeUpdate={setTree} onSummary={() => {}}
        onError={(e) => setError(api.errorMessage(e))} selection={selection}
        onSelect={setSelection} multiSelection={null} onItemClick={(_event, _kind, _id, item) => setSelection(item)} />
      <Inspector propertiesOnly={!showWorkspace} selection={selection} tree={tree} deviceDetail={detail}
        onApplied={setTree} onDeleted={setTree} />
    </main>
  );
}

createRoot(document.getElementById("root")!).render(<Fixture />);
