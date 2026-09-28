/** Switches the bus workspace between monitor, line scan, download and address programming. */

import { useState } from "react";
import AddressProgrammingPanel from "./AddressProgrammingPanel";
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
  const [tab, setTab] = useState<"monitor" | "scan" | "download" | "address">("monitor");
  return (
    <section className="bus-diagnostics-panel">
      <nav className="bus-diagnostics-tabs" aria-label={t("lineScan.diagnosticsTabs")}>
        <button aria-current={tab === "monitor" ? "page" : undefined} onClick={() => setTab("monitor")}>{t("toolbar.busMonitor")}</button>
        <button aria-current={tab === "scan" ? "page" : undefined} onClick={() => setTab("scan")}>{t("lineScan.title")}</button>
        <button aria-current={tab === "download" ? "page" : undefined} onClick={() => setTab("download")}>{t("deviceDownload.title")}</button>
        <button aria-current={tab === "address" ? "page" : undefined} onClick={() => setTab("address")}>{t("addressProgramming.tab")}</button>
      </nav>
      {tab === "monitor" && <BusMonitorPanel projectOpen={project !== null} />}
      {tab === "scan" && <LineScanPanel projectOpen={project !== null} projectRevision={project} onTreeUpdate={onTreeUpdate} />}
      {tab === "download" && <DeviceDownloadPanel project={project} />}
      {tab === "address" && <AddressProgrammingPanel project={project} />}
    </section>
  );
}
