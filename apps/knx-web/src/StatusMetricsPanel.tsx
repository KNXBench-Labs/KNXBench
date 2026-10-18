import { useEffect, useState } from "react";
import * as api from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import { useTranslate } from "./i18n";
import { useUiLanguage } from "./uiLanguage";
import { formatDecimalMb, type StatusMetrics } from "./statusMetrics";

export default function StatusMetricsPanel({ tree }: { tree: ProjectTree }) {
  const t = useTranslate();
  const [language] = useUiLanguage();
  const requestKey = `${tree.server_incarnation ?? ""}:${tree.project_incarnation ?? ""}:${tree.snapshot_revision ?? ""}`;
  const [latest, setLatest] = useState<({ key: string; state: "ready"; metrics: StatusMetrics } | { key: string; state: "error" | "stale" }) | null>(null);

  useEffect(() => {
    let active = true;
    setLatest(null);
    void api.statusMetrics().then(metrics => {
      if (!active) return;
      const sameProject = tree.project_incarnation == null || metrics.project.projectIncarnation === tree.project_incarnation;
      const sameRevision = tree.snapshot_revision == null || metrics.project.snapshotRevision === tree.snapshot_revision;
      setLatest(sameProject && sameRevision
        ? { key: requestKey, state: "ready", metrics }
        : { key: requestKey, state: "stale" });
    }).catch(() => { if (active) setLatest({ key: requestKey, state: "error" }); });
    return () => { active = false; };
  }, [requestKey]);

  // A new tree can render before its effect runs; never show the old file
  // measurement even for that frame.
  const result = latest?.key === requestKey ? latest : null;
  const metrics = result?.state === "ready" ? result.metrics : null;
  const size = (bytes: number | null) => bytes === null ? t("dashboard.metrics.error") : formatDecimalMb(bytes, language);
  const projectValue = !result ? t("dashboard.metrics.loading") : result.state === "error" ? t("dashboard.metrics.error")
    : result.state === "stale" ? t("dashboard.metrics.stale")
    : metrics!.project.state === "notSavedYet" ? t("dashboard.metrics.notSavedYet")
    : metrics!.project.state === "available" ? size(metrics!.project.bytes)
    : metrics!.project.state === "unavailable" ? t("dashboard.metrics.projectUnavailable") : t("dashboard.metrics.error");
  const catalogueValue = !result ? t("dashboard.metrics.loading") : result.state === "error" ? t("dashboard.metrics.error")
    : result.state === "stale" ? t("dashboard.metrics.stale")
    : metrics!.catalogue.state === "available" ? size(metrics!.catalogue.bytes)
    : metrics!.catalogue.state === "unavailable" ? t("dashboard.metrics.catalogueUnavailable") : t("dashboard.metrics.catalogueError");
  const countValue = (count: number | undefined) => !result ? t("dashboard.metrics.loading") : result.state === "error" ? t("dashboard.metrics.error")
    : result.state === "stale" ? t("dashboard.metrics.stale")
    : metrics!.catalogue.state === "available" && count !== undefined ? String(count)
    : metrics!.catalogue.state === "unavailable" ? t("dashboard.metrics.catalogueUnavailable") : t("dashboard.metrics.catalogueError");

  return <>
    <div className="dashboard-row"><dt>{t("dashboard.metrics.savedFile")}</dt><dd><span className="dashboard-value">{projectValue}</span></dd></div>
    <div className="dashboard-row"><dt>{t("dashboard.metrics.unsavedChanges")}</dt><dd><span className="dashboard-value">{t(tree.is_modified ? "dashboard.metrics.yes" : "dashboard.metrics.no")}</span></dd></div>
    <div className="dashboard-row"><dt>{t("dashboard.metrics.databaseSize")}</dt><dd><span className="dashboard-value">{catalogueValue}</span></dd></div>
    <div className="dashboard-row"><dt>{t("dashboard.metrics.manufacturers")}</dt><dd><span className="dashboard-value">{countValue(metrics?.catalogue.counts?.manufacturers)}</span></dd></div>
    <div className="dashboard-row"><dt>{t("dashboard.metrics.products")}</dt><dd><span className="dashboard-value">{countValue(metrics?.catalogue.counts?.products)}</span></dd></div>
    <div className="dashboard-row"><dt>{t("dashboard.metrics.programs")}</dt><dd><span className="dashboard-value">{countValue(metrics?.catalogue.counts?.applicationPrograms)}</span></dd></div>
  </>;
}
