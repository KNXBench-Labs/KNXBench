/** Root component wiring project state, panels, and toolbars into the KNX Web UI shell. */
import { useEffect, useRef, useState } from "react";
import { pickOpenPath, pickSavePath } from "./filePicker";
import * as api from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { DeviceDetail } from "./bindings/DeviceDetail";
import type { Selection } from "./selection";
import ProjectExplorer from "./ProjectExplorer";
import BulkActionToolbar from "./BulkActionToolbar";
import { useMultiSelection } from "./multiSelection";
import ResizablePane from "./ResizablePane";
import WorkbenchIcon from "./WorkbenchIcon";
import StructureWorkspace, { type StructureView } from "./StructureWorkspace";
import CatalogBrowser from "./CatalogBrowser";
import NewProjectDialog from "./NewProjectDialog";
import Inspector, { DeviceWorkspace } from "./Inspector";
import Search from "./Search";
import CommandPalette from "./CommandPalette";
import type { CommandContext } from "./commandRegistry";
import SettingsPanel from "./SettingsPanel";
import Dashboard from "./Dashboard";
import LogPanel from "./LogPanel";
import BusMonitorPanel from "./BusMonitorPanel";
import { publishProjectContext } from "./busContext";
import { openCompanionWindow } from "./diagnosticsWindow";
import { useAppearance } from "./appearance";
import { THEMES, useThemeId } from "./theme";
import { MOTION_LEVELS, MOTION_STYLES, useMotion } from "./motion";
import { useProductLanguage } from "./productLanguage";
import type { ProductLanguage } from "./api";
import { useTranslate } from "./i18n";
import ToastStack from "./Toast";
import { pickStartupToast, useToasts } from "./toast";
import GroupAddressCsvButtons from "./GroupAddressCsvButtons";
import DocumentationExportButton from "./DocumentationExportButton";
import ProjectDiffPanel from "./ProjectDiffPanel";

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
  // Called unconditionally on every render (not just from `SettingsPanel`,
  // which only mounts once Settings is opened) so `useUiLanguage()`'s own
  // effect — setting `document.documentElement.lang` — runs for the whole
  // session, not only for whoever happens to open Settings first. See
  // `App.test.tsx`'s "lang attribute is correct on a fresh mount" test.
  const t = useTranslate();
  // Rebuilt every render instead of hoisted to module scope: a module-level
  // `const` would call `t()` exactly once at import time and freeze the
  // filter name in whatever language happened to be active then — the same
  // trap `commandRegistry.ts`'s `COMMANDS` had before task 3's fix.
  const etsProjectFilter = [{ name: t("app.filterName.etsProject"), extensions: ["knxproj"] }];
  const knxdbFilter = [{ name: t("app.filterName.knxDesktopProject"), extensions: ["knxdb"] }];
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
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [view, setView] = useState<"overview" | StructureView>("overview");
  const [buildingScope, setBuildingScope] = useState<number | null>(null);
  // The group-address view's counterpart of `buildingScope`: which range
  // the address table is scoped to. Owned here, not inside the table, for
  // the same reason `buildingScope` is: selecting a range in the tree has
  // to move the workspace, and two copies of "the current scope" would be
  // two things that agree only by luck.
  const [addressScope, setAddressScope] = useState<number | null>(null);
  const [navigationOpen, setNavigationOpen] = useState(true);
  const [inspectorOpen, setInspectorOpen] = useState(true);
  const [catalogTarget, setCatalogTarget] = useState<{ lineId: number | null } | null>(null);
  // The from-scratch project launcher. Owned here rather than inside the
  // welcome screen because the File menu and the command palette open the
  // same dialog, and a project can be started with one already open.
  const [newProjectOpen, setNewProjectOpen] = useState(false);
  // Exactly one multi-selection for the whole shell, shared by the project
  // tree and the group-address table, feeding exactly one
  // `BulkActionToolbar` (stage 4 brief, item 2 — reuse the validated
  // commands, do not duplicate the state machine).
  const { multiSelection, onItemClick, clear: clearMultiSelection } = useMultiSelection(
    tree,
    (sel) => void selectEntity(sel),
  );
  const [themeId, setThemeId] = useThemeId();
  const appearance = useAppearance();
  const { level: motionLevel, setLevel: setMotionLevel, style: motionStyle, setStyle: setMotionStyle } = useMotion();
  const [productLanguage, setProductLanguage] = useProductLanguage();
  // `[]` both before the fetch resolves and if it fails — SettingsPanel
  // already renders that state honestly (a disabled select explaining "no
  // product database installed"), so a failed fetch needs no separate
  // error toast here.
  const [productLanguages, setProductLanguages] = useState<ProductLanguage[]>([]);

  useEffect(() => {
    api
      .productLanguages()
      .then(setProductLanguages)
      .catch(() => setProductLanguages([]));
  }, []);
  // Mirrors `selection` synchronously so in-flight device_detail responses
  // can tell, once they land, whether the selection has since moved on —
  // state updates alone are too late to check inside the same async
  // callback that reads them.
  const selectionRef = useRef<Selection | null>(null);
  // Guards every `api.deviceDetail` call site (`selectEntity`,
  // `handleTreeUpdate`, and the language-change effect below) against its
  // own stale replies: the selection check alone (`selectionRef.current`
  // still naming the same device) is always true across a language
  // change, since the selection never moves — only the language does. Two
  // rapid language switches, or a language switch racing a later
  // edit-triggered refetch, would otherwise let whichever response
  // happens to land last win, regardless of which request it answers.
  // Same `requestId` idiom `CatalogBrowser.tsx`'s `requestIdRef` and
  // `ParameterPanel.tsx`'s own use for the identical hazard.
  const deviceDetailRequestIdRef = useRef(0);

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
      if (e.target instanceof HTMLElement && e.target.closest('input, textarea, select, [contenteditable="true"], [role="dialog"]')) return;
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

  // The editing window is the only place that ever sees a `ProjectTree`
  // (there is no `GET` route that returns one — a tree only ever arrives as
  // the response to a mutation, import or open), so it is the only place
  // that can tell a companion window what the project looks like now.
  // Publishing on the `tree` state itself, rather than at each of the
  // half-dozen call sites that set it, means no edit path that *lands in
  // `tree`* can forget to — and a stale fingerprint is exactly the failure
  // the diagnostic companion's stale lock exists to prevent
  // (`busContext.ts`).
  //
  // This comment used to say "no future edit path can forget to", full
  // stop. That was false for a while: `api.setParameterValue` mutates the
  // project server-side — `domain.rs`'s `set_parameter_value_impl` runs
  // `apply(state, cmd)`, a real undoable `Command::SetParameterValue`,
  // before it returns — but answers with a `ParameterPanelDto`, never a
  // `ProjectTree`, and `ParameterPanel` had no channel back to `tree` at
  // all.
  //
  // Closed (T3, 2026-09-13; tree-sourcing corrected in T3 fix round 1,
  // 2026-09-14): `set_parameter_value_impl` now attaches its own freshly
  // rebuilt `ProjectTree` — the same one `apply(state, cmd)` already
  // produced — to a successful write's `ParameterPanelDto`.
  // `ParameterPanel` hands that tree straight to `onValueApplied`, and
  // `DeviceWorkspace` (`Inspector.tsx`) forwards it to `onApplied`
  // unchanged; no caller builds a `{...tree, can_undo, can_redo}` guess
  // anymore. So `setTree` does run, and this effect does fire, on every
  // parameter edit, with the server's own genuine `can_undo`/`can_redo`.
  //
  // What still does not move is the *fingerprint* itself:
  // `fingerprintProjectContext` (`busContext.ts`) deliberately excludes
  // parameters, so a parameter edit republishes the same fingerprint value
  // under a fresh `at` — exactly how every edit that leaves group
  // addresses untouched already behaves, not a leftover gap. The
  // remaining risk is unchanged from before this fix: the day a parameter
  // can influence a com object's DPT, links or activity, the fingerprint
  // would need to start covering it too, or this becomes a silent false
  // `"synced"`. `resolve_group_address_dpt`'s doc comment carries that
  // warning; `KNOWN_LIMITATIONS.md` §82 item 5 carries the entry.
  //
  // Never published for `tree === null`: a freshly reloaded window has no
  // tree while the server may still hold the same project open, and
  // publishing "no project" there would invent a change that never
  // happened and lock a valid session.
  useEffect(() => {
    if (tree) publishProjectContext(tree);
  }, [tree]);

  const startupToastShown = useRef(false);
  useEffect(() => {
    if (startupToastShown.current) return; // StrictMode double-invoke guard
    startupToastShown.current = true;
    const message = pickStartupToast(new Date());
    if (message) pushFun(message);
  }, []);

  // Opens (or focuses) the read-only diagnostic companion. Every outcome is
  // reported: a blocked popup and a refused webview are ordinary results on
  // the platforms this ships to, and codex-goal.md is explicit that the
  // monitor must stay fully usable in this window when no second one is
  // available — which it does, because this button adds a window and moves
  // nothing out of here.
  async function openCompanion() {
    const result = await openCompanionWindow(window.location.href);
    if (result === "blocked") pushError(t("companion.blocked"));
    else if (result === "failed") pushError(t("companion.failed"));
  }

  function resetTree(newTree: ProjectTree) {
    setTree(newTree);
    setBuildingScope(null);
    setAddressScope(null);
    // Ids from the previous project mean nothing in this one, and a stale
    // bulk selection would offer to delete whatever happens to share those
    // ids now.
    clearMultiSelection();
    selectionRef.current = null;
    setSelection(null);
    setDeviceDetail(null);
  }

  async function selectEntity(sel: Selection) {
    selectionRef.current = sel;
    setSelection(sel);
    if (sel.kind === "building_part") {
      setBuildingScope(sel.id);
      setView("buildings");
    } else if (sel.kind === "area" || sel.kind === "line") {
      setView("topology");
    } else if (sel.kind === "group_address" || sel.kind === "group_range") {
      if (sel.kind === "group_range") setAddressScope(sel.id);
      setView("addresses");
    }
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
    const requestId = ++deviceDetailRequestIdRef.current;
    try {
      const detail = await api.deviceDetail(sel.id, productLanguage);
      if (
        requestId === deviceDetailRequestIdRef.current &&
        selectionRef.current?.kind === "device" &&
        selectionRef.current.id === sel.id
      ) {
        setDeviceDetail(detail);
      }
    } catch (e) {
      if (
        requestId === deviceDetailRequestIdRef.current &&
        selectionRef.current?.kind === "device" &&
        selectionRef.current.id === sel.id
      ) {
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
    const requestId = ++deviceDetailRequestIdRef.current;
    try {
      const detail = await api.deviceDetail(sel.id, productLanguage);
      if (
        requestId === deviceDetailRequestIdRef.current &&
        selectionRef.current?.kind === "device" &&
        selectionRef.current.id === sel.id
      ) {
        setDeviceDetail(detail);
      }
    } catch (e) {
      if (
        requestId === deviceDetailRequestIdRef.current &&
        selectionRef.current?.kind === "device" &&
        selectionRef.current.id === sel.id
      ) {
        reportError(e);
      }
    }
  }

  // A language change while a device is already selected must refetch its
  // detail — otherwise the com-object names/descriptions on screen keep
  // showing the previous language until some unrelated edit happens to
  // trigger `handleTreeUpdate`. Neither `selectEntity` nor
  // `handleTreeUpdate` fires here on its own, since neither the selection
  // nor the tree changed — only the language setting did. Reads
  // `selectionRef.current` rather than the `selection` state so this
  // effect's only dependency is `productLanguage`: listing `selection`
  // too would make an ordinary device pick run this effect a second time,
  // duplicating the fetch `selectEntity` already issued for that pick.
  //
  // The selection-identity check below still matters (a device switch
  // mid-flight must still discard the reply), but on its own it guards
  // nothing against two language changes racing each other: the selection
  // never moves across a language change, so that check alone is always
  // true. `deviceDetailRequestIdRef` closes that gap — a reply is applied
  // only if it belongs to the most recently issued request across *all*
  // three call sites, so if the request for an earlier language happens
  // to resolve after a later one's, or after a later edit-triggered
  // refetch's, it loses.
  useEffect(() => {
    const sel = selectionRef.current;
    if (sel?.kind !== "device") return;
    const requestId = ++deviceDetailRequestIdRef.current;
    (async () => {
      try {
        const detail = await api.deviceDetail(sel.id, productLanguage);
        if (
          requestId === deviceDetailRequestIdRef.current &&
          selectionRef.current?.kind === "device" &&
          selectionRef.current.id === sel.id
        ) {
          setDeviceDetail(detail);
        }
      } catch (e) {
        if (
          requestId === deviceDetailRequestIdRef.current &&
          selectionRef.current?.kind === "device" &&
          selectionRef.current.id === sel.id
        ) {
          reportError(e);
        }
      }
    })();
  }, [productLanguage]);

  // Opening the dialog itself does nothing to the open project — the
  // request only leaves `NewProjectDialog` when the user submits, and the
  // server refuses it outright if that would discard unsaved edits.
  function startNewProject() {
    clearErrors();
    setNewProjectOpen(true);
  }

  // A brand-new project has no `.knxdb` behind it, so `hasStorePath` must
  // go false: otherwise the next plain Save would write this empty project
  // over whatever file the previous one came from. The backend clears its
  // own `store_path` for exactly the same reason (`new_project_impl`).
  function newProjectCreated(newTree: ProjectTree) {
    resetTree(newTree);
    setHasStorePath(false);
    setNewProjectOpen(false);
    setView("overview");
    setLogOpen(false);
    setMonitorOpen(false);
  }

  async function pickProject() {
    const path = await pickOpenPath(etsProjectFilter);
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
    const path = await pickOpenPath(knxdbFilter);
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
    const path = await pickSavePath(knxdbFilter, "project.knxdb");
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
    const path = await pickSavePath(etsProjectFilter, "project.knxproj");
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
    newProject: startNewProject,
    pickProject,
    openNativeProject,
    saveProject,
    saveProjectAs,
    undo,
    redo,
    openSearch: () => setSearchOpen(true),
    openLog: () => { setMonitorOpen(false); setLogOpen(true); },
    openBusMonitor: () => { setLogOpen(false); setMonitorOpen(true); },
    openSettings: () => setSettingsOpen(true),
    openCompanion: () => void openCompanion(),
  };

  return (
    <main className="workbench">
      <header className="workbench-toolbar">
        <a className="workbench-brand" href="#" onClick={(e) => { e.preventDefault(); setView("overview"); setLogOpen(false); setMonitorOpen(false); }}><span className="brand-mark">K</span><strong>KNXBench</strong></a>
        <details className="file-menu" onKeyDown={(e) => { if (e.key === "Escape") { e.currentTarget.open = false; e.currentTarget.querySelector("summary")?.focus(); } }}>
          <summary>{t("workbench.file")} <span aria-hidden="true">⌄</span></summary>
          <div className="file-menu-content">
      <button onClick={startNewProject}>{t("toolbar.newProject")}</button>
      <button onClick={pickProject}>{t("toolbar.openProject")}</button>
      <button onClick={openNativeProject}>{t("toolbar.openNativeProject")}</button>
      <button onClick={saveProjectAs} disabled={!tree}>
        {t("toolbar.saveAs")}
      </button>
      <button onClick={exportProject} disabled={!tree || !hasStorePath}>
        {t("toolbar.exportProject")}
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

          </div>
        </details>
        <div className="history-actions">
          <button onClick={undo} disabled={!tree?.can_undo} title={t("toolbar.undo")} aria-label={t("toolbar.undo")}><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true"><path d="M8 4L3 9l5 5 M3 9h10a6 6 0 010 12" /></svg><span className="sr-only">{t("toolbar.undo")}</span></button>
          <button onClick={redo} disabled={!tree?.can_redo} title={t("toolbar.redo")} aria-label={t("toolbar.redo")}><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true"><path d="M16 4l5 5-5 5 M21 9h-10a6 6 0 000 12" /></svg><span className="sr-only">{t("toolbar.redo")}</span></button>
        </div>
        <button className="workbench-search" onClick={() => tree && setSearchOpen(true)} disabled={!tree}>{t("toolbar.search")}<kbd>Ctrl K</kbd></button>
        <button className="command-entry" onClick={() => setPaletteOpen(true)}>{t("toolbar.commands")}</button>
        <button className="primary-action" onClick={saveProject} disabled={!tree}>{t("toolbar.save")}</button>
        <button onClick={() => setSettingsOpen(true)} title={t("toolbar.settings")} aria-label={t("toolbar.settings")}><GearIcon /></button>
      </header>
      <div className="workbench-panel-controls">
        <button aria-expanded={navigationOpen} onClick={() => setNavigationOpen(!navigationOpen)}><WorkbenchIcon name="panel" />{t("workbench.navigation")}</button>
        {tree && multiSelection && multiSelection.ids.size > 0 && (
          <BulkActionToolbar
            multiSelection={multiSelection}
            tree={tree}
            onTreeUpdate={handleTreeUpdate}
            onDone={clearMultiSelection}
          />
        )}
        {tree && (tree.errors > 0 || tree.warnings > 0) && <button className="import-notice" onClick={() => { setView("overview"); setLogOpen(false); setMonitorOpen(false); }}>{t("workbench.importNotices", { errors: tree.errors, warnings: tree.warnings })}</button>}
        <button aria-expanded={inspectorOpen} onClick={() => setInspectorOpen(!inspectorOpen)}>{t("workbench.properties")}<WorkbenchIcon name="panel" /></button>
      </div>
      <div className="workspace workbench-body">
        {navigationOpen && <ResizablePane label={t("workbench.navigation")} side="left" initialWidth={250} min={200} max={480}>
          <nav className="workbench-navigation" aria-label={t("workbench.navigation")}>
            {(["overview", "buildings", "topology", "addresses"] as const).map((item) => <button key={item} aria-current={!logOpen && !monitorOpen && view === item ? "page" : undefined} onClick={() => { setView(item); setLogOpen(false); setMonitorOpen(false); }}><WorkbenchIcon name={item} />{t(`workbench.${item}`)}</button>)}
            <button onClick={() => setCatalogTarget({ lineId: selection?.kind === "line" ? selection.id : null })}><WorkbenchIcon name="catalog" />{t("workbench.catalog")}</button>
          </nav>
          {tree && <ProjectExplorer tree={tree} selection={selection} onSelect={selectEntity} onTreeUpdate={handleTreeUpdate} multiSelection={multiSelection} onItemClick={onItemClick} />}
          <nav className="workbench-navigation diagnostic-navigation" aria-label={t("toolbar.busMonitor")}>
            <button aria-current={monitorOpen ? "page" : undefined} onClick={() => { setLogOpen(false); setMonitorOpen((open) => !open); }}><WorkbenchIcon name="monitor" />{t("toolbar.busMonitor")}</button>
            <button aria-current={logOpen ? "page" : undefined} onClick={() => { setMonitorOpen(false); setLogOpen((open) => !open); }}><WorkbenchIcon name="log" />{t("toolbar.log")}</button>
            <button className="companion-open" onClick={() => void openCompanion()}><WorkbenchIcon name="panel" />{t("companion.open")}</button>
            <button onClick={() => setSettingsOpen(true)}><GearIcon />{t("toolbar.settings")}</button>
          </nav>
        </ResizablePane>}
        <div className="workbench-center">
          {logOpen ? <LogPanel tree={tree} refreshKey={logVersion} /> : monitorOpen ? <BusMonitorPanel projectOpen={tree !== null} /> : tree ? (
            view === "overview" ? <Dashboard tree={tree} /> : <StructureWorkspace tree={tree} view={view} selection={selection} buildingScope={buildingScope} onBuildingScope={setBuildingScope}
              rangeScope={addressScope} onRangeScope={setAddressScope}
              multiSelection={multiSelection} onItemClick={onItemClick} onTreeUpdate={handleTreeUpdate}
              addressActions={<GroupAddressCsvButtons tree={tree} onTreeUpdate={handleTreeUpdate} onSummary={pushFun} onError={reportError} onClearErrors={clearErrors} />}
              onSelect={selectEntity} onCatalog={(lineId) => setCatalogTarget({ lineId })} />
          ) : <section className="welcome-workspace"><span className="eyebrow">KNX-compatible · Linux-first</span><h1>{t("workbench.welcome")}</h1><p>{t("workbench.openHint")}</p><div><button className="primary-action" onClick={startNewProject}>{t("toolbar.newProject")}</button><button onClick={pickProject}>{t("toolbar.openProject")}</button><button onClick={openNativeProject}>{t("toolbar.openNativeProject")}</button></div></section>}
          {tree && selection?.kind === "device" && deviceDetail && !logOpen && !monitorOpen && <DeviceWorkspace key={deviceDetail.id} detail={deviceDetail} tree={tree} onApplied={handleTreeUpdate} />}
        </div>
        {inspectorOpen && !logOpen && !monitorOpen && <ResizablePane label={t("workbench.properties")} side="right" initialWidth={360} min={280} max={700}>
          <header className="inspector-heading">{t("workbench.properties")}</header>
          {tree && selection ? <Inspector propertiesOnly key={`${selection.kind}-${selection.id}`} selection={selection} tree={tree} deviceDetail={deviceDetail} onApplied={handleTreeUpdate} onDeleted={resetTree} /> : <p className="inspector-empty">{t("workbench.noSelection")}</p>}
        </ResizablePane>}
      </div>
      <footer className="workbench-status"><span>{tree ? tree.installations.map((i) => i.name).join(" / ") : "KNXBench"}</span><span>KNX-compatible</span></footer>
      <ToastStack toasts={toasts} onDismiss={dismiss} />
      {catalogTarget && <CatalogBrowser lineId={catalogTarget.lineId} onCreated={handleTreeUpdate} onClose={() => setCatalogTarget(null)} />}
      {newProjectOpen && <NewProjectDialog onCreated={newProjectCreated} onClose={() => setNewProjectOpen(false)} />}
      {tree && searchOpen && (
        <Search tree={tree} onSelect={selectEntity} onClose={() => setSearchOpen(false)} />
      )}
      {paletteOpen && <CommandPalette ctx={ctx} onClose={() => setPaletteOpen(false)} />}
      {settingsOpen && (
        <SettingsPanel
          appearance={appearance}
          themes={THEMES}
          activeThemeId={themeId}
          onSelectTheme={setThemeId}
          motionStyles={MOTION_STYLES}
          activeMotionStyle={motionStyle}
          onSelectMotionStyle={setMotionStyle}
          motionLevels={MOTION_LEVELS}
          activeMotionLevel={motionLevel}
          onSelectMotionLevel={setMotionLevel}
          productLanguages={productLanguages}
          activeProductLanguage={productLanguage}
          onSelectProductLanguage={setProductLanguage}
          onClose={() => setSettingsOpen(false)}
        />
      )}
    </main>
  );
}

// Cycle 13's ThemeToggle drew its sun/moon/monitor icons the same way:
// hand-written inline SVG, no icon library. This gear is that convention's
// one new member.
function GearIcon() {
  return (
    <svg
      width="18"
      height="18"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <circle cx="12" cy="12" r="3.5" />
      <line x1="12" y1="1.5" x2="12" y2="4.5" />
      <line x1="12" y1="19.5" x2="12" y2="22.5" />
      <line x1="1.5" y1="12" x2="4.5" y2="12" />
      <line x1="19.5" y1="12" x2="22.5" y2="12" />
      <line x1="4.6" y1="4.6" x2="6.7" y2="6.7" />
      <line x1="17.3" y1="17.3" x2="19.4" y2="19.4" />
      <line x1="19.4" y1="4.6" x2="17.3" y2="6.7" />
      <line x1="6.7" y1="17.3" x2="4.6" y2="19.4" />
    </svg>
  );
}

export default App;
