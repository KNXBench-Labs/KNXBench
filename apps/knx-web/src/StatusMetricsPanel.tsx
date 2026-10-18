import { useEffect, useState } from "react";
import * as api from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import { useTranslate } from "./i18n";
import { useUiLanguage } from "./uiLanguage";
import { admitStatusMetrics, formatDecimalMb, type StatusMetrics } from "./statusMetrics";

type Result = { identity: string; requestKey: string; state: "ready"; metrics: StatusMetrics; projectMatches: boolean }
  | { identity: string; requestKey: string; state: "error" | "stale" };

export default function StatusMetricsPanel({ tree }: { tree: ProjectTree }) {
  const t = useTranslate();
  const [language] = useUiLanguage();
  const identity = `${tree.server_incarnation ?? ""}:${tree.project_incarnation ?? ""}`;
  const requestKey = `${identity}:${tree.snapshot_revision ?? ""}`;
  const [latest, setLatest] = useState<Result | null>(null);

  useEffect(() => {
    let active = true;
    void api.statusMetrics().then(value => {
      if (!active) return;
      try {
        const admitted = admitStatusMetrics(value, tree);
        setLatest(admitted
          ? { identity, requestKey, state: "ready", ...admitted }
          : { identity, requestKey, state: "stale" });
      } catch {
        setLatest({ identity, requestKey, state: "error" });
      }
    }).catch(() => { if (active) setLatest({ identity, requestKey, state: "error" }); });
    return () => { active = false; };
  }, [requestKey]);

  // Keep measured values while a revision of the same project is fetched.
  // A different server or project lifetime never borrows those values.
  const result = latest?.identity === identity ? latest : null;
  const busy = !result || result.requestKey !== requestKey;
  const metrics = result?.state === "ready" ? result.metrics : null;
  const stale = result?.state === "stale" || (result?.state === "ready" && !result.projectMatches && !busy);
  const size = (bytes: number) => formatDecimalMb(bytes, language);
  const projectValue = !result ? t("dashboard.metrics.loading") : result.state === "error" ? t("dashboard.metrics.error")
    : result.state === "stale" || (stale && !busy) ? "—"
    : metrics!.project.state === "notSavedYet" ? t("dashboard.metrics.notSavedYet")
    : metrics!.project.state === "available" ? size(metrics!.project.currentOnDiskBytes!)
    : metrics!.project.state === "unavailable" ? t("dashboard.metrics.projectUnavailable") : t("dashboard.metrics.error");
  const catalogueValue = !result ? t("dashboard.metrics.loading") : result.state === "error" ? t("dashboard.metrics.catalogueError")
    : result.state === "stale" ? "—"
    : metrics!.catalogue.state === "available" ? size(metrics!.catalogue.bytes!)
    : metrics!.catalogue.state === "unavailable" ? t("dashboard.metrics.catalogueUnavailable") : t("dashboard.metrics.catalogueError");
  const countValue = (count: number | undefined) => !result ? t("dashboard.metrics.loading") : result.state === "error" ? t("dashboard.metrics.countError")
    : result.state === "stale" ? "—"
    : metrics!.catalogue.state === "available" ? String(count)
    : metrics!.catalogue.state === "unavailable" ? t("dashboard.metrics.catalogueUnavailable") : t("dashboard.metrics.countError");

  return <div aria-busy={busy}>
    {stale && <p role="status">{t("dashboard.metrics.stale")}</p>}
    <div className="dashboard-row"><dt>{t("dashboard.metrics.savedFile")}</dt><dd><span className="dashboard-value">{projectValue}</span></dd></div>
    <div className="dashboard-row"><dt>{t("dashboard.metrics.unsavedChanges")}</dt><dd><span className="dashboard-value">{t(tree.is_modified ? "dashboard.metrics.yes" : "dashboard.metrics.no")}</span></dd></div>
    <div className="dashboard-row"><dt>{t("dashboard.metrics.databaseSize")}</dt><dd><span className="dashboard-value">{catalogueValue}</span></dd></div>
    <div className="dashboard-row"><dt>{t("dashboard.metrics.manufacturers")}</dt><dd><span className="dashboard-value">{countValue(metrics?.catalogue.counts?.manufacturers)}</span></dd></div>
    <div className="dashboard-row"><dt>{t("dashboard.metrics.products")}</dt><dd><span className="dashboard-value">{countValue(metrics?.catalogue.counts?.products)}</span></dd></div>
    <div className="dashboard-row"><dt>{t("dashboard.metrics.programs")}</dt><dd><span className="dashboard-value">{countValue(metrics?.catalogue.counts?.applicationPrograms)}</span></dd></div>
  </div>;
}
