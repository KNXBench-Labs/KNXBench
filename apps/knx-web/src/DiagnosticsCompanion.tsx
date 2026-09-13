/** Second-window shell that hosts only the bus monitor and the session log, never the editor. */
// apps/knx-web/src/DiagnosticsCompanion.tsx
//
// codex-goal.md §"Zweiter Bildschirm" asks for exactly one editing
// workspace. This component is how that is enforced rather than merely
// intended: it renders `BusMonitorPanel` and `LogPanel` and nothing else,
// and it imports nothing that could mutate a project. `App.tsx`, the
// explorer, the inspector, the catalogue, the command palette and every
// `api` mutation are absent from this module's import graph — a fact
// `DiagnosticsCompanion.test.tsx` asserts against the file's own source,
// because a future edit adding "just one small button" would otherwise
// break the guarantee silently.
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
import { canFocusMainWindow, focusMainWindow } from "./diagnosticsWindow";
import { useTranslate } from "./i18n";

type CompanionTab = "monitor" | "log";

export default function DiagnosticsCompanion() {
  const t = useTranslate();
  const [tab, setTab] = useState<CompanionTab>("monitor");
  // Whether any window has published an open project. The companion has no
  // tree and no route to ask for one (`busContext.ts`'s module comment), so
  // this is the honest available answer, and it is only used to decide
  // whether the compose form explains that DPTs will not resolve
  // automatically.
  const [projectOpen, setProjectOpen] = useState(projectContextKnown);
  const [canReturn, setCanReturn] = useState(false);

  useEffect(() => subscribeContextChanges(() => setProjectOpen(projectContextKnown())), []);

  // Asked once, asynchronously, because the Tauri answer is a promise. A
  // disabled button that explains itself beats a button that does nothing
  // when the companion was reopened from a bookmark and has no opener.
  useEffect(() => {
    let cancelled = false;
    void (async () => {
      const possible = await canFocusMainWindow();
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
          onClick={() => void focusMainWindow()}
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
