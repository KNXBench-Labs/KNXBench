// apps/knx-web/src/LogPanel.tsx
import { useEffect, useState } from "react";
import * as api from "./api";
import type { LogEntry } from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";

type Severity = LogEntry["severity"];

const SEVERITIES: Severity[] = ["error", "warning", "info"];

/// Renders the backend session log (`GET /api/log`). Fetches on mount and
/// whenever `props.tree` changes (a new tree reference means an
/// edit/undo/redo/import just landed, so the log may have grown) — the
/// severity filters below are purely a client-side render filter over the
/// already-fetched entries and never trigger a re-fetch on their own.
export default function LogPanel(props: { tree: ProjectTree }) {
  const [entries, setEntries] = useState<LogEntry[]>([]);
  const [filters, setFilters] = useState<Record<Severity, boolean>>({
    error: true,
    warning: true,
    info: true,
  });

  useEffect(() => {
    let cancelled = false;
    api.getSessionLog().then((fetched) => {
      if (!cancelled) setEntries(fetched);
    });
    return () => {
      cancelled = true;
    };
  }, [props.tree]);

  function toggleFilter(severity: Severity) {
    setFilters((f) => ({ ...f, [severity]: !f[severity] }));
  }

  // Backend appends chronologically (oldest first) — reverse for
  // newest-first display, then apply the (render-only) severity filter.
  const visible = [...entries].reverse().filter((entry) => filters[entry.severity]);

  return (
    <div className="log-panel">
      <h2>Session log</h2>
      <div className="log-panel-filters">
        {SEVERITIES.map((severity) => (
          <label key={severity} className="log-panel-filter">
            <input
              type="checkbox"
              checked={filters[severity]}
              onChange={() => toggleFilter(severity)}
            />
            {severity[0].toUpperCase() + severity.slice(1)}
          </label>
        ))}
      </div>
      {entries.length === 0 ? (
        <p className="log-panel-empty">No log entries yet.</p>
      ) : (
        <ul className="log-panel-list">
          {visible.map((entry, index) => (
            <li key={`${entry.timestamp}-${index}`} className={`log-entry log-entry-${entry.severity}`}>
              <span className="log-entry-severity">{entry.severity}</span>
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
