/** Root component wiring project state, panels, and toolbars into the KNX Web UI shell. */
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
import SettingsPanel from "./SettingsPanel";
import Dashboard from "./Dashboard";
import LogPanel from "./LogPanel";
import BusMonitorPanel from "./BusMonitorPanel";
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
  const [themeId, setThemeId] = useThemeId();
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
  // Guards the language-change effect below against its own stale
  // replies: the selection check alone (`selectionRef.current` still
  // naming the same device) is always true across a language change,
  // since the selection never moves — only the language does. Two rapid
  // language switches would otherwise race, with whichever response
  // happens to land last winning regardless of which request it answers.
  // Same `requestId` idiom `CatalogBrowser.tsx`'s `requestIdRef` and
  // `ParameterPanel.tsx`'s own use for the identical hazard.
  const languageRequestIdRef = useRef(0);

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
      const detail = await api.deviceDetail(sel.id, productLanguage);
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
      const detail = await api.deviceDetail(sel.id, productLanguage);
      if (selectionRef.current?.kind === "device" && selectionRef.current.id === sel.id) {
        setDeviceDetail(detail);
      }
    } catch (e) {
      if (selectionRef.current?.kind === "device" && selectionRef.current.id === sel.id) {
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
  // true. `languageRequestIdRef` closes that gap — a reply is applied
  // only if it belongs to the most recently issued request, so if the
  // request for an earlier language happens to resolve after a later
  // one's, it loses.
  useEffect(() => {
    const sel = selectionRef.current;
    if (sel?.kind !== "device") return;
    const requestId = ++languageRequestIdRef.current;
    (async () => {
      try {
        const detail = await api.deviceDetail(sel.id, productLanguage);
        if (
          requestId === languageRequestIdRef.current &&
          selectionRef.current?.kind === "device" &&
          selectionRef.current.id === sel.id
        ) {
          setDeviceDetail(detail);
        }
      } catch (e) {
        if (
          requestId === languageRequestIdRef.current &&
          selectionRef.current?.kind === "device" &&
          selectionRef.current.id === sel.id
        ) {
          reportError(e);
        }
      }
    })();
  }, [productLanguage]);

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
      <button onClick={pickProject}>{t("toolbar.openProject")}</button>
      <button onClick={openNativeProject}>{t("toolbar.openNativeProject")}</button>
      <button onClick={saveProject} disabled={!tree}>
        {t("toolbar.save")}
      </button>
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
      <button onClick={undo} disabled={!tree?.can_undo}>
        {t("toolbar.undo")}
      </button>
      <button onClick={redo} disabled={!tree?.can_redo}>
        {t("toolbar.redo")}
      </button>
      <button onClick={() => tree && setSearchOpen(true)} disabled={!tree}>
        {t("toolbar.search")}
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
        {t("toolbar.log")}
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
        {t("toolbar.busMonitor")}
      </button>
      <button
        onClick={() => {
          setSearchOpen(false);
          setPaletteOpen(true);
        }}
      >
        {t("toolbar.commands")}
      </button>
      {/* Theme, motion style and motion level all live behind this one
          gear button (design D32) instead of a toolbar that grows a new
          bare `<select>` per setting — see SettingsPanel.tsx. */}
      <button onClick={() => setSettingsOpen(true)} title={t("toolbar.settings")} aria-label={t("toolbar.settings")}>
        <GearIcon />
      </button>
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
            <BusMonitorPanel projectOpen={tree !== null} />
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
      {settingsOpen && (
        <SettingsPanel
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
