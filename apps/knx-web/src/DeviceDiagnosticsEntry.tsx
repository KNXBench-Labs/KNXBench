/** Diagnostics-tab buttons: analyse this device, or report a problem with it preselected. */
import { useEffect, useId, useState } from "react";
import { createPortal } from "react-dom";
import { useTranslate } from "./i18n";
import type { ProjectTree } from "./bindings/ProjectTree";
import { projectContextKey } from "./treeUtils";
import { DiagnosticExportDialog } from "./DiagnosticExportButton";
import { useReportProblem } from "./ReportProblemEntry";
import "./deviceDiagnosticsEntry.css";

/**
 * Plain buttons, never File-menu entries: the dialogs keep their own consent and review flows.
 * This tab closes the analysis dialog on a device or project change; a revision keeps the
 * dialog's own stale-review state and must not cut a running read short. The report button
 * opens the one Report problem wizard (DIAG-03) for reading/diagnostics with this device chosen.
 */
export default function DeviceDiagnosticsEntry({ project, deviceId }: { project: ProjectTree | null; deviceId: number }) {
  const t = useTranslate();
  const headingId = useId();
  const report = useReportProblem();
  const [analysing, setAnalysing] = useState(false);
  const analysisBinding = project ? projectContextKey(project) : "no-project";
  useEffect(() => setAnalysing(false), [analysisBinding, deviceId]);
  return <section className="device-diagnostics-entry" aria-labelledby={headingId}>
    <h3 id={headingId}>{t("deviceDiagnostics.heading")}</h3>
    <div className="device-diagnostics-entry-actions">
      <button type="button" data-device-analyse disabled={!project} onClick={() => setAnalysing(true)}>{t("deviceDiagnostics.analyse")}</button>
      {report && <button type="button" data-device-report disabled={!project}
        onClick={() => report({ problemType: "deviceReading", deviceId, operation: { selection: "none" } })}>{t("deviceDiagnostics.report")}</button>}
    </div>
    <p className="inspector-description">{t("deviceDiagnostics.hint", { analysis: t("diagnostics.title"), report: t("reportProblem.title") })}</p>
    {analysing && project && createPortal(<DiagnosticExportDialog project={project} initialDevice={deviceId} close={() => setAnalysing(false)} />, document.body)}
  </section>;
}
