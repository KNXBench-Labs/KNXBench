/** Exact raw log snapshot; no interpretation, HTML rendering, or persistence. */
import { useRef, useState, type RefObject } from "react";
import { createPortal } from "react-dom";
import type { LogEntry } from "./api";
import Overlay from "./Overlay";
import { useTranslate } from "./i18n";

export default function LogEntryDetails({ entry, restoreFocusRef, onClose }: { entry: LogEntry; restoreFocusRef?: RefObject<HTMLElement | null>; onClose: () => void }) {
  const t = useTranslate();
  const text = JSON.stringify(entry, null, 2);
  const field = useRef<HTMLTextAreaElement>(null);
  const [copyState, setCopyState] = useState<"copied" | "failed" | null>(null);
  async function copy(value: string) {
    setCopyState(null);
    try { await navigator.clipboard.writeText(value); setCopyState("copied"); }
    catch { setCopyState("failed"); field.current?.focus(); field.current?.select(); }
  }
  return createPortal(<Overlay label={t("logPanel.details")} className="log-detail-panel" initialFocusRef={field} restoreFocusRef={restoreFocusRef} onClose={onClose}>
    <header className="log-detail-heading"><h2>{t("logPanel.details")}</h2><button type="button" onClick={onClose}>{t("logPanel.closeDetails")}</button></header>
    <p className="log-detail-meta">{entry.timestamp} · {entry.severity} · {entry.source}</p>
    <pre className="log-detail-message">{entry.message}</pre>
    {entry.location !== null && <pre>{entry.location}</pre>}
    {entry.detail !== null && <pre>{entry.detail}</pre>}
    <div className="log-detail-actions">
      <button type="button" className="log-detail-copy-message" onClick={() => void copy(entry.message)}>{t("logPanel.copyMessage")}</button>
      <button type="button" className="log-detail-copy-all" onClick={() => void copy(text)}>{t("logPanel.copyAll")}</button>
    </div>
    {copyState && <p role={copyState === "failed" ? "alert" : "status"}>{t(copyState === "failed" ? "logPanel.copyFailed" : "logPanel.copied")}</p>}
    <label className="log-detail-raw">{t("logPanel.rawEntry")}<textarea ref={field} readOnly spellCheck={false} value={text} /></label>
  </Overlay>, document.body);
}
