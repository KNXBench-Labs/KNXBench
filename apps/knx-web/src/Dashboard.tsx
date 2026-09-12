/** Overview panel showing schema version and entity counts for the open project. */
// apps/knx-desktop/src/Dashboard.tsx
import type { ProjectTree } from "./bindings/ProjectTree";
import { computeStats } from "./dashboardStats";
import { useTranslate } from "./i18n";

export default function Dashboard(props: { tree: ProjectTree }) {
  const t = useTranslate();
  const stats = computeStats(props.tree);
  return (
    <div className="dashboard">
      <h2>{t("dashboard.title")}</h2>
      <dl className="dashboard-stats">
        <dt>{t("dashboard.schemaVersion")}</dt>
        <dd>{props.tree.schema_version}</dd>
        <dt>{t("dashboard.installations")}</dt>
        <dd>{stats.installations}</dd>
        <dt>{t("dashboard.areas")}</dt>
        <dd>{stats.areas}</dd>
        <dt>{t("dashboard.lines")}</dt>
        <dd>{stats.lines}</dd>
        <dt>{t("dashboard.devices")}</dt>
        <dd>
          {stats.devicesAssigned + stats.devicesUnassigned}
          {stats.devicesUnassigned > 0 &&
            t("dashboard.devicesUnassigned", { count: stats.devicesUnassigned })}
        </dd>
        <dt>{t("dashboard.groupAddresses")}</dt>
        <dd>{stats.groupAddresses}</dd>
        <dt>{t("dashboard.buildingParts")}</dt>
        <dd>{stats.buildingParts}</dd>
        <dt>{t("dashboard.comObjects")}</dt>
        <dd>{stats.comObjects}</dd>
      </dl>
      {(props.tree.errors > 0 || props.tree.warnings > 0) && (
        <dl className="dashboard-stats dashboard-issues">
          {props.tree.errors > 0 && (
            <>
              <dt>{t("dashboard.importErrors")}</dt>
              <dd className="dashboard-errors">{props.tree.errors}</dd>
            </>
          )}
          {props.tree.warnings > 0 && (
            <>
              <dt>{t("dashboard.importWarnings")}</dt>
              <dd>{props.tree.warnings}</dd>
            </>
          )}
        </dl>
      )}
    </div>
  );
}
