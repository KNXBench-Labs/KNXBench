/** Panel rendering the backend session log, refetched on tree changes and failed operations. */
// apps/knx-web/src/LogPanel.tsx
import { useEffect, useId, useMemo, useRef, useState } from "react";
import LogEntryDetails from "./LogEntryDetails";
import LogExportDialog from "./LogExportDialog";
import { useUiLanguage } from "./uiLanguage";
import type { ToastNotification } from "./toast";
import * as api from "./api";
import type { LogEntry } from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import { useTranslate } from "./i18n";
import type { MessageKey } from "./messages/en";
import { formatSettingsDiagnostic } from "./settingsDiagnostic";
import { droppedCount, saveSessionLog } from "./sessionLogExport";
import { countImportFindings, isImportFinding, type ImportFindingSeverity } from "./importFindings";
import type { LogExportScope } from "./sessionLogExport";
import { LOG_RENDER_STEP, detailPreview, formatLogTime, newestFirst, notificationRows, serverRows } from "./logRows";

type Severity = LogEntry["severity"];

const SEVERITIES: Severity[] = ["error", "warning", "info"];

// Small closed enum -> its own label key. Not a domain discriminant used for
// wire communication (that's `entry.severity` as sent/received verbatim);
// this is purely how the three severities are rendered as UI chrome.
const SEVERITY_LABEL_KEYS: Record<Severity, MessageKey> = {
  error: "logPanel.severity.error",
  warning: "logPanel.severity.warning",
  info: "logPanel.severity.info",
};

/// Renders the backend session log (`GET /api/log`). Fetches on mount and
/// whenever `props.tree` changes (a new tree reference means an
/// edit/undo/redo/import just landed, so the log may have grown) or
/// `props.refreshKey` changes (bumped by `App.tsx` on every *failed*
/// operation — those never change `props.tree`, since the operation didn't
/// succeed, but still append an entry on the server that needs to surface
/// here) — the severity filters below are purely a client-side render
/// filter over the already-fetched entries and never trigger a re-fetch on
/// their own.
///
/// `tree` is `null` when no project is open: `GET /api/log` deliberately
/// still answers (see `routes.rs`), so this panel is reachable — and still
/// refetches on every `tree`/`refreshKey` change — with or without one
/// (KNOWN_LIMITATIONS.md #36, part A).
const NO_NOTIFICATIONS: ToastNotification[] = [];

/** A request to show one severity of the current project's import findings.
 *  The number to compare against is always read from the current `tree`,
 *  so a project replaced while the log stays open cannot leave a stale one. */
export interface LogImportFocus {
  severity: ImportFindingSeverity;
}

export default function LogPanel(props: {
  tree: ProjectTree | null; refreshKey: number;
  notifications?: ToastNotification[]; notificationDroppedCount?: number;
  notificationTarget?: { id: number } | null;
  onNotificationTargetHandled?: () => void;
  importFocus?: LogImportFocus | null;
  onImportFocusHandled?: () => void;
}) {
  const t = useTranslate();
  const [language] = useUiLanguage();
  const [entries, setEntries] = useState<LogEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [filters, setFilters] = useState<Record<Severity, boolean>>({
    error: true,
    warning: true,
    info: true,
  });
  const [search, setSearch] = useState("");
  const [exportOpen, setExportOpen] = useState(false);
  const [selected, setSelected] = useState<{ key: string; entry: LogEntry } | null>(null);
  const [targetMissing, setTargetMissing] = useState(false);
  // While set, only import-report entries are listed (severity filters still
  // apply); the note above the list says so and offers the way back.
  const [importScope, setImportScope] = useState<LogImportFocus | null>(null);
  const importScopeRef = useRef<HTMLDivElement>(null);
  const importScopeTitleId = useId();
  const list = useRef<HTMLUListElement>(null);
  const restoreFocus = useRef<HTMLElement | null>(null);
  const notifications = props.notifications ?? NO_NOTIFICATIONS;
  const target = props.notificationTarget;
  useEffect(() => {
    if (!target) return;
    const notification = notifications.find(n => n.id === target.id);
    setTargetMissing(!notification);
    setSelected(notification ? { key: `ui:${notification.id}`, entry: notification.entry } : null);
    if (notification) {
      setSearch("");
      setFilters({ error: true, warning: true, info: true });
    }
    props.onNotificationTargetHandled?.();
  }, [target]);
  useEffect(() => {
    const focus = props.importFocus;
    if (!focus) return;
    setImportScope(focus);
    setSelected(null);
    setTargetMissing(false);
    setSearch("");
    setFilters({ error: focus.severity === "error", warning: focus.severity === "warning", info: false });
    props.onImportFocusHandled?.();
  }, [props.importFocus]);
  // The control that asked for this view is gone (the log replaced the
  // Overview), so keyboard focus moves to the note explaining the view.
  useEffect(() => {
    if (importScope) importScopeRef.current?.focus();
  }, [importScope]);
  useEffect(() => {
    if (!selected) return;
    const row = [...(list.current?.querySelectorAll<HTMLElement>("[data-log-key]") ?? [])]
      .find(row => row.dataset.logKey === selected.key);
    restoreFocus.current = row?.querySelector<HTMLElement>(".log-entry-open") ?? null;
    row?.scrollIntoView?.({ block: "nearest" });
  }, [selected, loading]);

  useEffect(() => {
    let cancelled = false;
    // A new project/log revision invalidates the previous snapshot at once:
    // it must not remain exportable while the next request is in flight.
    setEntries([]);
    setLoading(true);
    setError(null);
    api
      .getSessionLog()
      .then((fetched) => {
        if (!cancelled) {
          setEntries(fetched);
          setLoading(false);
        }
      })
      .catch((e) => {
        if (!cancelled) {
          setError(api.errorMessage(e));
          setLoading(false);
        }
      });
    return () => {
      cancelled = true;
    };
  }, [props.tree, props.refreshKey]);

  function toggleFilter(severity: Severity) {
    setFilters((f) => ({ ...f, [severity]: !f[severity] }));
  }

  // Newest first for display; export-all keeps the server's chronological
  // ordering while a filtered export mirrors the current visible order.
  const query = search.trim().toLocaleLowerCase();
  // Up to 16000 server entries (#7): keys, times and order are computed once
  // per snapshot, not on every keystroke, and only a page of rows is rendered.
  const serverRowList = useMemo(() => serverRows(entries), [entries]);
  const rows = useMemo(() => newestFirst([...serverRowList, ...notificationRows(notifications)]), [serverRowList, notifications]);
  const allEntries = notifications.length === 0 ? entries : [...rows].reverse().map(row => row.entry);
  const scopedToImport = Boolean(importScope && props.tree);
  const visibleRows = useMemo(() => rows.filter(({ entry }) =>
    filters[entry.severity] &&
    (!scopedToImport || isImportFinding(entry)) &&
    (!query || [entry.source, entry.message, formatSettingsDiagnostic(t, entry.diagnostic, entry.message), entry.location ?? "", entry.detail ?? ""]
      .some((field) => field.toLocaleLowerCase().includes(query))),
  ), [rows, filters, scopedToImport, query, t]);
  const visibleServerEntries = visibleRows.filter(row => row.key.startsWith("server:")).map(row => row.entry);
  const [renderLimit, setRenderLimit] = useState(LOG_RENDER_STEP);
  // A new filter or search starts at the first page again; a refetch (every
  // project change while the panel is open) keeps what the user expanded.
  useEffect(() => setRenderLimit(LOG_RENDER_STEP), [filters, query, scopedToImport]);
  // A selected entry beyond the current page (opened from a notification) is still shown.
  const selectedIndex = selected ? visibleRows.findIndex(row => row.key === selected.key) : -1;
  const shownRows = visibleRows.slice(0, Math.max(renderLimit, selectedIndex + 1));
  const remainingRows = visibleRows.length - shownRows.length;

  function clearImportScope() {
    setImportScope(null);
    setFilters({ error: true, warning: true, info: true });
  }

  function importScopeMessage(scope: LogImportFocus, tree: ProjectTree): string {
    const found = countImportFindings(entries, scope.severity);
    const expected = scope.severity === "error" ? tree.errors : tree.warnings;
    const findings = scope.severity === "error"
      ? t("logPanel.importScope.errors", { count: expected })
      : t("logPanel.importScope.warnings", { count: expected });
    if (found === expected) return t("logPanel.importScope.match", { findings });
    if (found === 0) return t("logPanel.importScope.none", { findings });
    return t("logPanel.importScope.mismatch", { findings, entries: t("logPanel.importScope.entries", { count: found }) });
  }

  async function exportLog(scope: LogExportScope) {
    try {
      // v1/native validation describes the server run and its retained server records.
      // Browser notifications have independent retention/loss; copy them from details.
      await saveSessionLog(entries, scope === "all" ? entries : visibleServerEntries, scope);
    } catch (e) {
      throw new Error(api.errorMessage(e));
    }
  }
  // One clock per render: "today" is the reader's local day.
  const now = new Date();

  return (
    <div className="log-panel">
      {/* Same `.workspace-heading` shape as `StructureWorkspace`/
          `DeviceWorkspace`: eyebrow, `h1`, actions on the right. The
          severity checkboxes are this view's actions, so they sit where
          every other workspace puts its action cluster. */}
      <header className="workspace-heading">
        <div>
          <p className="eyebrow">{t("logPanel.eyebrow")}</p>
          <h1>{t("logPanel.title")}</h1>
        </div>
        <div className="log-panel-filters">
          {SEVERITIES.map((severity) => (
            <label key={severity} className="log-panel-filter">
              <input
                type="checkbox"
                checked={filters[severity]}
                onChange={() => toggleFilter(severity)}
              />
              {t(SEVERITY_LABEL_KEYS[severity])}
            </label>
          ))}
        </div>
      </header>
      <div className="log-panel-tools">
        <label className="log-panel-search">
          {t("logPanel.search")}
          <input type="search" aria-label={t("logPanel.search")} value={search} onChange={(event) => setSearch(event.target.value)} />
        </label>
        <button type="button" className="log-panel-clear" onClick={() => setSearch("")} disabled={!search}>
          {t("logPanel.clear")}
        </button>
        <span className="log-panel-count" role="status">{t("logPanel.count", { shown: visibleRows.length, total: allEntries.length })}</span>
        {/* #13: one small button; the scope is chosen in its dialog. */}
        <button type="button" className="log-panel-export" onClick={() => setExportOpen(true)} disabled={loading || error !== null}>{t("logPanel.export")}</button>
      </div>
      {exportOpen && <LogExportDialog filteredCount={visibleServerEntries.length} allCount={entries.length}
        disabled={loading || error !== null} onExport={exportLog} onClose={() => setExportOpen(false)} />}
      <p className="log-panel-hint">{t("logPanel.retention", { count: droppedCount(entries) ?? t("logPanel.unknownCount") })}</p>
      {(notifications.length > 0 || (props.notificationDroppedCount ?? 0) > 0) && <p className="log-panel-hint">{t("logPanel.uiHistory", { count: props.notificationDroppedCount ?? 0 })}</p>}
      {targetMissing && <p role="status">{t("logPanel.targetMissing")}</p>}
      {importScope && props.tree && (
        <div className="log-panel-import-scope" role="region" ref={importScopeRef} tabIndex={-1} aria-labelledby={importScopeTitleId}>
          <div>
            <strong id={importScopeTitleId}>{t("logPanel.importScope.title")}</strong>
            {/* A failed fetch is reported below; it is not "none in the log". */}
            <p role="status">{loading ? t("logPanel.loading") : error ? "" : importScopeMessage(importScope, props.tree)}</p>
          </div>
          <button type="button" className="log-panel-import-scope-clear" onClick={clearImportScope}>{t("logPanel.importScope.showAll")}</button>
        </div>
      )}
      {selected && <LogEntryDetails key={selected.key} entry={selected.entry} restoreFocusRef={restoreFocus} onClose={() => {
        setSelected(null);
        // The navigation toast may still be leaving. Restore to its durable row,
        // after Overlay has released background isolation, not to that transient trigger.
        queueMicrotask(() => {
          const row = restoreFocus.current;
          if (row?.isConnected && !row.closest("[inert]")) row.focus();
        });
      }} />}
      {/* §66/§67 disclosure (fix round 2, B4): message/location/detail below
          are the server's own text and are never translated — stated here,
          where the reader actually meets them, not just in
          KNOWN_LIMITATIONS.md. Shown whenever there is at least one entry
          to disclose about, independent of the severity filter above. */}
      {entries.length > 0 && <p className="log-panel-hint">{t("logPanel.entryTextIsEnglish")}</p>}
      {error && <span className="field-error">{error}</span>}
      {loading ? (
        <p className="log-panel-empty">{t("logPanel.loading")}</p>
      ) : error && allEntries.length === 0 ? null : allEntries.length === 0 ? (
        <p className="log-panel-empty">{t("logPanel.emptyNoEntries")}</p>
      ) : visibleRows.length === 0 ? (
        <p className="log-panel-empty">{t("logPanel.emptyFiltered")}</p>
      ) : (
        <>
        {/* #13: real columns. The header is visual; each row still reads in order. */}
        <div className="log-panel-table">
        <div className="log-panel-columns" aria-hidden="true">
          <span>{t("logPanel.column.time")}</span><span>{t("logPanel.column.level")}</span>
          <span>{t("logPanel.column.source")}</span><span>{t("logPanel.column.message")}</span>
        </div>
        <ul className="log-panel-list" ref={list}>
          {shownRows.map(({ entry, key, time }) => {
            const shown = formatLogTime(time, language, now);
            return (
            <li key={key} data-log-key={key} className={`log-entry log-entry-${entry.severity}${selected?.key === key ? " log-entry--selected" : ""}`}>
              <time className="log-entry-time" dateTime={shown === null ? undefined : entry.timestamp} title={entry.timestamp}>{shown ?? "—"}</time>
              <span className="log-entry-severity">{t(SEVERITY_LABEL_KEYS[entry.severity])}</span>
              <span className="log-entry-source" title={entry.source}>{entry.source}</span>
              <span className="log-entry-text">
                <button type="button" className="log-entry-message log-entry-open" onClick={() => setSelected({ key, entry })} title={t("logPanel.details")}>
                  {formatSettingsDiagnostic(t, entry.diagnostic, entry.message)}
                </button>
                {entry.location && <span className="log-entry-location">{entry.location}</span>}
                {entry.detail && entry.source !== "import:opaque:raw-text" && <LogDetailPreview detail={entry.detail} />}
              </span>
              {/* ADR-0110: a raw RTF-like preview stays literal and visible, so this row is taller on purpose. */}
              {entry.source === "import:opaque:raw-text" && <span className="log-entry-raw-label">{t("logPanel.rawRtfLike")}</span>}
              {entry.detail && entry.source === "import:opaque:raw-text" && <pre className="log-entry-detail log-entry-raw-text">{entry.detail}</pre>}
            </li>
            );
          })}
        </ul>
        </div>
        {remainingRows > 0 && (
          <p className="log-panel-more">
            {t("logPanel.renderLimit", { shown: shownRows.length, total: visibleRows.length })}{" "}
            <button type="button" className="log-panel-show-more" onClick={() => setRenderLimit(shownRows.length + LOG_RENDER_STEP)}>
              {t("logPanel.showMore", { count: Math.min(LOG_RENDER_STEP, remainingRows) })}
            </button>
          </p>
        )}
        </>
      )}
    </div>
  );
}

/** A detail shows its first line here, cut to the row; the dialog shows all of it. */
function LogDetailPreview({ detail }: { detail: string }) {
  const t = useTranslate();
  const { text, hiddenLines } = detailPreview(detail);
  // The text is cut to the row; the "more lines" note never is.
  return <span className="log-entry-detail">
    <span className="log-entry-detail-text">{text}</span>
    {hiddenLines > 0 && <span className="log-entry-detail-more">{t("logPanel.detailMoreLines", { count: hiddenLines })}</span>}
  </span>;
}
