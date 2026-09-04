import { useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { DeviceDetail } from "./bindings/DeviceDetail";
import ProjectExplorer from "./ProjectExplorer";
import Inspector from "./Inspector";

const KNXDB_FILTER = [{ name: "knx-desktop project", extensions: ["knxdb"] }];

function App() {
  const [tree, setTree] = useState<ProjectTree | null>(null);
  const [error, setError] = useState<string | null>(null);
  // Whether the backend's `AppState.store_path` is set — mirrored here only
  // so "Save" knows whether it can skip the dialog; the backend remains the
  // source of truth and still refuses `save_project` if this ever drifts.
  const [hasStorePath, setHasStorePath] = useState(false);
  const [selectedDeviceId, setSelectedDeviceId] = useState<number | null>(null);
  const [deviceDetail, setDeviceDetail] = useState<DeviceDetail | null>(null);

  function resetTree(newTree: ProjectTree) {
    setTree(newTree);
    setSelectedDeviceId(null);
    setDeviceDetail(null);
  }

  async function selectDevice(id: number) {
    setSelectedDeviceId(id);
    setError(null);
    try {
      setDeviceDetail(await invoke<DeviceDetail>("device_detail", { deviceId: id }));
    } catch (e) {
      setError(String(e));
      setDeviceDetail(null);
    }
  }

  // After any command/undo/redo: the tree refreshes unconditionally (an
  // address edit changes its label), and the currently selected device's
  // detail refreshes alongside it (its own fields, or nothing if the edit
  // targeted a different device — device_detail is cheap enough to always
  // refetch rather than track which device a given command touched).
  async function handleTreeUpdate(newTree: ProjectTree) {
    setTree(newTree);
    if (selectedDeviceId !== null) {
      try {
        setDeviceDetail(await invoke<DeviceDetail>("device_detail", { deviceId: selectedDeviceId }));
      } catch (e) {
        setError(String(e));
      }
    }
  }

  async function pickProject() {
    const path = await open({
      multiple: false,
      filters: [{ name: "ETS project", extensions: ["knxproj"] }],
    });
    if (typeof path !== "string") return;
    setError(null);
    try {
      resetTree(await invoke<ProjectTree>("open_project", { path }));
      setHasStorePath(false); // ETS import has no `.knxdb` location yet
    } catch (e) {
      setError(String(e));
    }
  }

  async function openNativeProject() {
    const path = await open({ multiple: false, filters: KNXDB_FILTER });
    if (typeof path !== "string") return;
    setError(null);
    try {
      resetTree(await invoke<ProjectTree>("open_native_project", { path }));
      setHasStorePath(true);
    } catch (e) {
      setError(String(e));
    }
  }

  async function saveProjectAs() {
    const path = await save({ filters: KNXDB_FILTER, defaultPath: "project.knxdb" });
    if (typeof path !== "string") return;
    setError(null);
    try {
      await invoke("save_project_as", { path });
      setHasStorePath(true);
    } catch (e) {
      setError(String(e));
    }
  }

  async function saveProject() {
    if (!hasStorePath) return saveProjectAs();
    setError(null);
    try {
      await invoke("save_project");
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <main>
      <button onClick={pickProject}>Open project…</button>
      <button onClick={openNativeProject}>Open (.knxdb)…</button>
      <button onClick={saveProject} disabled={!tree}>
        Save
      </button>
      <button onClick={saveProjectAs} disabled={!tree}>
        Save As…
      </button>
      {error && (
        <p role="alert" className="error-banner">
          {error}
        </p>
      )}
      {tree && (
        <div className="workspace">
          <ProjectExplorer tree={tree} selectedId={selectedDeviceId} onSelectDevice={selectDevice} />
          {deviceDetail && (
            <Inspector key={deviceDetail.id} detail={deviceDetail} onApplied={handleTreeUpdate} />
          )}
        </div>
      )}
    </main>
  );
}

export default App;
