/** Root component wiring project state, panels, and toolbars into the KNX Web UI shell. */
import { useEffect, useRef, useState, type MouseEvent as ReactMouseEvent } from "react";
import { version as packageVersion } from "../package.json";
import { isTauri, pickOpenPath, pickSavePath } from "./filePicker";
import * as api from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { DeviceDetail } from "./bindings/DeviceDetail";
import type { Selection } from "./selection";
import ProjectExplorer from "./ProjectExplorer";
import { useCrtInteractions } from "./useCrtInteractions";
import { requestCrtActivation } from "./crtInteractions";
import BulkActionToolbar from "./BulkActionToolbar";
import { useMultiSelection } from "./multiSelection";
import ResizablePane from "./ResizablePane";
import PaneSplitter from "./PaneSplitter";
import WorkbenchIcon from "./WorkbenchIcon";
import StructureWorkspace, { type StructureView } from "./StructureWorkspace";
import CatalogBrowser from "./CatalogBrowser";
import NewProjectDialog from "./NewProjectDialog";
import ProjectPasswordDialog from "./ProjectPasswordDialog";
import { projectPasswordRefusal, type ProjectPasswordRefusal } from "./projectPassword";
import Inspector, { DeviceWorkspace } from "./Inspector";
import Search from "./Search";
import CommandPalette from "./CommandPalette";
import type { CommandContext } from "./commandRegistry";
import SettingsPanel from "./SettingsPanel";
import Dashboard from "./Dashboard";
import LogPanel from "./LogPanel";
import BusDiagnosticsPanel from "./BusDiagnosticsPanel";
import { publishProjectContext, rebasePublishedSessionContext } from "./busContext";
import { ensureBusDiscovery } from "./busDiscovery";
import { openCompanionWindow } from "./diagnosticsWindow";
import { useAppearance } from "./appearance";
import { getThemeDefinitions, useThemeId, type ThemePreview } from "./theme";
import { MOTION_LEVELS, MOTION_STYLES, useMotion } from "./motion";
import { useProductLanguage } from "./productLanguage";
import type { ProductLanguage } from "./api";
import { useTranslate } from "./i18n";
import ToastStack from "./Toast";
import { pickStartupToast, useToasts } from "./toast";
import GroupAddressCsvButtons from "./GroupAddressCsvButtons";
import DocumentationExportButton from "./DocumentationExportButton";
import DebugReportButton from "./DebugReportButton";
import ProjectDiffPanel from "./ProjectDiffPanel";
import LoadProgressBanner from "./LoadProgressBanner";
import { localFailure, ownsOperation } from "./loadProgress";
import HelpPanel from "./HelpPanel";
import AboutDialog from "./AboutDialog";
import Overlay from "./Overlay";
import { canQuit, onWindowCloseRequested, quitApp } from "./quit";
import type { SessionControls } from "./session";
import { DEFAULT_HELP_TOPIC_ID, HELP_TOPICS, focusedHelpTopic, opensHelp, requestHelpTopic } from "./help";
import type { HelpTopicId } from "./help";
import { useAutosaveSettings } from "./autosaveSettings";
import { getSetting, setSetting, useSettingsRevision } from "./settingsStore";
import { useAutosave } from "./useAutosave";
import { useUiLanguage } from "./uiLanguage";

// How often the browser asks the server what a running load is doing
// (ADR-0023). Fast enough that a phase lasting a second is still seen,
// slow enough that a poll costs nothing next to the import it is watching.
const LOAD_POLL_INTERVAL_MS = 250;

// F3. The left column's two fixed blocks may be dragged between roughly
// two rows and most of a tall column; the project explorer between them
// takes whatever is left, which is the point of the exercise. `MIN` is
// "one row plus its padding, still recognisably a list"; `MAX` exists so
// a slip of the hand cannot hide the explorer entirely.
const STACK_BLOCK_MIN_PX = 72;
const STACK_BLOCK_MAX_PX = 480;

const NAVIGATION_PANE = { defaultWidth: 250, min: 200, max: 480 } as const;
const INSPECTOR_PANE = { defaultWidth: 360, min: 280, max: 700 } as const;
const UI_SCALE = { defaultValue: 1, min: 0.8, max: 1.5, step: 0.1 } as const;
// Match styles.css's inspector stack breakpoint and center minimum: the
// browser's media queries do not account for root CSS zoom.
const INSPECTOR_STACK_BREAKPOINT_PX = 1100;
const WORKBENCH_CENTER_MIN_PX = 200;

function boundedPreference(value: unknown, fallback: number, min: number, max: number): number {
  return typeof value === "number" && Number.isFinite(value)
    ? Math.max(min, Math.min(max, value))
    : fallback;
}

// The banner names the file, never the path the user picked it from —
// same rule the server's snapshot follows, for the same reason.
function fileNameOf(path: string): string {
  const name = path.split(/[\\/]/).pop();
  return name && name.length > 0 ? name : path;
}

// ISSUE-04: the user's selected UI language, not an unrelated browser locale.
function formatLastSaved(iso: string, language: string): string {
  const parsed = new Date(iso);
  if (Number.isNaN(parsed.getTime())) return iso;
  return new Intl.DateTimeFormat(language, {
    dateStyle: "short",
    timeStyle: "medium",
  }).format(parsed);
}

type AppProps = {
  manifestVersion?: string;
  /**
   * What `AuthGate` knows about the session this shell is running inside
   * (ADR-0026). Absent in the tests that render `App` directly, and
   * `required: false` on the desktop shell — both mean the same thing here:
   * no session exists, so no way to end one is offered.
   */
  session?: SessionControls;
};

interface SnapshotLifetime {
  initialized: boolean;
  serverIncarnation: string | null;
  revision: number;
  retiredServerIncarnations: Set<string>;
}

function acceptedSnapshotLifetime(
  current: SnapshotLifetime,
  tree: ProjectTree,
): SnapshotLifetime | null {
  const revision = tree.snapshot_revision ?? 0;
  const serverIncarnation = tree.server_incarnation ?? null;
  if (!current.initialized) {
    return {
      initialized: true,
      serverIncarnation,
      revision,
      retiredServerIncarnations: new Set(),
    };
  }
  if (serverIncarnation === null) {
    if (current.serverIncarnation !== null || revision < current.revision) return null;
    return { ...current, revision };
  }
  if (serverIncarnation === current.serverIncarnation) {
    return revision < current.revision ? null : { ...current, revision };
  }
  if (current.retiredServerIncarnations.has(serverIncarnation)) return null;
  const retiredServerIncarnations = new Set(current.retiredServerIncarnations);
  if (current.serverIncarnation !== null) {
    retiredServerIncarnations.add(current.serverIncarnation);
  }
  return { initialized: true, serverIncarnation, revision, retiredServerIncarnations };
}

function App({ manifestVersion = packageVersion, session }: AppProps) {
  // Called unconditionally on every render (not just from `SettingsPanel`,
  // which only mounts once Settings is opened) so `useUiLanguage()`'s own
  // effect — setting `document.documentElement.lang` — runs for the whole
  // session, not only for whoever happens to open Settings first. See
  // `App.test.tsx`'s "lang attribute is correct on a fresh mount" test.
  const t = useTranslate();
  const [uiLanguage] = useUiLanguage();
  useEffect(() => {
    document.title = `KNXBench ${manifestVersion}`;
  }, [manifestVersion]);
  // Rebuilt every render instead of hoisted to module scope: a module-level
  // `const` would call `t()` exactly once at import time and freeze the
  // filter name in whatever language happened to be active then — the same
  // trap `commandRegistry.ts`'s `COMMANDS` had before task 3's fix.
  // Open only: KNXBench reads `.knxproj` and never writes one (ADR-0028).
  const etsProjectFilter = [{ name: t("app.filterName.etsProject"), extensions: ["knxproj"] }];
  const knxdbFilter = [{ name: t("app.filterName.knxDesktopProject"), extensions: ["knxdb"] }];
  const [tree, setTree] = useState<ProjectTree | null>(null);
  // AR08: which import is waiting for its project password, and why.
  const [passwordPrompt, setPasswordPrompt] = useState<{ path: string; reason: ProjectPasswordRefusal } | null>(null);
  // Modern responses are ordered within an opaque server lifetime. The set
  // of retired lifetimes prevents a delayed reply from switching the UI back
  // after a restarted server has been accepted. Legacy responses remain in
  // one revision-only lifetime and cannot replace a modern one.
  const snapshotLifetimeRef = useRef<SnapshotLifetime>({
    initialized: false,
    serverIncarnation: null,
    revision: 0,
    retiredServerIncarnations: new Set(),
  });
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
  // Whether the backend's atomically published `AppState.store_path` is set,
  // mirrored only so "Save" knows whether it can skip the dialog. Accepted
  // server snapshots update this flag with their owning project; stale or
  // retired responses cannot change either one.
  const [hasStorePath, setHasStorePath] = useState(false);
  // The one project load that can be in flight, ADR-0023's whole client
  // side. `loadSource` doubles as the banner's visibility: it is set the
  // moment a file is picked and cleared only by a load that succeeded, so
  // a failure leaves the banner up saying which phase it died in.
  const [loadSource, setLoadSource] = useState<string | null>(null);
  const [loadSnapshot, setLoadSnapshot] = useState<api.LoadProgressSnapshot | null>(null);
  const [loading, setLoading] = useState(false);
  // The duplicate guard is a ref rather than `loading`: two clicks in the
  // same tick would both read the same stale state value, and the second
  // one would reach the server for a `409` it never needed to earn.
  const loadingRef = useRef(false);
  // ADR-0023's client half of ownership (fix round 3, F9): the opaque
  // token this load generates for itself before its POST, so `ownsOperation`
  // has an exact fact to compare against instead of an id-and-source
  // heuristic. A ref, not state: the poll must read the current value, not
  // the one captured when its effect was created.
  const loadClientTokenRef = useRef<string>("");
  // Which load is current, bumped the instant one ends (fix round 6,
  // F-C). Ownership answers "whose operation is this?"; this answers the
  // other half, "is that load still the one on screen?". The poll
  // interval is still armed while `runLoad`'s catch awaits its final
  // snapshot, and the effect's `cancelled` latch closes later still —
  // it is set by React's cleanup, not synchronously by `finally` — so a
  // poll resolving in between used to write a `running` snapshot over a
  // failure that was already on screen, and polling then stopped with
  // the banner frozen on a phase that was over. Captured before the
  // await, compared after: a generation that moved means the answer is
  // about a load nobody is watching any more.
  const loadGenerationRef = useRef(0);
  // Purely decorative, and kept well away from `loadGenerationRef` above:
  // this one only makes the banner a new element per load, so its flavour
  // line reshuffles instead of carrying the previous load's order and
  // position into the next one. Nothing reads it but React's `key`.
  const [loadKey, setLoadKey] = useState(0);
  const [selection, setSelection] = useState<Selection | null>(null);
  const [revealRequest, setRevealRequest] = useState<{
    selection: Selection;
    generation: number;
  } | null>(null);
  const revealGenerationRef = useRef(0);
  const [deviceDetail, setDeviceDetail] = useState<DeviceDetail | null>(null);
  const [searchOpen, setSearchOpen] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [logOpen, setLogOpen] = useState(false);
  const [monitorOpen, setMonitorOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [helpOpen, setHelpOpen] = useState(false);
  const [helpTopicId, setHelpTopicId] = useState<HelpTopicId>(DEFAULT_HELP_TOPIC_ID);
  const [aboutOpen, setAboutOpen] = useState(false);
  // F4. Only ever true inside the Tauri shell, and only with edits the
  // command stack can still undo — see `quitRequested` below for why that
  // is the dirty signal.
  const [quitConfirmOpen, setQuitConfirmOpen] = useState(false);
  const [quitSaving, setQuitSaving] = useState(false);
  // Which Save-and-quit attempt, if any, the user still wants. Every way
  // out of the quit dialog (Cancel, Escape, backdrop) bumps it, so a save
  // that resolves after the user changed their mind saves and nothing
  // more — it never quits on an answer the user has already withdrawn.
  const quitAttemptRef = useRef(0);
  // F1. The native `<details>` the File menu is. React does not own its
  // `open` attribute (nothing here re-renders when the user clicks the
  // summary), so closing it means writing that attribute, exactly as the
  // `Escape` handler on the element already does.
  const fileMenuRef = useRef<HTMLDetailsElement | null>(null);
  // F3. `null` means "whatever the content needs", which is the height
  // these two blocks have always had; a number means the user has moved
  // the separator. `PaneSplitter` measures the element to report an honest
  // `aria-valuenow` while the value is still `null`.
  const [navHeight, setNavHeight] = useState<number | null>(null);
  const [diagnosticsHeight, setDiagnosticsHeight] = useState<number | null>(null);
  const navBlockRef = useRef<HTMLElement | null>(null);
  const diagnosticsBlockRef = useRef<HTMLElement | null>(null);
  const [view, setView] = useState<"overview" | "catalog" | StructureView>("overview");
  const [buildingScope, setBuildingScope] = useState<number | null>(null);
  // The group-address view's counterpart of `buildingScope`: which range
  // the address table is scoped to. Owned here, not inside the table, for
  // the same reason `buildingScope` is: selecting a range in the tree has
  // to move the workspace, and two copies of "the current scope" would be
  // two things that agree only by luck.
  const [addressScope, setAddressScope] = useState<number | null>(null);
  const [navigationOpen, setNavigationOpen] = useState(true);
  const workbenchRef = useRef<HTMLElement>(null);
  const saveButtonRef = useRef<HTMLButtonElement>(null);
  useCrtInteractions(workbenchRef);
  const [inspectorOpen, setInspectorOpen] = useState(true);
  const [viewportWidth, setViewportWidth] = useState(() => window.innerWidth);
  useEffect(() => {
    const onResize = () => setViewportWidth(window.innerWidth);
    window.addEventListener("resize", onResize);
    return () => window.removeEventListener("resize", onResize);
  }, []);
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
  const [themePreview, setThemePreview] = useState<ThemePreview>();
  const [themeId, setThemeId] = useThemeId(themePreview);
  const appearance = useAppearance();  const { level: motionLevel, setLevel: setMotionLevel, style: motionStyle, setStyle: setMotionStyle } = useMotion();
  const [productLanguage, setProductLanguage] = useProductLanguage();
  useSettingsRevision();
  const uiScale = boundedPreference(getSetting("uiScale"), UI_SCALE.defaultValue, UI_SCALE.min, UI_SCALE.max);
  const navigationWidth = boundedPreference(getSetting("navigationPaneWidth"), NAVIGATION_PANE.defaultWidth, NAVIGATION_PANE.min, NAVIGATION_PANE.max);
  const inspectorWidth = boundedPreference(getSetting("inspectorPaneWidth"), INSPECTOR_PANE.defaultWidth, INSPECTOR_PANE.min, INSPECTOR_PANE.max);
  const inspectorVisible = inspectorOpen && !logOpen && !monitorOpen;
  const stackInspector = inspectorVisible && viewportWidth > INSPECTOR_STACK_BREAKPOINT_PX
    && (navigationOpen ? navigationWidth : 0) + inspectorWidth + WORKBENCH_CENTER_MIN_PX > viewportWidth / uiScale;
  useEffect(() => {
    document.documentElement.style.setProperty("--app-ui-scale", String(uiScale));
    return () => { document.documentElement.style.removeProperty("--app-ui-scale"); };
  }, [uiScale]);
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
    function openTopic(event: Event) {
      const requested = (event as CustomEvent<{ topicId?: string }>).detail?.topicId;
      const topicId = HELP_TOPICS.find((topic) => topic.id === requested)?.id ?? DEFAULT_HELP_TOPIC_ID;
      if (document.querySelector(".fs-picker")) return;
      setSearchOpen(false);
      setPaletteOpen(false);
      setSettingsOpen(false);
      setNewProjectOpen(false);
      setCatalogTarget(null);
      setView((current) => current === "catalog" ? "overview" : current);
      setHelpTopicId(topicId);
      setHelpOpen(true);
    }
    window.addEventListener("knxbench:open-help", openTopic);
    return () => window.removeEventListener("knxbench:open-help", openTopic);
  }, []);

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      const zoomDirection = ["+", "=", "Add"].includes(e.key) ? 1
        : ["-", "_", "Subtract"].includes(e.key) ? -1 : e.key === "0" ? 0 : null;
      if ((e.ctrlKey || e.metaKey) && !e.altKey && zoomDirection !== null) {
        const active = e.target instanceof HTMLElement ? e.target : document.activeElement;
        if (active instanceof HTMLElement && active.closest('input, textarea, select, [contenteditable="true"], [contenteditable=""]')) return;
        e.preventDefault();
        const current = boundedPreference(getSetting("uiScale"), UI_SCALE.defaultValue, UI_SCALE.min, UI_SCALE.max);
        const next = zoomDirection === 0 ? UI_SCALE.defaultValue
          : Math.round((current + zoomDirection * UI_SCALE.step) * 10) / 10;
        setSetting("uiScale", boundedPreference(next, UI_SCALE.defaultValue, UI_SCALE.min, UI_SCALE.max));
        return;
      }
      if (opensHelp(e)) {
        // F1 is the browser's help key as well as ours, so it has to be
        // taken before anything else looks at it.
        e.preventDefault();
        // Replace other overlays through the shared topic-opening path.
        // The native file picker owns a separate React root and cannot be
        // replaced from here.
        if (document.querySelector(".fs-picker")) return;
        requestHelpTopic(focusedHelpTopic(document.activeElement));
        return;
      }
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

  // F1, second half. A native `<details>` closes on `Escape` and on a
  // second click of its summary, and on nothing else — clicking the
  // workbench behind an open File menu leaves it hanging over the
  // workspace. This is deliberately `pointerdown` rather than `click`: the
  // menu should be gone by the time whatever was clicked reacts, and a
  // drag that starts outside the menu counts as leaving it.
  //
  // Unlike the F1 handler above, this one needs no `.fs-picker` carve-out.
  // The picker mounts outside the menu, so a pointer landing in it closes
  // the menu — which is exactly right, and closes nothing the picker owns.
  useEffect(() => {
    function handlePointerDown(e: PointerEvent) {
      const menu = fileMenuRef.current;
      if (!menu?.open) return;
      if (e.target instanceof Node && menu.contains(e.target)) return;
      menu.open = false;
    }
    document.addEventListener("pointerdown", handlePointerDown);
    return () => document.removeEventListener("pointerdown", handlePointerDown);
  }, []);

  const startupToastShown = useRef(false);
  useEffect(() => {
    if (startupToastShown.current) return; // StrictMode double-invoke guard
    startupToastShown.current = true;
    const message = pickStartupToast(new Date());
    if (message) pushFun(message);
  }, []);

  // T25 — look for KNX-compatible IP interfaces once, at application
  // start. Deliberately not awaited and deliberately without any UI of its
  // own here: it must not hold up first paint, must not delay a project
  // from loading, and must not put anything in front of a user whose
  // network has no KNX installation on it, which is the common case on a
  // laptop. Whatever it finds is an offer the bus monitor's gateway field
  // makes later; nothing connects on its own. `ensureBusDiscovery` is
  // idempotent, so StrictMode's double invoke needs no guard of its own
  // here — unlike the toast above, which would otherwise fire twice.
  useEffect(() => {
    ensureBusDiscovery();
  }, []);

  // Opens (or focuses) the read-only diagnostic companion. Every outcome is
  // reported: a blocked popup and a refused webview are ordinary results on
  // the platforms this ships to, and codex-goal.md is explicit that the
  // monitor must stay fully usable in this window when no second one is
  // available — which it does, because this button adds a window and moves
  // nothing out of here.
  async function openCompanion() {
    const result = await openCompanionWindow(window.location.href);
    if (result === "blocked") pushError(t("companion.blocked"), { serverText: false });
    else if (result === "failed") pushError(t("companion.failed"), { serverText: false });
  }

  function publishTree(newTree: ProjectTree): boolean {
    const nextLifetime = acceptedSnapshotLifetime(snapshotLifetimeRef.current, newTree);
    if (nextLifetime === null) return false;
    // Publish before adopting the tree so another window's already accepted
    // lifetime can veto a delayed local reply. The same record must exist
    // before a confirmed style response can rebase its exact bus session.
    if (!publishProjectContext(newTree)) return false;
    snapshotLifetimeRef.current = nextLifetime;
    rebasePublishedSessionContext(newTree);
    setTree(newTree);
    return true;
  }

  function resetTree(newTree: ProjectTree): boolean {
    if (!publishTree(newTree)) return false;
    setBuildingScope(null);
    setAddressScope(null);
    // Ids from the previous project mean nothing in this one, and a stale
    // bulk selection would offer to delete whatever happens to share those
    // ids now.
    clearMultiSelection();
    selectionRef.current = null;
    setSelection(null);
    setDeviceDetail(null);
    return true;
  }

  async function selectEntity(sel: Selection) {
    selectionRef.current = sel;
    setSelection(sel);
    // Never leave the previous device's editable fields under a new selection
    // while its asynchronous detail request is pending or fails.
    setDeviceDetail(null);
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

  function openCatalog(lineId: number | null) {
    setCatalogTarget({ lineId });
    setView("catalog");
    setLogOpen(false);
    setMonitorOpen(false);
    // On a stacked viewport the navigation occupies the first screenful.
    // Collapse it after its own catalog action so the workspace is visible;
    // the toolbar's Navigation control still reopens it on demand.
    if (window.innerWidth <= 650) setNavigationOpen(false);
  }

  function selectSearchResult(sel: Selection): void {
    setRevealRequest({ selection: sel, generation: ++revealGenerationRef.current });
    void selectEntity(sel);
  }

  function completeSearchReveal(generation: number): void {
    setRevealRequest((request) => request?.generation === generation ? null : request);
  }

  function toggleNavigation(): void {
    if (navigationOpen) setRevealRequest(null);
    setNavigationOpen((open) => !open);
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
    if (!publishTree(newTree)) return;
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
    if (!resetTree(newTree)) return;
    setCatalogTarget(null); // Never carry a former project's target line into this one.
    setHasStorePath(false);
    // The banner outlives a failed load on purpose, but only until that
    // load stops being the last thing that happened (fix round 6, F-D):
    // "Could not load villa.knxproj" pinned above a project started from
    // scratch describes nothing on screen.
    setLoadSource(null);
    setLoadSnapshot(null);
    setNewProjectOpen(false);
    setView("overview");
    setLogOpen(false);
    setMonitorOpen(false);
  }

  // Both ways a project enters the application, in one place: the same
  // duplicate guard, the same banner, the same polling. `storePath` is the
  // only thing that differs — an ETS import has no `.knxdb` location yet.
  function finishLoadedProject(loadedTree: ProjectTree, path: string | null, storePath: boolean) {
    if (!resetTree(loadedTree)) return;
    setCatalogTarget(null); // Product search/selection is scoped to this project lifetime.
    setHasStorePath(storePath);
    setLoadSource(null);
    setLoadSnapshot(null);
    pushFun(path === null ? t("loadProgress.recovered") : t("loadProgress.succeeded", { source: fileNameOf(path) }));
  }

  async function runLoad(
    path: string,
    load: (p: string, clientToken: string) => Promise<ProjectTree>,
    storePath: boolean,
    kind: "import" | "open" = storePath ? "open" : "import",
  ) {
    if (loadingRef.current) return;
    loadingRef.current = true;
    clearErrors();
    setLoadSource(fileNameOf(path));
    setLoadSnapshot(null);
    setLoadKey((k) => k + 1);
    // The one fact `ownsOperation` needs: an id nobody else could send,
    // generated before the POST so every snapshot from here on — the
    // poll's and the post-failure fetch's alike — can be judged against
    // it (ADR-0023 fix round 3, F9).
    loadClientTokenRef.current = crypto.randomUUID();
    setLoading(true);
    try {
      finishLoadedProject(await load(path, loadClientTokenRef.current), path, storePath);
    } catch (e) {
      // The transport rejection is not necessarily a load failure: the
      // server may have committed our operation before its response was
      // lost. Only an exact-token succeeded snapshot earns a read of the
      // current server tree. Foreign, missing and failed snapshots retain
      // the ordinary local-failure path, and a failed recovery reports the
      // recovery error instead of the superseded transport error.
      let failure = e;
      const final = await api.loadProgress().catch(() => null);
      if (final?.status === "succeeded"
        && ownsOperation({ clientToken: loadClientTokenRef.current }, final)) {
        try {
          const current = await api.currentProject();
          finishLoadedProject(current, null, current.has_store_path);
          return;
        } catch (recoveryError) {
          failure = recoveryError;
        }
      }
      // AR08: a protected ETS project is not a failure to report but a
      // question to ask; the server wrote nothing, so nothing is pinned.
      const passwordRefusal = kind === "import" ? projectPasswordRefusal(failure) : null;
      if (passwordRefusal) {
        setLoadSource(null);
        setLoadSnapshot(null);
        setPasswordPrompt({ path, reason: passwordRefusal });
        return;
      }
      reportError(failure);
      const message = api.errorMessage(failure);
      const ours = final?.status === "failed" && ownsOperation({ clientToken: loadClientTokenRef.current }, final);
      setLoadSnapshot((previous) => (ours && final ? final : localFailure(previous, message)));
    } finally {
      // Synchronously, before anything React does: from here on every
      // poll still in flight belongs to a load that is over (F-C).
      loadGenerationRef.current += 1;
      setLoading(false);
      loadingRef.current = false;
    }
  }

  // A poll that fails is not a load that failed — the POST is the only
  // thing that decides that — so a rejected snapshot fetch is swallowed
  // rather than turned into an error the user cannot act on. Two filters
  // must both pass: `running`, because anything else belongs to an
  // operation that is already over, and ownership, because a `running`
  // operation can still be somebody else's — the `409` case, where our
  // POST is refused precisely *because* another operation is in flight.
  useEffect(() => {
    if (!loading) return;
    let cancelled = false;
    async function poll() {
      // The generation this poll was fired for. `cancelled` alone cannot
      // carry this: it is closed by the cleanup below, which React runs
      // only once it commits `loading: false` — a whole microtask queue
      // after `runLoad` has finished writing its failure (F-C).
      const generation = loadGenerationRef.current;
      try {
        const snapshot = await api.loadProgress();
        if (cancelled || generation !== loadGenerationRef.current) return;
        if (snapshot?.status !== "running") return;
        if (!ownsOperation({ clientToken: loadClientTokenRef.current }, snapshot)) return;
        setLoadSnapshot(snapshot);
      } catch {
        /* see above */
      }
    }
    void poll(); // first poll immediately, not after the first interval tick
    const id = setInterval(() => void poll(), LOAD_POLL_INTERVAL_MS);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, [loading]);

  async function pickProject() {
    if (loadingRef.current) return;
    const path = await pickOpenPath(etsProjectFilter);
    if (!path) return;
    await runLoad(path, api.importProject, false);
  }

  async function openNativeProject() {
    if (loadingRef.current) return;
    const path = await pickOpenPath(knxdbFilter);
    if (!path) return;
    await runLoad(path, api.openProject, true);
  }

  // Resolves `true` only when the refreshed snapshot was accepted *and*
  // the server reports it clean. That is the one answer Save-and-quit and
  // Save-and-create may act on: a save that worked but raced a newer edit
  // is not "nothing left to lose".
  async function refreshSavedProject(): Promise<boolean> {
    const current = await api.currentProject();
    if (!publishTree(current)) return false;
    setHasStorePath(current.has_store_path);
    return !current.is_modified;
  }

  // Both manual saves still swallow failures into the error toast, but
  // they now also *answer*: `true` means the project is on disk and clean,
  // anything else (picker cancelled, snapshot moved on, server refused)
  // is `false`. The Save-and-* buttons below depend on that answer.
  async function saveProjectAs(): Promise<boolean> {
    autosave.cancelCountdown();
    const projectSnapshot = {
      serverIncarnation: snapshotLifetimeRef.current.serverIncarnation,
      revision: snapshotLifetimeRef.current.revision,
    };
    const path = await pickSavePath(knxdbFilter, "project.knxdb");
    const latestSnapshot = snapshotLifetimeRef.current;
    if (
      !path ||
      projectSnapshot.serverIncarnation !== latestSnapshot.serverIncarnation ||
      projectSnapshot.revision !== latestSnapshot.revision
    ) return false;
    clearErrors();
    try {
      requestCrtActivation(saveButtonRef.current);
      await api.saveProjectAs(path);
      return await refreshSavedProject();
    } catch (e) {
      reportError(e);
      return false;
    }
  }

  async function saveProject(): Promise<boolean> {
    if (!hasStorePath) return saveProjectAs();
    autosave.cancelCountdown();
    clearErrors();
    try {
      requestCrtActivation(saveButtonRef.current);
      await api.saveProject();
      return await refreshSavedProject();
    } catch (e) {
      reportError(e);
      return false;
    }
  }

  // ISSUE-04's autosave engine calls this, never `saveProject()` above —
  // same underlying `api.saveProject()` call, but this one rethrows so
  // `useAutosave`'s own failure handling (skip clearing dirty state,
  // reschedule without wedging) can tell a failed autosave from a
  // successful one, and it never opens a Save-As picker nobody asked
  // for. This is not a second persistence path, only a second caller of
  // the one save the button already uses.
  async function autosaveProject() {
    if (!hasStorePath) return;
    clearErrors();
    await api.saveProject();
    await refreshSavedProject();
  }

  function exportProject() {
    const anchor = document.createElement("a");
    anchor.href = "/api/project/download";
    anchor.download = "project.knxdb";
    anchor.click();
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

  // F1, first half. One handler on the menu's container rather than an
  // `onClick` per item: three of the items are child components whose
  // buttons this file does not own (`GroupAddressCsvButtons`,
  // `DocumentationExportButton`, `ProjectDiffPanel`), so a per-button
  // approach would be incomplete the day one of them grows a fourth
  // button. Keyboard activation needs no separate path — `Enter` and
  // `Space` on a `<button>` dispatch a real `click` that bubbles here just
  // like a mouse one.
  //
  // The one opt-out is `data-menu-stays-open`, and exactly one item wears
  // it: `ProjectDiffPanel`'s Compare button renders its report *inside*
  // this menu, so closing the menu on that click would hide the very
  // thing the click asked for.
  function handleFileMenuActivation(e: ReactMouseEvent<HTMLDivElement>) {
    if (!(e.target instanceof Element)) return;
    const activated = e.target.closest("button");
    if (!activated || !e.currentTarget.contains(activated)) return;
    if (activated.closest("[data-menu-stays-open]")) return;
    const menu = fileMenuRef.current;
    if (menu) menu.open = false;
  }

  // F4. The server owns the clean project snapshot. Undo availability stays
  // a toolbar concern; only the server's normalized content comparison may
  // decide whether closing would discard user-visible changes.
  async function saveAndQuit() {
    if (quitSaving) return;
    const attempt = ++quitAttemptRef.current;
    setQuitSaving(true);
    try {
      const saved = await saveProject();
      if (saved && quitAttemptRef.current === attempt) await quitApp();
    } finally {
      setQuitSaving(false);
    }
  }

  function dismissQuitConfirm() {
    quitAttemptRef.current += 1;
    setQuitConfirmOpen(false);
  }

  function quitRequested() {
    if (tree?.is_modified) {
      setQuitConfirmOpen(true);
      return;
    }
    void quitApp();
  }

  // §132. The window manager's close (×, Alt+F4) takes the same decision as
  // File › Quit. The listener is registered once, so it reads
  // `is_modified` through a ref rather than a stale closure; a modified
  // project keeps the window open and shows the same quit-confirm dialog.
  const isModifiedRef = useRef(false);
  isModifiedRef.current = tree?.is_modified ?? false;
  useEffect(() => {
    if (!canQuit()) return;
    let unlisten: (() => void) | null = null;
    let disposed = false;
    void onWindowCloseRequested(() => {
      if (!isModifiedRef.current) return true;
      setQuitConfirmOpen(true);
      return false;
    }).then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  // ISSUE-04's autosave settings and engine. `hasStorePath`/`is_modified`
  // come straight from server-authoritative state (T13's owner, extended
  // above for `last_saved_at`) — this hook never invents a second dirty
  // signal, only reads the one that already exists.
  const autosaveSettings = useAutosaveSettings();
  const autosave = useAutosave({
    enabled: autosaveSettings.enabled,
    intervalMinutes: autosaveSettings.intervalMinutes,
    hasStorePath,
    isModified: tree?.is_modified ?? false,
    onSave: autosaveProject,
    onSaveFailed: () => pushError(t("autosave.failed"), { serverText: false }),
  });

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
    openHelp: () => requestHelpTopic(DEFAULT_HELP_TOPIC_ID),
  };

  // With no project, the welcome routes should precede the navigation in
  // DOM/Tab order. At wide widths the explorer still sits visually left;
  // at narrow widths the welcome stays above its tall diagnostics block.
  const welcomeVisible = !tree && !logOpen && !monitorOpen && view !== "catalog";
  const centerWorkspace = (
    <div className="workbench-center">
      {logOpen ? <LogPanel tree={tree} refreshKey={logVersion} /> : monitorOpen ? <BusDiagnosticsPanel project={tree} onTreeUpdate={handleTreeUpdate} /> : view === "catalog" ? null : tree ? (
        view === "overview" ? <Dashboard tree={tree} /> : <StructureWorkspace tree={tree} view={view} selection={selection} buildingScope={buildingScope} onBuildingScope={setBuildingScope}
          rangeScope={addressScope} onRangeScope={setAddressScope}
          multiSelection={multiSelection} onItemClick={onItemClick} onTreeUpdate={handleTreeUpdate} onDeleted={resetTree}
          addressActions={<GroupAddressCsvButtons tree={tree} onTreeUpdate={handleTreeUpdate} onSummary={pushFun} onError={reportError} onClearErrors={clearErrors} />}
          onSelect={selectEntity} onCatalog={openCatalog} />
      ) : (
        <section className="welcome-workspace" aria-labelledby="welcome-title">
          <h1 id="welcome-title">{t("workbench.welcome")}</h1>
          <p>{t("workbench.openHint")}</p>
          <div className="welcome-actions">
            <button type="button" className="welcome-card primary-action" onClick={startNewProject} aria-labelledby="welcome-new-title" aria-describedby="welcome-new-description">
              <span id="welcome-new-title" className="welcome-card-title">{t("toolbar.newProject")}</span>
              <span id="welcome-new-description" className="welcome-card-description">{t("workbench.newDescription")}</span>
            </button>
            <button type="button" className="welcome-card" onClick={openNativeProject} disabled={loading} aria-labelledby="welcome-native-title" aria-describedby="welcome-native-description">
              <span id="welcome-native-title" className="welcome-card-title">{t("workbench.openNativeTitle")}</span>
              <span id="welcome-native-description" className="welcome-card-description">{t("workbench.openNativeDescription")}</span>
            </button>
            <button type="button" className="welcome-card" onClick={pickProject} disabled={loading} aria-labelledby="welcome-ets-title" aria-describedby="welcome-ets-description">
              <span id="welcome-ets-title" className="welcome-card-title">{t("workbench.importEtsTitle")}</span>
              <span id="welcome-ets-description" className="welcome-card-description">{t("workbench.importEtsDescription")}</span>
            </button>
          </div>
        </section>
      )}
      {catalogTarget && <CatalogBrowser lineId={catalogTarget.lineId} active={!logOpen && !monitorOpen && view === "catalog"}
        serverIncarnation={tree?.server_incarnation} onCreated={handleTreeUpdate} onClose={() => { setCatalogTarget(null); setView("overview"); }} />}
      {tree && selection?.kind === "device" && deviceDetail?.id === selection.id && !logOpen && !monitorOpen && view !== "catalog" && <DeviceWorkspace key={deviceDetail.id} detail={deviceDetail} tree={tree} onApplied={handleTreeUpdate} />}
    </div>
  );

  return (
    <main ref={workbenchRef} className={`workbench${stackInspector ? " workbench--stacked-inspector" : ""}${welcomeVisible ? " workbench--welcome" : ""}`}>
      <header className="workbench-toolbar">
        <a className="workbench-brand" href="#" onClick={(e) => { e.preventDefault(); setView("overview"); setLogOpen(false); setMonitorOpen(false); }}><span className="brand-mark">K</span><strong>KNXBench</strong></a>
        <details ref={fileMenuRef} className="file-menu" onKeyDown={(e) => { if (e.key === "Escape") { e.currentTarget.open = false; e.currentTarget.querySelector("summary")?.focus(); } }}>
          <summary>{t("workbench.file")} <span aria-hidden="true">⌄</span></summary>
          <div className="file-menu-content" onClick={handleFileMenuActivation}>
      <button onClick={startNewProject}>{t("toolbar.newProject")}</button>
      <button onClick={pickProject} disabled={loading}>{t("toolbar.openProject")}</button>
      <button onClick={openNativeProject} disabled={loading}>{t("toolbar.openNativeProject")}</button>
      <button onClick={saveProjectAs} disabled={!tree}>
        {t("toolbar.saveAs")}
      </button>
      {!isTauri() && (
        <button onClick={exportProject} disabled={!tree}>{t("toolbar.exportProject")}</button>
      )}
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
      <DebugReportButton onSummary={pushFun} onError={reportError} onClearErrors={clearErrors} />
      <button onClick={() => setAboutOpen(true)}>{t("toolbar.about")}</button>
      {/* ADR-0026: only where a session exists to end. On the desktop shell
          — and on any server started without a password — `required` is
          false and a logout button would be an offer to leave a room with
          no door. */}
      {session?.required === true && (
        <button className="file-menu-logout" onClick={session.logout}>
          {t("toolbar.logout")}
        </button>
      )}
      {/* F4: present only in the desktop shell. A browser tab cannot close
          itself, so in the web build this item would be a button that
          does nothing — see `quit.ts`. */}
      {canQuit() && <button className="file-menu-quit" onClick={quitRequested}>{t("toolbar.quit")}</button>}

          </div>
        </details>
        <div className="history-actions">
          <button onClick={undo} disabled={!tree?.can_undo} title={t("toolbar.undo")} aria-label={t("toolbar.undo")}><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true"><path d="M8 4L3 9l5 5 M3 9h10a6 6 0 010 12" /></svg><span className="sr-only">{t("toolbar.undo")}</span></button>
          <button onClick={redo} disabled={!tree?.can_redo} title={t("toolbar.redo")} aria-label={t("toolbar.redo")}><svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6" aria-hidden="true"><path d="M16 4l5 5-5 5 M21 9h-10a6 6 0 000 12" /></svg><span className="sr-only">{t("toolbar.redo")}</span></button>
        </div>
        <button className="workbench-search" onClick={() => tree && setSearchOpen(true)} disabled={!tree}>{t("toolbar.search")}<kbd>Ctrl K</kbd></button>
        <button className="command-entry" onClick={() => setPaletteOpen(true)}>{t("toolbar.commands")}</button>
        <button ref={saveButtonRef} data-crt-surface="save" className="primary-action" onClick={saveProject} disabled={!tree}>{t("toolbar.save")}</button>
        <button onClick={() => requestHelpTopic(DEFAULT_HELP_TOPIC_ID)} title={t("toolbar.help")} aria-label={t("toolbar.help")}><QuestionIcon /></button>
        <button onClick={() => setSettingsOpen(true)} title={t("toolbar.settings")} aria-label={t("toolbar.settings")}><GearIcon /></button>
      </header>
      <div className="workbench-panel-controls">
        <button aria-expanded={navigationOpen} onClick={toggleNavigation}><WorkbenchIcon name="panel" />{t("workbench.navigation")}</button>
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
      {loadSource && <LoadProgressBanner key={loadKey} source={loadSource} snapshot={loadSnapshot} />}
      <div className="workspace workbench-body">
        {welcomeVisible && centerWorkspace}
        {navigationOpen && <ResizablePane label={t("workbench.navigation")} side="left" initialWidth={navigationWidth} min={NAVIGATION_PANE.min} max={NAVIGATION_PANE.max} onWidthCommit={(width) => setSetting("navigationPaneWidth", width)}>
          <nav ref={navBlockRef} className="workbench-navigation" aria-label={t("workbench.navigation")} style={{ height: navHeight ?? undefined }}>
            {(["overview", "buildings", "topology", "addresses"] as const).map((item) => <button key={item} aria-current={!logOpen && !monitorOpen && view === item ? "page" : undefined} onClick={() => { setView(item); setLogOpen(false); setMonitorOpen(false); }}><WorkbenchIcon name={item} />{t(`workbench.${item}`)}</button>)}
            <button aria-current={!logOpen && !monitorOpen && view === "catalog" ? "page" : undefined}
              onClick={() => openCatalog(selection?.kind === "line" ? selection.id : null)}><WorkbenchIcon name="catalog" />{t("workbench.catalog")}</button>
          </nav>
          {/* F3: the two horizontal separators exist only alongside the
              block they hand space to. Without a project there is no
              explorer between these three, so there is nothing to
              redistribute and no separator to offer. */}
          {tree && <PaneSplitter label={t("workbench.resizeNavigation")} target={navBlockRef} resizes="above" value={navHeight} onChange={setNavHeight} min={STACK_BLOCK_MIN_PX} max={STACK_BLOCK_MAX_PX} />}
          {tree && <ProjectExplorer tree={tree} selection={selection} onSelect={selectEntity} onTreeUpdate={handleTreeUpdate} multiSelection={multiSelection} onItemClick={onItemClick} onSummary={pushFun} onError={reportError} revealRequest={revealRequest} onRevealComplete={completeSearchReveal} />}
          {tree && <PaneSplitter label={t("workbench.resizeDiagnostics")} target={diagnosticsBlockRef} resizes="below" value={diagnosticsHeight} onChange={setDiagnosticsHeight} min={STACK_BLOCK_MIN_PX} max={STACK_BLOCK_MAX_PX} />}
          <nav ref={diagnosticsBlockRef} className="workbench-navigation diagnostic-navigation" aria-label={t("toolbar.busMonitor")} style={{ height: diagnosticsHeight ?? undefined }}>
            <button aria-current={monitorOpen ? "page" : undefined} onClick={() => { setLogOpen(false); setMonitorOpen((open) => !open); }}><WorkbenchIcon name="monitor" />{t("toolbar.busMonitor")}</button>
            <button aria-current={logOpen ? "page" : undefined} onClick={() => { setMonitorOpen(false); setLogOpen((open) => !open); }}><WorkbenchIcon name="log" />{t("toolbar.log")}</button>
            <button className="companion-open" onClick={() => void openCompanion()}><WorkbenchIcon name="panel" />{t("companion.open")}</button>
            <button onClick={() => setSettingsOpen(true)}><GearIcon />{t("toolbar.settings")}</button>
          </nav>
        </ResizablePane>}
        {!welcomeVisible && centerWorkspace}
        {inspectorOpen && !logOpen && !monitorOpen && <ResizablePane label={t("workbench.properties")} side="right" initialWidth={inspectorWidth} min={INSPECTOR_PANE.min} max={INSPECTOR_PANE.max} onWidthCommit={(width) => setSetting("inspectorPaneWidth", width)}>
          <header className="inspector-heading">{t("workbench.properties")}</header>
          {tree && selection ? <Inspector propertiesOnly key={`${selection.kind}-${selection.id}`} selection={selection} tree={tree} deviceDetail={deviceDetail} onApplied={handleTreeUpdate} onDeleted={resetTree} /> : <p className="inspector-empty">{t("workbench.noSelection")}</p>}
        </ResizablePane>}
      </div>
      <footer className="workbench-status">
        <span>{tree ? tree.installations.map((i) => i.name).join(" / ") : "KNXBench"}</span>
        {tree && (
          <span className="workbench-status-saved">
            {tree.last_saved_at
              ? t("statusBar.lastSaved", { time: formatLastSaved(tree.last_saved_at, uiLanguage) })
              : t("statusBar.neverSaved")}
          </span>
        )}
        <span>v{manifestVersion}</span>
      </footer>
      <ToastStack toasts={toasts} onDismiss={dismiss} />
      {autosave.secondsRemaining !== null && (
        <div className="autosave-countdown" role="status">
          <span>{t("autosave.countdown", { seconds: autosave.secondsRemaining })}</span>
          <button type="button" onClick={autosave.cancelCountdown}>
            {t("autosave.cancel")}
          </button>
        </div>
      )}

      {newProjectOpen && (
        <NewProjectDialog onCreated={newProjectCreated} onClose={() => setNewProjectOpen(false)} onSaveFirst={saveProject} />
      )}
      {passwordPrompt && (
        <ProjectPasswordDialog fileName={fileNameOf(passwordPrompt.path)} reason={passwordPrompt.reason}
          onCancel={() => setPasswordPrompt(null)}
          onSubmit={(password) => {
            const { path } = passwordPrompt;
            setPasswordPrompt(null);
            // The password lives only in this closure for the one retry.
            void runLoad(path, (p, clientToken) => api.importProject(p, clientToken, password), false, "import");
          }} />
      )}
      {tree && searchOpen && (
        <Search tree={tree} onSelect={selectSearchResult} onClose={() => setSearchOpen(false)} />
      )}
      {paletteOpen && <CommandPalette ctx={ctx} onClose={() => setPaletteOpen(false)} />}
      {helpOpen && <HelpPanel key={helpTopicId} initialTopicId={helpTopicId} onClose={() => setHelpOpen(false)} />}
      {aboutOpen && <AboutDialog onClose={() => setAboutOpen(false)} />}
      {quitConfirmOpen && (
        <Overlay labelledBy="quit-confirm-title" className="quit-confirm" onClose={dismissQuitConfirm}>
          <h2 id="quit-confirm-title">{t("quit.title")}</h2>
          <p>{t("quit.message")}</p>
          <p className="quit-confirm-hint">{t("quit.hint")}</p>
          {/* Cancel comes first so `Overlay`'s initial focus lands on it:
              the safe answer is the one already under the finger. "Save
              and quit" quits only on `saveProject()`'s `true` — saved *and*
              clean. A failed save (toast), a cancelled Save-As picker or a
              save that raced a newer edit keeps both this dialog and the
              project open, which is the whole point of asking. */}
          <footer className="quit-confirm-footer">
            {/* Cancel stays live during a save: it withdraws the quit, not
                the save, which finishes (or fails) and leaves the app open. */}
            <button type="button" onClick={dismissQuitConfirm}>{t("quit.cancel")}</button>
            <button type="button" className="quit-confirm-discard" disabled={quitSaving} onClick={() => void quitApp()}>{t("quit.discard")}</button>
            <button type="button" className="primary-action quit-confirm-save" disabled={quitSaving} onClick={() => void saveAndQuit()}>
              {quitSaving ? t("quit.saving") : t("quit.save")}
            </button>
          </footer>
        </Overlay>
      )}
      {settingsOpen && (
        <SettingsPanel
          appearance={appearance}
          themes={getThemeDefinitions()}
          activeThemeId={themeId}
          onSelectTheme={setThemeId}
          onPreviewTheme={setThemePreview}
          previewTheme={themePreview}
          motionStyles={MOTION_STYLES}
          activeMotionStyle={motionStyle}
          onSelectMotionStyle={setMotionStyle}
          motionLevels={MOTION_LEVELS}
          activeMotionLevel={motionLevel}
          onSelectMotionLevel={setMotionLevel}
          productLanguages={productLanguages}
          activeProductLanguage={productLanguage}
          onSelectProductLanguage={setProductLanguage}
          autosaveEnabled={autosaveSettings.enabled}
          onSelectAutosaveEnabled={autosaveSettings.setEnabled}
          autosaveIntervalMinutes={autosaveSettings.intervalMinutes}
          onSelectAutosaveIntervalMinutes={autosaveSettings.setIntervalMinutes}
          onClose={() => setSettingsOpen(false)}
        />
      )}
    </main>
  );
}

// Cycle 13's ThemeToggle drew its sun/moon/monitor icons the same way:
// hand-written inline SVG, no icon library. The question mark and the gear
// below it are that convention's two newest members.
function QuestionIcon() {
  return (
    <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false">
      <circle cx="8" cy="8" r="6.4" fill="none" stroke="currentColor" strokeWidth="1.4" />
      <path
        d="M6.2 6.1a1.9 1.9 0 1 1 2.5 1.8c-.5.2-.8.6-.8 1.1v.5"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.4"
        strokeLinecap="round"
      />
      <circle cx="8" cy="11.6" r="0.85" fill="currentColor" />
    </svg>
  );
}

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
      <path d="M10 2h4l.7 2.3 1.7.7 2.1-1.1 2.8 2.8-1.1 2.1.7 1.7L22 10v4l-2.3.7-.7 1.7 1.1 2.1-2.8 2.8-2.1-1.1-1.7.7L14 22h-4l-.7-2.3-1.7-.7-2.1 1.1-2.8-2.8 1.1-2.1-.7-1.7L2 14v-4l2.3-.7.7-1.7-1.1-2.1 2.8-2.8 2.1 1.1 1.7-.7L10 2Z" />
      <circle cx="12" cy="12" r="3" />
    </svg>
  );
}

export default App;
