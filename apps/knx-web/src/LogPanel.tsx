/** Panel rendering the backend session log, refetched on tree changes and failed operations. */
// apps/knx-web/src/LogPanel.tsx
import { useEffect, useState } from "react";
import * as api from "./api";
import type { LogEntry } from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import { useTranslate } from "./i18n";
import type { MessageKey } from "./messages/en";

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
  const [error, setError] = useState<string | null>(null);
  const [filters, setFilters] = useState<Record<Severity, boolean>>({
    error: true,
    warning: true,
    info: true,
  });

  useEffect(() => {
    let cancelled = false;
    setError(null); // clear a stale error from a previous fetch before retrying
    api
      .getSessionLog()
      .then((fetched) => {
        if (!cancelled) setEntries(fetched);
      })
      .catch((e) => {
        if (!cancelled) setError(api.errorMessage(e));
      });
    return () => {
      cancelled = true;
    };
  }, [props.tree, props.refreshKey]);

  function toggleFilter(severity: Severity) {
    setFilters((f) => ({ ...f, [severity]: !f[severity] }));
  }

  // Backend appends chronologically (oldest first) — reverse for
  // newest-first display, then apply the (render-only) severity filter.
  const visible = [...entries].reverse().filter((entry) => filters[entry.severity]);

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
      {error && <span className="field-error">{error}</span>}
      {entries.length === 0 ? (
        <p className="log-panel-empty">{t("logPanel.emptyNoEntries")}</p>
      ) : visible.length === 0 ? (
        <p className="log-panel-empty">{t("logPanel.emptyFiltered")}</p>
      ) : (
        <ul className="log-panel-list">
          {visible.map((entry, index) => (
            <li key={`${entry.timestamp}-${index}`} className={`log-entry log-entry-${entry.severity}`}>
              <span className="log-entry-severity">{t(SEVERITY_LABEL_KEYS[entry.severity])}</span>
              <span className="log-entry-source">{entry.source}</span>
              <span className="log-entry-message">{entry.message}</span>
              {entry.location && <span className="log-entry-location">{entry.location}</span>}
              {entry.detail && <span className="log-entry-detail">{entry.detail}</span>}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
