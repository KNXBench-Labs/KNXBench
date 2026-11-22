/** Mounts the real DeviceConfigPanel; the spec mocks every /api request. */
import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import DeviceConfigPanel from "../src/DeviceConfigPanel";
import * as api from "../src/api";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import { setSetting } from "../src/settingsStore";
import { UI_LANGUAGE_STORAGE_KEY } from "../src/uiLanguage";
import "../src/styles.css";

setSetting(UI_LANGUAGE_STORAGE_KEY, new URLSearchParams(location.search).get("lang") === "de" ? "de" : "en");

const treeOf = (revision: number) => ({ server_incarnation: "device-config-fixture", project_incarnation: 1, snapshot_revision: revision } as unknown as ProjectTree);

function App() {
  const [tree, setTree] = useState(treeOf(7));
  // The spec moves the revision as an undo or another editor would.
  useEffect(() => {
    const reload = () => { void api.currentProject().then(setTree); };
    window.addEventListener("device-config-external-change", reload);
    return () => window.removeEventListener("device-config-external-change", reload);
  }, []);
  // The real parent re-reads the project after an apply; so does this stand-in.
  return <DeviceConfigPanel deviceId={3} tree={tree} onApplied={async () => { setTree(await api.currentProject()); }} />;
}

createRoot(document.getElementById("root")!).render(<App />);
