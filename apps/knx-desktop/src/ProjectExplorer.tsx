import { useState } from "react";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { AreaNode } from "./bindings/AreaNode";
import type { LineNode } from "./bindings/LineNode";
import type { BuildingNode } from "./bindings/BuildingNode";
import type { DeviceNode } from "./bindings/DeviceNode";

function TreeNode(props: { label: string; children?: React.ReactNode }) {
  const [open, setOpen] = useState(true);
  const hasChildren = props.children !== undefined;
  return (
    <li>
      <span
        className={hasChildren ? "tree-label expandable" : "tree-label"}
        onClick={hasChildren ? () => setOpen(!open) : undefined}
      >
        {hasChildren ? (open ? "▾ " : "▸ ") : ""}
        {props.label}
      </span>
      {hasChildren && open && <ul>{props.children}</ul>}
    </li>
  );
}

function DeviceItem(props: { device: DeviceNode }) {
  const { device } = props;
  const label = device.address ? `${device.address} ${device.name}` : device.name;
  return <TreeNode label={label} />;
}

function LineItem(props: { line: LineNode }) {
  const { line } = props;
  return (
    <TreeNode label={`Line ${line.address}: ${line.name}`}>
      {line.devices.map((d) => (
        <DeviceItem key={d.id} device={d} />
      ))}
    </TreeNode>
  );
}

function AreaItem(props: { area: AreaNode }) {
  const { area } = props;
  return (
    <TreeNode label={`Area ${area.address}: ${area.name}`}>
      {area.lines.map((l) => (
        <LineItem key={l.id} line={l} />
      ))}
    </TreeNode>
  );
}

function BuildingItem(props: { building: BuildingNode }) {
  const { building } = props;
  return (
    <TreeNode label={`${building.name} (${building.kind})`}>
      {building.children.map((c) => (
        <BuildingItem key={c.id} building={c} />
      ))}
      {building.devices.map((d) => (
        <DeviceItem key={d.id} device={d} />
      ))}
    </TreeNode>
  );
}

function InstallationItem(props: { installation: InstallationNode }) {
  const { installation } = props;
  return (
    <TreeNode label={installation.name}>
      <TreeNode label="Topology">
        {installation.topology.map((a) => (
          <AreaItem key={a.id} area={a} />
        ))}
      </TreeNode>
      <TreeNode label="Buildings">
        {installation.buildings.map((b) => (
          <BuildingItem key={b.id} building={b} />
        ))}
      </TreeNode>
      {installation.unassigned.length > 0 && (
        <TreeNode label="Unassigned">
          {installation.unassigned.map((d) => (
            <DeviceItem key={d.id} device={d} />
          ))}
        </TreeNode>
      )}
    </TreeNode>
  );
}

export default function ProjectExplorer(props: { tree: ProjectTree }) {
  const { tree } = props;
  return (
    <div className="project-explorer">
      <ul className="tree-root">
        {tree.installations.map((inst) => (
          <InstallationItem key={inst.id} installation={inst} />
        ))}
      </ul>
      {tree.warnings > 0 && (
        <footer>{tree.warnings} import warning{tree.warnings === 1 ? "" : "s"}</footer>
      )}
    </div>
  );
}
