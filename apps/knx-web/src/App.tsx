import { useEffect, useRef, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import * as api from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { DeviceDetail } from "./bindings/DeviceDetail";
import type { Selection } from "./selection";
import ProjectExplorer from "./ProjectExplorer";
import Inspector from "./Inspector";
import Search from "./Search";
import CommandPalette from "./CommandPalette";
import type { CommandContext } from "./commandRegistry";
import ThemeToggle from "./ThemeToggle";
import Dashboard from "./Dashboard";
import { useTheme } from "./theme";
import ToastStack from "./Toast";
import { pickStartupToast, useToasts } from "./toast";

const KNXDB_FILTER = [{ name: "knx-desktop project", extensions: ["knxdb"] }];

function App() {
  const [tree, setTree] = useState<ProjectTree | null>(null);
  const { toasts, pushError, clearErrors, pushFun, dismiss } = useToasts();
  // Whether the backend's `AppState.store_path` is set — mirrored here so
  // "Save" knows whether it can skip the dialog. Safety here rests on this
  // flag staying in lockstep with the backend's own `store_path`: the
  // backend does NOT check which project is loaded against `store_path`
  // before writing — `save_project` just writes wherever `store_path`
  // points. See docs/KNOWN_LIMITATIONS.md for the tracked gap (importing a
  // fresh `.knxproj` while `store_path` still points at a different
  // `.knxdb` would let a subsequent Save overwrite the wrong file).
  const [hasStorePath, setHasStorePath] = useState(false);
  const [selection, setSelection] = useState<Selection | null>(null);
  const [deviceDetail, setDeviceDetail] = useState<DeviceDetail | null>(null);
  const [searchOpen, setSearchOpen] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [theme, cycleTheme] = useTheme();
  // Mirrors `selection` synchronously so in-flight device_detail responses
  // can tell, once they land, whether the selection has since moved on —
  // state updates alone are too late to check inside the same async
  // callback that reads them.
  const selectionRef = useRef<Selection | null>(null);

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        if (tree) {
          setPaletteOpen(false);
          setSearchOpen(true);
        }
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.shiftKey && e.key.toLowerCase() === "p") {
        e.preventDefault();
        setSearchOpen(false);
        setPaletteOpen(true);
        return;
      }
      if (!(e.ctrlKey || e.metaKey) || e.key.toLowerCase() !== "z") return;
      e.preventDefault();
      if (e.shiftKey) {
        if (tree?.can_redo) void redo();
      } else {
        if (tree?.can_undo) void undo();
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  });

  const startupToastShown = useRef(false);
  useEffect(() => {
    if (startupToastShown.current) return; // StrictMode double-invoke guard
    startupToastShown.current = true;
    const message = pickStartupToast(new Date());
    if (message) pushFun(message);
  }, []);

  function resetTree(newTree: ProjectTree) {
    setTree(newTree);
    selectionRef.current = null;
    setSelection(null);
    setDeviceDetail(null);
  }

  async function selectEntity(sel: Selection) {
    selectionRef.current = sel;
    setSelection(sel);
    clearErrors();
    if (sel.kind !== "device") {
      // Group-address/building-part detail resolves synchronously from
      // `tree` inside Inspector — nothing to fetch, and no stale
      // `deviceDetail` should linger from a previous device selection.
      setDeviceDetail(null);
      return;
    }
    try {
      const detail = await api.deviceDetail(sel.id);
      if (selectionRef.current?.kind === "device" && selectionRef.current.id === sel.id) {
        setDeviceDetail(detail);
      }
    } catch (e) {
      if (selectionRef.current?.kind === "device" && selectionRef.current.id === sel.id) {
        pushError(String(e));
        setDeviceDetail(null);
      }
    }
  }

  // After any command/undo/redo: the tree refreshes unconditionally (an
  // address edit changes its label), and — if a device is currently
  // selected — its detail refreshes alongside it (its own fields, or
  // nothing if the edit targeted a different device — device_detail is
  // cheap enough to always refetch rather than track which device a given
  // command touched). A group-address or building-part selection needs no
  // refetch: Inspector reads it straight out of the refreshed `tree` prop.
  // If the selection has moved on by the time this response lands (the
  // user clicked another device while this edit's request was in flight),
  // the stale result is discarded instead of overwriting the newly
  // selected device's detail.
  async function handleTreeUpdate(newTree: ProjectTree) {
    setTree(newTree);
    const sel = selectionRef.current;
    if (sel?.kind !== "device") return;
    try {
      const detail = await api.deviceDetail(sel.id);
      if (selectionRef.current?.kind === "device" && selectionRef.current.id === sel.id) {
        setDeviceDetail(detail);
      }
    } catch (e) {
      if (selectionRef.current?.kind === "device" && selectionRef.current.id === sel.id) {
        pushError(String(e));
      }
    }
  }

  async function pickProject() {
    const path = await open({
      multiple: false,
      filters: [{ name: "ETS project", extensions: ["knxproj"] }],
    });
    if (typeof path !== "string") return;
    clearErrors();
    try {
      resetTree(await api.importProject(path));
      setHasStorePath(false); // ETS import has no `.knxdb` location yet
    } catch (e) {
      pushError(String(e));
    }
  }

  async function openNativeProject() {
    const path = await open({ multiple: false, filters: KNXDB_FILTER });
    if (typeof path !== "string") return;
    clearErrors();
    try {
      resetTree(await api.openProject(path));
      setHasStorePath(true);
    } catch (e) {
      pushError(String(e));
    }
  }

  async function saveProjectAs() {
    const path = await save({ filters: KNXDB_FILTER, defaultPath: "project.knxdb" });
    if (typeof path !== "string") return;
    clearErrors();
    try {
      await api.saveProjectAs(path);
      setHasStorePath(true);
    } catch (e) {
      pushError(String(e));
    }
  }

  async function saveProject() {
    if (!hasStorePath) return saveProjectAs();
    clearErrors();
    try {
      await api.saveProject();
    } catch (e) {
      pushError(String(e));
    }
  }

  async function undo() {
    clearErrors();
    try {
      await handleTreeUpdate(await api.undo());
    } catch (e) {
      pushError(String(e));
    }
  }

  async function redo() {
    clearErrors();
    try {
      await handleTreeUpdate(await api.redo());
    } catch (e) {
      pushError(String(e));
    }
  }

  const ctx: CommandContext = {
    tree,
    pickProject,
    openNativeProject,
    saveProject,
    saveProjectAs,
    undo,
    redo,
    openSearch: () => setSearchOpen(true),
  };

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
      <button onClick={undo} disabled={!tree?.can_undo}>
        Undo
      </button>
      <button onClick={redo} disabled={!tree?.can_redo}>
        Redo
      </button>
      <button onClick={() => tree && setSearchOpen(true)} disabled={!tree}>
        Search… (Ctrl+K)
      </button>
      <button
        onClick={() => {
          setSearchOpen(false);
          setPaletteOpen(true);
        }}
      >
        Commands… (Ctrl+Shift+P)
      </button>
      <ThemeToggle theme={theme} onCycle={cycleTheme} />
      <ToastStack toasts={toasts} onDismiss={dismiss} />
      {tree && (
        <div className="workspace">
          <ProjectExplorer
            tree={tree}
            selection={selection}
            onSelect={selectEntity}
            onTreeUpdate={handleTreeUpdate}
          />
          {selection ? (
            <Inspector
              key={`${selection.kind}-${selection.id}`}
              selection={selection}
              tree={tree}
              deviceDetail={deviceDetail}
              onApplied={handleTreeUpdate}
              onDeleted={resetTree}
            />
          ) : (
            <Dashboard tree={tree} />
          )}
        </div>
      )}
      {tree && searchOpen && (
        <Search tree={tree} onSelect={selectEntity} onClose={() => setSearchOpen(false)} />
      )}
      {paletteOpen && <CommandPalette ctx={ctx} onClose={() => setPaletteOpen(false)} />}
    </main>
  );
}

export default App;
