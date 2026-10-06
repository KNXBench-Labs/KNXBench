/** UI-04: what this server holds right now — partial, volatile, never proof of an idle bus. */
import { useEffect, useRef, useState } from "react";
import * as api from "./api";
import { HistoryContractError } from "./activityHistory";
import { useTranslate } from "./i18n";
import type { ActivitySnapshot, SessionActivity } from "./liveActivity";
import { useUiLanguage } from "./uiLanguage";

/** How often the open tab asks again; the server answers without waiting on a bus lock. */
export const LIVE_POLL_MS = 2_000;

type Failure = "unavailable" | "malformed";

export default function BusActivityLive() {
  const t = useTranslate();
  const [language] = useUiLanguage();
  const [snapshot, setSnapshot] = useState<ActivitySnapshot | null>(null);
  const [failure, setFailure] = useState<Failure | null>(null);
  const [restarted, setRestarted] = useState(false);
  const incarnation = useRef<string | null>(null);
  const generation = useRef(0);

  useEffect(() => {
    let disposed = false;
    async function poll() {
      // A hidden page asks nothing; the next visible tick catches up.
      if (document.visibilityState !== "visible") return;
      const ticket = ++generation.current;
      try {
        const next = await api.busActivity();
        if (disposed || ticket !== generation.current) return;
        if (incarnation.current !== null && incarnation.current !== next.serverIncarnation) setRestarted(true);
        incarnation.current = next.serverIncarnation;
        setSnapshot(next);
        setFailure(null);
      } catch (reason) {
        if (disposed || ticket !== generation.current) return;
        // No stale snapshot next to a failure, and no raw server text.
        setSnapshot(null);
        setFailure(reason instanceof HistoryContractError ? "malformed" : "unavailable");
      }
    }
    void poll();
    const timer = setInterval(() => void poll(), LIVE_POLL_MS);
    return () => { disposed = true; clearInterval(timer); };
  }, []);

  function time(value: string) {
    return <time dateTime={value}>{new Intl.DateTimeFormat(language, { timeStyle: "medium" }).format(new Date(value))}</time>;
  }
  function progress(session: SessionActivity): string | null {
    switch (session.kind) {
      case "deviceDownload":
        return t("activityLive.progress.download", { done: session.completedSteps, steps: session.totalSteps,
          written: session.writtenOctets, octets: session.totalOctets });
      case "lineScan":
        return t("activityLive.progress.scan", { probed: session.probed, total: session.total });
      case "addressProgramming":
        return session.rounds === undefined ? null : t("activityLive.progress.rounds", { rounds: session.rounds });
      case "busMonitor":
        return null;
    }
  }

  return (
    <section className="activity-live-panel" aria-live="polite">
      <header>
        <p className="eyebrow">{t("activityLive.source")}</p>
        <h2>{t("activityLive.title")}</h2>
        <p>{t("activityLive.partial")}</p>
        <p>{t("activityLive.volatile")}</p>
      </header>
      {restarted && <p className="activity-live-restart" role="status">{t("activityLive.restarted")}</p>}
      {failure !== null && <p role="alert">{t(`activityLive.${failure}`)}</p>}
      {snapshot === null && failure === null && <p role="status">{t("activityLive.loading")}</p>}
      {snapshot !== null && <>
        <p className="activity-live-history-state" role={snapshot.historyState === "unavailable" ? "alert" : undefined}>
          {t(`activityLive.historyState.${snapshot.historyState}`)}
        </p>
        <h3>{t("activityLive.sessions")}</h3>
        {snapshot.sessions.length === 0
          ? <p>{t("activityLive.noSessions")}</p>
          : <ul className="activity-live-list">
            {snapshot.sessions.map((session) => <li className="activity-live-session" key={session.kind}>
              <strong>{t(`activityHistory.kind.${session.kind}`)}{"address" in session ? ` — ${session.address}` : ""}</strong>
              <span>{t(`activityLive.state.${session.state}`)}</span>
              {progress(session) !== null && <span>{progress(session)}</span>}
            </li>)}
          </ul>}
        {snapshot.busyLocks.length > 0 && <p className="activity-live-busy">
          {t("activityLive.busy")}: {snapshot.busyLocks.map((lock) => t(`activityLive.lock.${lock}`)).join(", ")}
        </p>}
        <h3>{t("activityLive.oneShot")}</h3>
        {snapshot.oneShot.length === 0
          ? <p>{t("activityLive.noOneShot")}</p>
          : <ol className="activity-live-list">
            {snapshot.oneShot.map((entry) => <li className="activity-live-oneshot" key={entry.id}>
              <strong>{t(`activityHistory.kind.${entry.kind}`)}{entry.address === null ? "" : ` — ${entry.address}`}</strong>
              <span>{t(`activityHistory.state.${entry.state}`)}</span>
              <span>{time(entry.startedAt)}{entry.finishedAt === null ? "" : " – "}{entry.finishedAt === null ? null : time(entry.finishedAt)}</span>
            </li>)}
          </ol>}
        {snapshot.oneShotDropped > 0 && <p>{t(snapshot.oneShotDropped === 1 ? "activityLive.dropped.one" : "activityLive.dropped.other",
          { count: snapshot.oneShotDropped })}</p>}
        <p>{t("activityLive.untracked")}: {snapshot.untracked.map((kind) => t(`activityHistory.kind.${kind}`)).join(", ")
          || t("activityHistory.noneDeclared")}</p>
      </>}
    </section>
  );
}
