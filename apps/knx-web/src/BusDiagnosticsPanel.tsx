/** Switches the bus workspace between monitor, scan, checks, download, address and Debug tools. */

import { useCallback, useEffect, useRef, useState } from "react";
import type { FlowModel } from "./flowModel";
import type { FlowTarget } from "./flowNavigation";
import AddressProgrammingPanel from "./AddressProgrammingPanel";
import BusActivityHistory from "./BusActivityHistory";
import BusActivityLive from "./BusActivityLive";
import BusMonitorPanel from "./BusMonitorPanel";
import DeviceDownloadPanel from "./DeviceDownloadPanel";
import DeviceInspectionPanel from "./DeviceInspectionPanel";
import LineScanPanel from "./LineScanPanel";
import ServiceControlPanel from "./ServiceControlPanel";
import { useTranslate } from "./i18n";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { DownloadHandoff } from "./downloadHandoff";

export default function BusDiagnosticsPanel({
  project,
  onTreeUpdate,
  projectScope,
  onFlowNavigate,
  active = true,
  downloadHandoff = null,
}: {
  project: ProjectTree | null;
  active?: boolean;
  onTreeUpdate: (tree: ProjectTree) => void | Promise<void>;
  projectScope?: string;
  onFlowNavigate?: (model: FlowModel, target: FlowTarget) => Promise<boolean>;
  /** "Download to device…" from the device views: opens the download tab for it. */
  downloadHandoff?: DownloadHandoff | null;
}) {
  const t = useTranslate();
  const [tab, setTab] = useState<"monitor" | "scan" | "checks" | "download" | "address" | "service" | "live" | "history">("monitor");
  // Each click is one request: it opens the download tab and is handed to the
  // panel until answered, so a later tab switch cannot replay it.
  const [pendingHandoff, setPendingHandoff] = useState<DownloadHandoff | null>(null);
  const seenNonce = useRef<number | null>(null);
  useEffect(() => {
    if (downloadHandoff === null || seenNonce.current === downloadHandoff.nonce) return;
    seenNonce.current = downloadHandoff.nonce;
    setTab("download");
    setPendingHandoff(downloadHandoff);
  }, [downloadHandoff]);
  const handoffHandled = useCallback((nonce: number) => {
    setPendingHandoff((current) => current?.nonce === nonce ? null : current);
  }, []);
  return (
    <section className="bus-diagnostics-panel">
      <nav className="bus-diagnostics-tabs" aria-label={t("lineScan.diagnosticsTabs")}>
        <button aria-current={tab === "monitor" ? "page" : undefined} onClick={() => setTab("monitor")}>{t("toolbar.busMonitor")}</button>
        <button aria-current={tab === "scan" ? "page" : undefined} onClick={() => setTab("scan")}>{t("lineScan.title")}</button>
        <button aria-current={tab === "checks" ? "page" : undefined} onClick={() => setTab("checks")}>{t("deviceChecks.tab")}</button>
        <button aria-current={tab === "download" ? "page" : undefined} onClick={() => setTab("download")}>{t("deviceDownload.title")}</button>
        <button aria-current={tab === "address" ? "page" : undefined} onClick={() => setTab("address")}>{t("addressProgramming.tab")}</button>
        <button aria-current={tab === "service" ? "page" : undefined} onClick={() => setTab("service")}>{t("serviceControl.tab")}</button>
        <button aria-current={tab === "live" ? "page" : undefined} onClick={() => setTab("live")}>{t("activityLive.tab")}</button>
        <button aria-current={tab === "history" ? "page" : undefined} onClick={() => setTab("history")}>{t("activityHistory.title")}</button>
      </nav>
      <div hidden={tab !== "monitor"}><BusMonitorPanel active={active && tab === "monitor"} projectOpen={project !== null} project={project} projectScope={projectScope} onFlowNavigate={onFlowNavigate} /></div>
      {tab === "scan" && <LineScanPanel projectOpen={project !== null} projectRevision={project} onTreeUpdate={onTreeUpdate} />}
      {tab === "checks" && <DeviceInspectionPanel project={project} />}
      {tab === "download" && <DeviceDownloadPanel project={project} handoff={pendingHandoff} onHandoffHandled={handoffHandled} />}
      {tab === "address" && <AddressProgrammingPanel project={project} />}
      {tab === "service" && <ServiceControlPanel projectOpen={project !== null} projectRevision={project} />}
      {tab === "live" && <BusActivityLive />}
      {tab === "history" && <BusActivityHistory />}
    </section>
  );
}
