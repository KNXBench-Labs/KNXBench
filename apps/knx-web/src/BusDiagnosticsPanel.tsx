/** Switches the bus workspace between the monitor, the line scan and download to a device. */

import { useState } from "react";
import BusMonitorPanel from "./BusMonitorPanel";
import DeviceDownloadPanel from "./DeviceDownloadPanel";
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
  const [tab, setTab] = useState<"monitor" | "scan" | "download">("monitor");
  return (
    <section className="bus-diagnostics-panel">
      <nav className="bus-diagnostics-tabs" aria-label={t("lineScan.diagnosticsTabs")}>
        <button aria-current={tab === "monitor" ? "page" : undefined} onClick={() => setTab("monitor")}>{t("toolbar.busMonitor")}</button>
        <button aria-current={tab === "scan" ? "page" : undefined} onClick={() => setTab("scan")}>{t("lineScan.title")}</button>
        <button aria-current={tab === "download" ? "page" : undefined} onClick={() => setTab("download")}>{t("deviceDownload.title")}</button>
      </nav>
      {tab === "monitor" && <BusMonitorPanel projectOpen={project !== null} />}
      {tab === "scan" && <LineScanPanel projectOpen={project !== null} projectRevision={project} onTreeUpdate={onTreeUpdate} />}
      {tab === "download" && <DeviceDownloadPanel project={project} />}
    </section>
  );
}
