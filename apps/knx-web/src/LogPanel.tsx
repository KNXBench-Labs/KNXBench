/** Panel rendering the backend session log, refetched on tree changes and failed operations. */
// apps/knx-web/src/LogPanel.tsx
import { useEffect, useState } from "react";
import * as api from "./api";
import type { LogEntry } from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import { useTranslate } from "./i18n";
import type { MessageKey } from "./messages/en";
import { formatSettingsDiagnostic } from "./settingsDiagnostic";
import { droppedCount, saveSessionLog } from "./sessionLogExport";
import type { LogExportScope } from "./sessionLogExport";

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
export default function LogPanel(props: { tree: ProjectTree | null; refreshKey: number }) {
  const t = useTranslate();
  const [entries, setEntries] = useState<LogEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [filters, setFilters] = useState<Record<Severity, boolean>>({
    error: true,
    warning: true,
    info: true,
  });
  const [search, setSearch] = useState("");
  const [exportScope, setExportScope] = useState<LogExportScope>("filtered");
  const [exportError, setExportError] = useState<string | null>(null);

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
  const visible = [...entries].reverse().filter((entry) =>
    filters[entry.severity] &&
    (!query || [entry.source, formatSettingsDiagnostic(t, entry.diagnostic, entry.message), entry.detail ?? ""]
      .some((field) => field.toLocaleLowerCase().includes(query))),
  );

  async function exportLog() {
    setExportError(null);
    try {
      await saveSessionLog(entries, exportScope === "all" ? entries : visible, exportScope);
    } catch (e) {
      setExportError(api.errorMessage(e));
    }
  }

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
        <span className="log-panel-count" role="status">{t("logPanel.count", { shown: visible.length, total: entries.length })}</span>
        <label className="log-panel-scope">
          {t("logPanel.exportScope")}
          <select className="log-panel-export-scope" value={exportScope} onChange={(event) => setExportScope(event.target.value as LogExportScope)}>
            <option value="filtered">{t("logPanel.exportFiltered")}</option>
            <option value="all">{t("logPanel.exportAll")}</option>
          </select>
        </label>
        <button type="button" className="log-panel-export" onClick={exportLog} disabled={loading || error !== null}>{t("logPanel.exportJson")}</button>
      </div>
      <p className="log-panel-hint">{t("logPanel.retention", { count: droppedCount(entries) ?? t("logPanel.unknownCount") })}</p>
      {exportError && <span className="field-error" role="alert">{exportError}</span>}
      {/* §66/§67 disclosure (fix round 2, B4): message/location/detail below
          are the server's own text and are never translated — stated here,
          where the reader actually meets them, not just in
          KNOWN_LIMITATIONS.md. Shown whenever there is at least one entry
          to disclose about, independent of the severity filter above. */}
      {entries.length > 0 && <p className="log-panel-hint">{t("logPanel.entryTextIsEnglish")}</p>}
      {error && <span className="field-error">{error}</span>}
      {loading ? (
        <p className="log-panel-empty">{t("logPanel.loading")}</p>
      ) : error ? null : entries.length === 0 ? (
        <p className="log-panel-empty">{t("logPanel.emptyNoEntries")}</p>
      ) : visible.length === 0 ? (
        <p className="log-panel-empty">{t("logPanel.emptyFiltered")}</p>
      ) : (
        <ul className="log-panel-list">
          {visible.map((entry, index) => (
            <li key={`${entry.timestamp}-${index}`} className={`log-entry log-entry-${entry.severity}`}>
              <span className="log-entry-severity">{t(SEVERITY_LABEL_KEYS[entry.severity])}</span>
              <span className="log-entry-source">{entry.source}</span>
              <span className="log-entry-message">
                {formatSettingsDiagnostic(t, entry.diagnostic, entry.message)}
              </span>
              {entry.location && <span className="log-entry-location">{entry.location}</span>}
              {entry.detail && <span className="log-entry-detail">{entry.detail}</span>}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
