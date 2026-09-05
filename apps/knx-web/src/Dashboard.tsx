// apps/knx-desktop/src/Dashboard.tsx
import type { ProjectTree } from "./bindings/ProjectTree";
import { computeStats } from "./dashboardStats";

export default function Dashboard(props: { tree: ProjectTree }) {
  const stats = computeStats(props.tree);
  return (
    <div className="dashboard">
      <h2>Project status</h2>
      <dl className="dashboard-stats">
        <dt>Schema version</dt>
        <dd>{props.tree.schema_version}</dd>
        <dt>Installations</dt>
        <dd>{stats.installations}</dd>
        <dt>Areas</dt>
        <dd>{stats.areas}</dd>
        <dt>Lines</dt>
        <dd>{stats.lines}</dd>
        <dt>Devices</dt>
        <dd>
          {stats.devicesAssigned + stats.devicesUnassigned}
          {stats.devicesUnassigned > 0 && ` (${stats.devicesUnassigned} unassigned)`}
        </dd>
        <dt>Group addresses</dt>
        <dd>{stats.groupAddresses}</dd>
        <dt>Building parts</dt>
        <dd>{stats.buildingParts}</dd>
        <dt>Communication objects</dt>
        <dd>{stats.comObjects}</dd>
      </dl>
      {(props.tree.errors > 0 || props.tree.warnings > 0) && (
        <dl className="dashboard-stats dashboard-issues">
          {props.tree.errors > 0 && (
            <>
              <dt>Import errors</dt>
              <dd className="dashboard-errors">{props.tree.errors}</dd>
            </>
          )}
          {props.tree.warnings > 0 && (
            <>
              <dt>Import warnings</dt>
              <dd>{props.tree.warnings}</dd>
            </>
          )}
        </dl>
      )}
    </div>
  );
}
