/** Read-only commissioning metadata, never a recovery image or authority to write a device. */
import { useEffect, useRef, useState } from "react";
import * as api from "./api";
import { appendHistoryPage, HistoryContractError, type HistoryEntry, type UntrackedKind } from "./activityHistory";
import { useTranslate } from "./i18n";
import { useUiLanguage } from "./uiLanguage";

export default function BusActivityHistory() {
  const t = useTranslate();
  const [language] = useUiLanguage();
  const [entries, setEntries] = useState<HistoryEntry[]>([]);
  const [untracked, setUntracked] = useState<UntrackedKind[]>([]);
  const [cursor, setCursor] = useState(0);
  const [more, setMore] = useState(false);
  const [loading, setLoading] = useState(true);
  const [loaded, setLoaded] = useState(false);
  const [failure, setFailure] = useState<"unavailable" | "unsupported" | "malformed" | null>(null);
  const generation = useRef(0);

  async function load(append: boolean) {
    const ticket = ++generation.current;
    setLoading(true);
    setFailure(null);
    try {
      const page = await api.activityHistory(append ? cursor : 0);
      if (ticket !== generation.current) return;
      const next = append ? appendHistoryPage(entries, page) : page.entries;
      setEntries(next);
      setCursor(page.nextCursor);
      setMore(page.hasMore);
      setUntracked(page.untracked);
      setLoaded(true);
    } catch (reason) {
      if (ticket !== generation.current) return;
      setEntries([]);
      setUntracked([]);
      setMore(false);
      // No raw server/SQLite/backup-path text in the UI.
      setFailure(reason instanceof HistoryContractError ? reason.reason : "unavailable");
    } finally {
      if (ticket === generation.current) setLoading(false);
    }
  }

  useEffect(() => {
    void load(false);
    return () => { generation.current += 1; };
  }, []);

  function time(value: string) {
    return <time dateTime={value}>{new Intl.DateTimeFormat(language, { dateStyle: "short", timeStyle: "medium" }).format(new Date(value))}</time>;
  }

  return (
    <section className="activity-history-panel" aria-busy={loading}>
      <header>
        <p className="eyebrow">{t("activityHistory.source")}</p>
        <h2>{t("activityHistory.title")}</h2>
        <p>{t("activityHistory.partial")}</p>
        <p>{t("activityHistory.notRecovery")}</p>
        <p className="activity-history-boundary">{t("activityHistory.validationBoundary")}</p>
      </header>
      <div className="activity-history-controls">
        <button disabled={loading} onClick={() => void load(false)}>{t("activityHistory.refresh")}</button>
        {more && <button disabled={loading} onClick={() => void load(true)}>{t("activityHistory.more")}</button>}
      </div>
      <p>{t("activityHistory.cursorMeaning")}</p>
      {loading && <p role="status">{t("activityHistory.loading")}</p>}
      {failure !== null && <p role="alert">{t(`activityHistory.${failure}`)}</p>}
      {failure === null && loaded && entries.length === 0 && <p>{t("activityHistory.empty")}</p>}
      {failure === null && loaded && <>
        <p>{t("activityHistory.untracked")}: {untracked.map((kind) => t(`activityHistory.kind.${kind}`)).join(", ") || t("activityHistory.noneDeclared")}</p>
        <ol className="activity-history-list">
          {entries.map((entry) => <li className="activity-history-entry" key={entry.sequence}>
            <h3>{t(`activityHistory.kind.${entry.kind}`)}{entry.address === null ? "" : ` — ${entry.address}`}</h3>
            <p>{t(`activityHistory.state.${entry.state}`)}</p>
            {entry.interrupted && <p className="activity-history-warning">{t("activityHistory.interrupted")}</p>}
            <dl>
              <dt>{t("activityHistory.started")}</dt><dd>{time(entry.startedAt)}</dd>
              <dt>{t("activityHistory.finished")}</dt><dd>{entry.finishedAt === null ? t("activityHistory.noFinish") : time(entry.finishedAt)}</dd>
              {entry.writeEvidence && <>
                <dt>{t("activityHistory.backup")}</dt><dd>{t(entry.writeEvidence.backupRecorded ? "activityHistory.recorded" : "activityHistory.notRecorded")}</dd>
                <dt>{t("activityHistory.intent")}</dt><dd>{t(entry.writeEvidence.sendPossible ? "activityHistory.sendPossible" : "activityHistory.noSendRecorded")}</dd>
              </>}
              {entry.downloadEvidence && <>
                <dt>{t("activityHistory.written")}</dt><dd>{t(`activityHistory.written.${entry.downloadEvidence.written ?? "unknown"}`)}</dd>
                <dt>{t("activityHistory.restart")}</dt><dd>{t(`activityHistory.restart.${entry.downloadEvidence.restart ?? "unknown"}`)}</dd>
                <dt>{t("activityHistory.cleanup")}</dt><dd>{t(`activityHistory.cleanup.${entry.downloadEvidence.cleanup}`)}</dd>
              </>}
            </dl>
          </li>)}
        </ol>
      </>}
    </section>
  );
}
