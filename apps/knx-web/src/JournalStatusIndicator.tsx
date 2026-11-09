/** Status-bar note while background journal writes run, or after one failed (ADR-0166). */
import { useEffect, useRef, useState } from "react";
import * as api from "./api";
import { useTranslate } from "./i18n";

/** How often the state is asked for while a write runs. */
export const JOURNAL_POLL_MS = 400;

/**
 * Shows nothing while every edit is in the file. After `revision` changes in a
 * project that has a file (`saved`), it asks the server until the write is
 * done; a failure stays visible with a retry until a write succeeds. An unsaved
 * project journals nothing: changes before its first save, and the save
 * itself, ask nothing.
 */
export default function JournalStatusIndicator({ revision, saved }: { revision: number | undefined; saved: boolean }) {
  const t = useTranslate();
  const [status, setStatus] = useState<api.JournalStatus>({ state: "saved" });
  // The last revision already in the file as far as this note knows: while
  // unsaved it follows every revision, so the first save starts from there.
  const seen = useRef(revision);
  const [checks, setChecks] = useState(0);
  const handled = useRef(checks);
  const wasSaved = useRef(saved);

  useEffect(() => {
    const retried = checks !== handled.current;
    handled.current = checks;
    const firstSave = !wasSaved.current && saved;
    wasSaved.current = saved;
    // Unsaved, nothing is journaled; the first save writes everything itself.
    if (!saved || firstSave) { seen.current = revision; return; }
    if (revision === seen.current && !retried) return;
    seen.current = revision;
    let stopped = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const poll = () => {
      api.journalStatus().then((next) => {
        if (stopped) return;
        setStatus(next);
        if (next.state === "saving") timer = setTimeout(poll, JOURNAL_POLL_MS);
      }, () => {
        // An older server without this route has no background writer to show.
        if (!stopped) setStatus({ state: "saved" });
      });
    };
    poll();
    return () => { stopped = true; if (timer !== undefined) clearTimeout(timer); };
  }, [revision, checks, saved]);

  if (status.state === "saved") return null;
  if (status.state === "saving") {
    return <span className="workbench-status-journal" role="status">{t("statusBar.journalSaving")}</span>;
  }
  return (
    <span className="workbench-status-journal workbench-status-journal-failed" role="alert">
      {t("statusBar.journalFailed", { message: status.message })}
      <button type="button" onClick={() => {
        api.retryJournal().then(setStatus, () => undefined).finally(() => setChecks((n) => n + 1));
      }}>{t("statusBar.journalRetry")}</button>
    </span>
  );
}
