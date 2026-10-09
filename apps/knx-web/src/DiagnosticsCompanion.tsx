/** Second-window shell that hosts only the bus monitor and the session log, never the editor. */
// apps/knx-web/src/DiagnosticsCompanion.tsx
//
// codex-goal.md §"Zweiter Bildschirm" asks for exactly one editing
// workspace. This component is how that is enforced rather than merely
// intended: it renders `BusMonitorPanel` and `LogPanel` and nothing else.
//
// Be precise about *why*, because the earlier version of this comment was
// not. It claimed that this file's import list was the guarantee, and that
// "anything reachable from this module is reachable from the companion
// window". Both halves were wrong. The implication runs the other way, and
// the transitive graph is not clean: this module imports
// `./diagnosticsWindow`, which imports `isTauri` from `./filePicker`,
// which imports `./FsPicker`, which `POST`s to `/api/fs/upload`. A
// depth-1 list proves nothing about depth 3.
//
// What is actually true, and what `DiagnosticsCompanion.test.tsx` asserts:
//
//  1. This file names no editing surface directly, so "just one small
//     button" cannot be added here without failing a test.
//  2. Across the whole transitive value-import graph — fifteen modules,
//     pinned by name — every `api` call is a bus or diagnostics call, and
//     the only two project-mutating raw `fetch`es (`/api/catalog/install`,
//     `/api/fs/upload`) sit inside functions this window never calls.
//     `filePicker` is in the graph for `isTauri`, a two-line `window`
//     predicate; its picker paths are what reach `FsPicker`.
//  3. Mounted and left alone, the companion calls exactly one `api`
//     export — the telegram poll. Opening the Log tab adds the log read,
//     and nothing else, ever. Asserted as the *set* of exports called, so
//     a new mutator fails the test rather than being forgotten.
//
// Not claimed: that a deliberate click cannot reach the bus. It can —
// Connect starts a session and the compose form writes a telegram. Neither
// touches the project, which is the property this window exists to keep.
//
// No undo, either. The Ctrl+Z / Ctrl+Shift+Z handler lives in `App.tsx`'s
// global `keydown` listener, which never mounts here; this window has no
// command stack of its own and no way to reach the server's.
//
// The one bus session is shared, not duplicated: `BusMonitorPanel` asks
// `GET /api/bus/monitor/telegrams` on mount and attaches to whatever
// session already exists (`bus_routes.rs:272-311` answers `404` when there
// is none). Closing this window runs no cleanup against that session —
// `BusMonitorPanel`'s unmount clears its polling interval and nothing else
// — so the bus connection outlives the companion, as codex-goal.md
// requires.

import { useEffect, useState } from "react";
import BusMonitorPanel from "./BusMonitorPanel";
import LogPanel from "./LogPanel";
import { projectContextKnown, subscribeContextChanges } from "./busContext";
import { canReturnToMainWindow, returnToMainWindow } from "./diagnosticsWindow";
import { useTranslate } from "./i18n";

type CompanionTab = "monitor" | "log";

export default function DiagnosticsCompanion() {
  const t = useTranslate();
  const [tab, setTab] = useState<CompanionTab>("monitor");
  // Whether any window has published an open project. The companion keeps
  // no tree of its own; this synchronous, cross-window record is the last
  // confirmed context publication and is used only to decide whether the
  // compose form explains that DPTs will not resolve automatically.
  const [projectOpen, setProjectOpen] = useState(projectContextKnown);
  const [canReturn, setCanReturn] = useState(false);

  useEffect(() => subscribeContextChanges(() => setProjectOpen(projectContextKnown())), []);

  // Asked once, asynchronously, because the Tauri answer is a promise. A
  // native return requires a main webview; browsers can always navigate
  // this tab back to the editor, including a standalone bookmark.
  useEffect(() => {
    let cancelled = false;
    void (async () => {
      const possible = await canReturnToMainWindow();
      if (!cancelled) setCanReturn(possible);
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  return (
    <main className="companion-shell">
      <header className="companion-heading">
        <div>
          <p className="eyebrow">{t("companion.eyebrow")}</p>
          <h1>{t("companion.title")}</h1>
        </div>
        <button
          className="companion-return"
          onClick={() => void returnToMainWindow()}
          disabled={!canReturn}
          title={canReturn ? undefined : t("companion.noMainWindow")}
        >
          {t("companion.backToMain")}
        </button>
      </header>
      {/* Stated, not implied. A window that looks like the application but
          silently refuses to edit is worse than one that says why. */}
      <p className="companion-read-only" role="note">
        {t("companion.readOnly")}
      </p>
      <nav className="companion-tabs" aria-label={t("companion.title")}>
        <button aria-current={tab === "monitor" ? "page" : undefined} onClick={() => setTab("monitor")}>
          {t("toolbar.busMonitor")}
        </button>
        <button aria-current={tab === "log" ? "page" : undefined} onClick={() => setTab("log")}>
          {t("toolbar.log")}
        </button>
      </nav>
      <div className="companion-body">
        {tab === "monitor" ? (
          <BusMonitorPanel projectOpen={projectOpen} />
        ) : (
          <LogPanel tree={null} refreshKey={0} />
        )}
      </div>
    </main>
  );
}
