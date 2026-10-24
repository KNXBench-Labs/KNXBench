/** Diagnostics-tab buttons opening the existing analysis and support dialogs for one device. */
import { useEffect, useId, useState } from "react";
import { createPortal } from "react-dom";
import { useTranslate } from "./i18n";
import type { ProjectTree } from "./bindings/ProjectTree";
import { projectContextKey } from "./treeUtils";
import { DiagnosticExportDialog } from "./DiagnosticExportButton";
import { SupportDialog, supportProjectBinding } from "./UniversalSupportButton";
import "./deviceDiagnosticsEntry.css";

type OpenDialog = "analyse" | "report" | null;

/**
 * Plain buttons, never File-menu entries: the dialogs keep their own consent and review flows.
 * This tab closes both dialogs on a device or project change, and the report on any revision.
 * The File-menu analysis entry has no such close-on-project-change binding. Here, a revision
 * keeps the analysis dialog's own stale-review state and must not cut a running read short.
 */
export default function DeviceDiagnosticsEntry({ project, deviceId }: { project: ProjectTree | null; deviceId: number }) {
  const t = useTranslate();
  const headingId = useId();
  const [open, setOpen] = useState<OpenDialog>(null);
  const reportBinding = supportProjectBinding(project);
  const analysisBinding = project ? projectContextKey(project) : "no-project";
  useEffect(() => setOpen((current) => (current === "report" ? null : current)), [reportBinding]);
  useEffect(() => setOpen(null), [analysisBinding, deviceId]);
  const close = () => setOpen(null);
  return <section className="device-diagnostics-entry" aria-labelledby={headingId}>
    <h3 id={headingId}>{t("deviceDiagnostics.heading")}</h3>
    <div className="device-diagnostics-entry-actions">
      <button type="button" data-device-analyse disabled={!project} onClick={() => setOpen("analyse")}>{t("deviceDiagnostics.analyse")}</button>
      <button type="button" data-device-report disabled={!project} onClick={() => setOpen("report")}>{t("deviceDiagnostics.report")}</button>
    </div>
    <p className="inspector-description">{t("deviceDiagnostics.hint", { analysis: t("diagnostics.title"), report: t("support.title") })}</p>
    {open === "analyse" && project && createPortal(<DiagnosticExportDialog project={project} initialDevice={deviceId} close={close} />, document.body)}
    {open === "report" && project && createPortal(<SupportDialog project={project} initialDevice={deviceId} close={close} />, document.body)}
  </section>;
}
