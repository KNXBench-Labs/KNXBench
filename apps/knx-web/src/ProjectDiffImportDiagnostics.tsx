/** Collapsed list of the ETS import diagnostics a `.knxproj` comparison input produced. */
import type { LogEntry } from "./api";
import { useTranslate } from "./i18n";
import type { MessageKey } from "./messages/en";
import { importDiagnosticsSummary } from "./projectDiffView";

const SEVERITY_KEYS: Record<LogEntry["severity"], MessageKey> = {
  error: "logPanel.severity.error",
  warning: "logPanel.severity.warning",
  info: "logPanel.severity.info",
};

// Collapsed by default: the diff is what the user asked for, the report is
// what they need to trust it. The summary line always carries the count,
// so a closed block still says how much there is to read.
export default function ProjectDiffImportDiagnostics(props: { diagnostics: LogEntry[] }) {
  const { diagnostics } = props;
  const t = useTranslate();
  return (
    <details className="project-diff-import">
      <summary>{importDiagnosticsSummary(t, diagnostics)}</summary>
      <ul className="project-diff-import-list">
        {diagnostics.map((entry, index) => (
          <li key={index} className={`project-diff-import-${entry.severity}`}>
            <span className="project-diff-import-severity">{t(SEVERITY_KEYS[entry.severity])}</span>{" "}
            {entry.message}
            {entry.location && <code className="project-diff-import-location">{entry.location}</code>}
            {entry.detail && <span className="project-diff-import-detail">{entry.detail}</span>}
          </li>
        ))}
      </ul>
    </details>
  );
}
