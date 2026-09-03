import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import type { ProjectTree } from "./bindings/ProjectTree";
import ProjectExplorer from "./ProjectExplorer";

function App() {
  const [tree, setTree] = useState<ProjectTree | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function pickProject() {
    const path = await open({
      multiple: false,
      filters: [{ name: "ETS project", extensions: ["knxproj"] }],
    });
    if (typeof path !== "string") return;
    setError(null);
    try {
      setTree(await invoke<ProjectTree>("open_project", { path }));
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <main>
      <button onClick={pickProject}>Open project…</button>
      {error && (
        <p role="alert" className="error-banner">
          {error}
        </p>
      )}
      {tree && <ProjectExplorer tree={tree} />}
    </main>
  );
}

export default App;
