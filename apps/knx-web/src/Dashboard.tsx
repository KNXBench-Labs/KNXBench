/** Project status: contents, import findings and file details for the open project. */
// apps/knx-desktop/src/Dashboard.tsx
//
// Three labelled sections instead of one undifferentiated list. Every count
// the server sends is still shown as its exact number; layout never hides
// a counter, including zeros and the import counters of a project whose
// historical report is unavailable.
//
// Non-zero import counters open the existing session log, narrowed to the
// matching import-report entries (`App` owns that navigation). A zero is
// plain text, not a control that would lead nowhere.
import type { ReactNode } from "react";
import type { ProjectTree } from "./bindings/ProjectTree";
import { computeStats } from "./dashboardStats";
import ImportDiagnosticsNotice from "./ImportDiagnosticsNotice";
import { importReportState, type ImportFindingSeverity } from "./importFindings";
import ImportEvidencePanel from "./ImportEvidencePanel";
import SourceReportPanel from "./SourceReportPanel";
import { useTranslate } from "./i18n";
import type { MessageKey } from "./messages/en";
import { useDeviceNavigation } from "./DeviceLink";
import "./Dashboard.css";

const STATE_KEYS = {
  errors: "dashboard.importState.errors",
  warnings: "dashboard.importState.warnings",
  clean: "dashboard.importState.clean",
  unavailable: "dashboard.importState.unavailable",
} as const satisfies Record<ReturnType<typeof importReportState>, MessageKey>;

function Row(props: { id?: string; label: string; children: ReactNode; className?: string }) {
  return (
    <div className={`dashboard-row${props.className ? ` ${props.className}` : ""}`}>
      <dt id={props.id}>{props.label}</dt>
      <dd>{props.children}</dd>
    </div>
  );
}

export default function Dashboard(props: {
  tree: ProjectTree;
  /** Opens the session log narrowed to this severity's import findings. */
  onOpenImportFindings?: (severity: ImportFindingSeverity) => void;
}) {
  const t = useTranslate();
  const { tree } = props;
  const stats = computeStats(tree);
  const navigation = useDeviceNavigation();
  const state = importReportState(tree);
  const devices = stats.devicesAssigned + stats.devicesUnassigned;

  function findingValue(severity: ImportFindingSeverity, count: number, labelId: string) {
    if (count === 0 || !props.onOpenImportFindings) {
      return <span className="dashboard-value">{count}</span>;
    }
    const open = props.onOpenImportFindings;
    return (
      <button type="button" className={`dashboard-finding-link dashboard-finding-link--${severity}`}
        aria-describedby={labelId} onClick={() => open(severity)}>
        <span className="dashboard-finding-action">{t("dashboard.showInLog")}</span>
        <span className="dashboard-value">{count}</span>
      </button>
    );
  }

  return (
    <div className="dashboard">
      <h2>{t("dashboard.title")}</h2>
      <div className="dashboard-sections">
        <section className="dashboard-section dashboard-contents" aria-labelledby="dashboard-contents-heading">
          <h3 id="dashboard-contents-heading">{t("dashboard.section.contents")}</h3>
          <dl className="dashboard-stats">
            <Row label={t("dashboard.installations")}><span className="dashboard-value">{stats.installations}</span></Row>
            <Row label={t("dashboard.areas")}><span className="dashboard-value">{stats.areas}</span></Row>
            <Row label={t("dashboard.lines")}><span className="dashboard-value">{stats.lines}</span></Row>
            <Row label={t("dashboard.devices")}>
              {navigation
                ? <button type="button" className="devices-count-link table-select" onClick={navigation.openList}><span className="dashboard-value">{devices}</span></button>
                : <span className="dashboard-value">{devices}</span>}
              {stats.devicesUnassigned > 0 && (
                <span className="dashboard-value-note">{t("dashboard.devicesUnassigned", { count: stats.devicesUnassigned }).trim()}</span>
              )}
            </Row>
            <Row label={t("dashboard.buildingParts")}><span className="dashboard-value">{stats.buildingParts}</span></Row>
            <Row label={t("dashboard.groupAddresses")}><span className="dashboard-value">{stats.groupAddresses}</span></Row>
            <Row label={t("dashboard.comObjects")}><span className="dashboard-value">{stats.comObjects}</span></Row>
          </dl>
        </section>

        <section className="dashboard-section dashboard-import" aria-labelledby="dashboard-import-heading">
          <h3 id="dashboard-import-heading">{t("dashboard.section.import")}</h3>
          <p className={`dashboard-import-state dashboard-import-state--${state}`}>
            <span className="dashboard-import-state-mark" aria-hidden="true" />
            {t(STATE_KEYS[state])}
          </p>
          <dl className="dashboard-stats dashboard-issues">
            <Row id="dashboard-import-errors-label" label={t("dashboard.importErrors")} className={tree.errors > 0 ? "dashboard-row--error" : undefined}>
              {findingValue("error", tree.errors, "dashboard-import-errors-label")}
            </Row>
            <Row id="dashboard-import-warnings-label" label={t("dashboard.importWarnings")} className={tree.warnings > 0 ? "dashboard-row--warning" : undefined}>
              {findingValue("warning", tree.warnings, "dashboard-import-warnings-label")}
            </Row>
          </dl>
          <ImportDiagnosticsNotice available={tree.import_report_available} historical={tree.import_report_historical} />
          <ImportEvidencePanel tree={tree} />
          <SourceReportPanel tree={tree} />
        </section>

        <section className="dashboard-section dashboard-file" aria-labelledby="dashboard-file-heading">
          <h3 id="dashboard-file-heading">{t("dashboard.section.file")}</h3>
          <dl className="dashboard-stats">
            <Row label={t("dashboard.schemaVersion")}><span className="dashboard-value">{tree.schema_version}</span></Row>
          </dl>
        </section>
      </div>
    </div>
  );
}
