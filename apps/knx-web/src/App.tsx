import { useEffect, useRef, useState } from "react";
import { pickOpenPath, pickSavePath } from "./filePicker";
import * as api from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { DeviceDetail } from "./bindings/DeviceDetail";
import type { Selection } from "./selection";
import ProjectExplorer from "./ProjectExplorer";
import Inspector from "./Inspector";
import Search from "./Search";
import CommandPalette from "./CommandPalette";
import type { CommandContext } from "./commandRegistry";
import ThemeSwitcher from "./ThemeSwitcher";
import Dashboard from "./Dashboard";
import LogPanel from "./LogPanel";
import BusMonitorPanel from "./BusMonitorPanel";
import { THEMES, useThemeId } from "./theme";
import ToastStack from "./Toast";
import { pickStartupToast, useToasts } from "./toast";
import GroupAddressCsvButtons from "./GroupAddressCsvButtons";
import DocumentationExportButton from "./DocumentationExportButton";
import ProjectDiffPanel from "./ProjectDiffPanel";

const KNXDB_FILTER = [{ name: "knx-desktop project", extensions: ["knxdb"] }];
const EXPORT_FILTER = [{ name: "ETS project", extensions: ["knxproj"] }];

// `ExportWarningDto` (apps/knx-server/src/routes.rs) has no `tag` attribute,
// so serde serializes it externally tagged: `{ "unsigned": { "detail":
// "..." } }`, `{ "missingManufacturerData": { "sourcePath": "...", "sha256":
// "..." } }`, etc — one key, whose value is the variant's fields. Unwrap
// that single key, use `detail` if the variant has one, else fall back to
// stringifying the inner value (covers `ManufacturerDataFromProductDb`'s
// `entries`/`StaleSignature`'s `sourcePath`/`MissingManufacturerData`'s
// `sourcePath`+`sha256`, none of which carry a `detail` field).
function describeExportWarning(w: unknown): string {
  if (typeof w === "object" && w !== null) {
    const [variant, value] = Object.entries(w)[0] ?? [];
    if (typeof value === "object" && value !== null) {
      if ("detail" in value) return String((value as { detail: unknown }).detail);
      return `${variant}: ${JSON.stringify(value)}`;
    }
  }
  return JSON.stringify(w);
}

function App() {
  const [tree, setTree] = useState<ProjectTree | null>(null);
  const { toasts, pushError, clearErrors, pushFun, dismiss } = useToasts();
  // Bumped on every error path below, threaded into `LogPanel` as a second
  // effect dependency alongside `tree`. `tree` only changes on a
  // *successful* operation, so without this a failed save/export/edit/
  // undo/redo/import pushes an error entry on the server that the open Log
  // tab would not show until some unrelated successful operation happened
  // to change the tree — exactly the case this feature exists for.
  const [logVersion, setLogVersion] = useState(0);

  function reportError(e: unknown) {
    setLogVersion((v) => v + 1);
    pushError(api.errorMessage(e));
  }
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
  const [logOpen, setLogOpen] = useState(false);
  const [monitorOpen, setMonitorOpen] = useState(false);
  const [themeId, setThemeId] = useThemeId();
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
    setLogOpen(false);
    setMonitorOpen(false);
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
        reportError(e);
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
        reportError(e);
      }
    }
  }

  async function pickProject() {
    const path = await pickOpenPath([{ name: "ETS project", extensions: ["knxproj"] }]);
    if (!path) return;
    clearErrors();
    try {
      resetTree(await api.importProject(path));
      setHasStorePath(false); // ETS import has no `.knxdb` location yet
    } catch (e) {
      reportError(e);
    }
  }

  async function openNativeProject() {
    const path = await pickOpenPath(KNXDB_FILTER);
    if (!path) return;
    clearErrors();
    try {
      resetTree(await api.openProject(path));
      setHasStorePath(true);
    } catch (e) {
      reportError(e);
    }
  }

  async function saveProjectAs() {
    const path = await pickSavePath(KNXDB_FILTER, "project.knxdb");
    if (!path) return;
    clearErrors();
    try {
      await api.saveProjectAs(path);
      setHasStorePath(true);
    } catch (e) {
      reportError(e);
    }
  }

  async function saveProject() {
    if (!hasStorePath) return saveProjectAs();
    clearErrors();
    try {
      await api.saveProject();
    } catch (e) {
      reportError(e);
    }
  }

  async function exportProject() {
    const path = await pickSavePath(EXPORT_FILTER, "project.knxproj");
    if (!path) return;
    clearErrors();
    try {
      const { warnings } = await api.exportProject(path);
      if (warnings.length > 0) {
        // `pushError` is single-slot (each call evicts the previous error
        // toast — see toast.ts's own doc comment), so N separate calls in a
        // loop would only ever leave the last warning visible. Export
        // warnings are commonly plural (one `MissingManufacturerData` per
        // unresolved manufacturer reference, see knx-app's export code), so
        // all of them are joined into a single toast instead.
        pushError(warnings.map(describeExportWarning).join(" | "));
      }
    } catch (e) {
      reportError(e);
    }
  }

  async function undo() {
    clearErrors();
    try {
      await handleTreeUpdate(await api.undo());
    } catch (e) {
      reportError(e);
    }
  }

  async function redo() {
    clearErrors();
    try {
      await handleTreeUpdate(await api.redo());
    } catch (e) {
      reportError(e);
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
      <button onClick={exportProject} disabled={!tree || !hasStorePath}>
        Export to .knxproj…
      </button>
      <GroupAddressCsvButtons
        tree={tree}
        onTreeUpdate={handleTreeUpdate}
        onSummary={pushFun}
        onError={reportError}
        onClearErrors={clearErrors}
      />
      <DocumentationExportButton
        tree={tree}
        onSummary={pushFun}
        onError={reportError}
        onClearErrors={clearErrors}
      />
      <ProjectDiffPanel tree={tree} onError={reportError} onClearErrors={clearErrors} />
      <button onClick={undo} disabled={!tree?.can_undo}>
        Undo
      </button>
      <button onClick={redo} disabled={!tree?.can_redo}>
        Redo
      </button>
      <button onClick={() => tree && setSearchOpen(true)} disabled={!tree}>
        Search… (Ctrl+K)
      </button>
      {/* Enabled with no project open: `GET /api/log` deliberately works
          then too (routes.rs), specifically so a failed import with
          nothing loaded still leaves an inspectable trail
          (KNOWN_LIMITATIONS.md #36, part A). */}
      <button
        onClick={() => {
          setMonitorOpen(false);
          setLogOpen((open) => !open);
        }}
      >
        Log
      </button>
      {/* Also enabled with no project open, same reasoning as the Log
          button above: the bus monitor talks straight to a KNXnet/IP
          gateway (`apps/knx-server/src/bus_routes.rs`), not to the open
          project — a project only supplies group-address names and DPTs
          for decoding, so with none open the table still works, it just
          shows raw addresses and undecoded/`unresolved` rows
          (KNOWN_LIMITATIONS.md #36, part A, same slot the Log panel
          uses). */}
      <button
        onClick={() => {
          setLogOpen(false);
          setMonitorOpen((open) => !open);
        }}
      >
        Bus monitor
      </button>
      <button
        onClick={() => {
          setSearchOpen(false);
          setPaletteOpen(true);
        }}
      >
        Commands… (Ctrl+Shift+P)
      </button>
      <ThemeSwitcher themes={THEMES} activeId={themeId} onSelect={setThemeId} />
      <ToastStack toasts={toasts} onDismiss={dismiss} />
      {/* `ProjectExplorer` genuinely needs a project; `Inspector`/`Dashboard`
          likewise. `LogPanel`/`BusMonitorPanel` alone do not
          (KNOWN_LIMITATIONS.md #36, part A) — with no project open and
          neither tab open, there is nothing for this slot to show, so it
          stays unrendered same as before; with either tab open, that panel
          is the only thing in here (they are mutually exclusive — opening
          one closes the other, same slot). */}
      {(tree || logOpen || monitorOpen) && (
        <div className="workspace">
          {tree && (
            <ProjectExplorer
              tree={tree}
              selection={selection}
              onSelect={selectEntity}
              onTreeUpdate={handleTreeUpdate}
            />
          )}
          {logOpen ? (
            <LogPanel tree={tree} refreshKey={logVersion} />
          ) : monitorOpen ? (
            <BusMonitorPanel />
          ) : (
            tree &&
            (selection ? (
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
            ))
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
