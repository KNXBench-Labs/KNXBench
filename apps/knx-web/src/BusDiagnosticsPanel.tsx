/** Switches the diagnostics workspace between telegram monitoring and read-only line scan. */

import { useState } from "react";
import BusMonitorPanel from "./BusMonitorPanel";
import LineScanPanel from "./LineScanPanel";
import { useTranslate } from "./i18n";

export default function BusDiagnosticsPanel({ projectOpen }: { projectOpen: boolean }) {
  const t = useTranslate();
  const [tab, setTab] = useState<"monitor" | "scan">("monitor");
  return (
    <section className="bus-diagnostics-panel">
      <nav className="bus-diagnostics-tabs" aria-label={t("lineScan.diagnosticsTabs")}>
        <button aria-current={tab === "monitor" ? "page" : undefined} onClick={() => setTab("monitor")}>{t("toolbar.busMonitor")}</button>
        <button aria-current={tab === "scan" ? "page" : undefined} onClick={() => setTab("scan")}>{t("lineScan.title")}</button>
      </nav>
      {tab === "monitor" ? <BusMonitorPanel projectOpen={projectOpen} /> : <LineScanPanel />}
    </section>
  );
}
