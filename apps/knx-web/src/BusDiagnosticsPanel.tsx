/** Switches the diagnostics workspace between telegram monitoring and read-only line scan. */

import { useState } from "react";
import BusMonitorPanel from "./BusMonitorPanel";
import LineScanPanel from "./LineScanPanel";
import { useTranslate } from "./i18n";
import type { ProjectTree } from "./bindings/ProjectTree";

export default function BusDiagnosticsPanel({
  project,
  onTreeUpdate,
}: {
  project: ProjectTree | null;
  onTreeUpdate: (tree: ProjectTree) => void | Promise<void>;
}) {
  const t = useTranslate();
  const [tab, setTab] = useState<"monitor" | "scan">("monitor");
  return (
    <section className="bus-diagnostics-panel">
      <nav className="bus-diagnostics-tabs" aria-label={t("lineScan.diagnosticsTabs")}>
        <button aria-current={tab === "monitor" ? "page" : undefined} onClick={() => setTab("monitor")}>{t("toolbar.busMonitor")}</button>
        <button aria-current={tab === "scan" ? "page" : undefined} onClick={() => setTab("scan")}>{t("lineScan.title")}</button>
      </nav>
      {tab === "monitor" ? <BusMonitorPanel projectOpen={project !== null} /> : <LineScanPanel projectOpen={project !== null} projectRevision={project} onTreeUpdate={onTreeUpdate} />}
    </section>
  );
}
