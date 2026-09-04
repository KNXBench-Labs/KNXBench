import { useState } from "react";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { AreaNode } from "./bindings/AreaNode";
import type { LineNode } from "./bindings/LineNode";
import type { BuildingNode } from "./bindings/BuildingNode";
import type { DeviceNode } from "./bindings/DeviceNode";

function TreeNode(props: {
  label: string;
  children?: React.ReactNode;
  selected?: boolean;
  onClick?: () => void;
}) {
  const [open, setOpen] = useState(true);
  const hasChildren = props.children !== undefined;
  const classes = ["tree-label"];
  if (hasChildren) classes.push("expandable");
  if (props.selected) classes.push("selected");
  const handleClick = hasChildren ? () => setOpen(!open) : props.onClick;
  return (
    <li>
      <span className={classes.join(" ")} onClick={handleClick}>
        {hasChildren ? (open ? "▾ " : "▸ ") : ""}
        {props.label}
      </span>
      {hasChildren && open && <ul>{props.children}</ul>}
    </li>
  );
}

type SelectionProps = {
  selectedId: number | null;
  onSelectDevice: (id: number) => void;
};

function DeviceItem(props: { device: DeviceNode } & SelectionProps) {
  const { device, selectedId, onSelectDevice } = props;
  const label = device.address ? `${device.address} ${device.name}` : device.name;
  return (
    <TreeNode
      label={label}
      selected={device.id === selectedId}
      onClick={() => onSelectDevice(device.id)}
    />
  );
}

function LineItem(props: { line: LineNode } & SelectionProps) {
  const { line, selectedId, onSelectDevice } = props;
  return (
    <TreeNode label={`Line ${line.address}: ${line.name}`}>
      {line.devices.map((d) => (
        <DeviceItem key={d.id} device={d} selectedId={selectedId} onSelectDevice={onSelectDevice} />
      ))}
    </TreeNode>
  );
}

function AreaItem(props: { area: AreaNode } & SelectionProps) {
  const { area, selectedId, onSelectDevice } = props;
  return (
    <TreeNode label={`Area ${area.address}: ${area.name}`}>
      {area.lines.map((l) => (
        <LineItem key={l.id} line={l} selectedId={selectedId} onSelectDevice={onSelectDevice} />
      ))}
    </TreeNode>
  );
}

function BuildingItem(props: { building: BuildingNode } & SelectionProps) {
  const { building, selectedId, onSelectDevice } = props;
  return (
    <TreeNode label={`${building.name} (${building.kind})`}>
      {building.children.map((c) => (
        <BuildingItem key={c.id} building={c} selectedId={selectedId} onSelectDevice={onSelectDevice} />
      ))}
      {building.devices.map((d) => (
        <DeviceItem key={d.id} device={d} selectedId={selectedId} onSelectDevice={onSelectDevice} />
      ))}
    </TreeNode>
  );
}

function InstallationItem(props: { installation: InstallationNode } & SelectionProps) {
  const { installation, selectedId, onSelectDevice } = props;
  return (
    <TreeNode label={installation.name}>
      <TreeNode label="Topology">
        {installation.topology.map((a) => (
          <AreaItem key={a.id} area={a} selectedId={selectedId} onSelectDevice={onSelectDevice} />
        ))}
      </TreeNode>
      <TreeNode label="Buildings">
        {installation.buildings.map((b) => (
          <BuildingItem key={b.id} building={b} selectedId={selectedId} onSelectDevice={onSelectDevice} />
        ))}
      </TreeNode>
      {installation.unassigned.length > 0 && (
        <TreeNode label="Unassigned">
          {installation.unassigned.map((d) => (
            <DeviceItem key={d.id} device={d} selectedId={selectedId} onSelectDevice={onSelectDevice} />
          ))}
        </TreeNode>
      )}
    </TreeNode>
  );
}

export default function ProjectExplorer(props: { tree: ProjectTree } & SelectionProps) {
  const { tree, selectedId, onSelectDevice } = props;
  return (
    <div className="project-explorer">
      <ul className="tree-root">
        {tree.installations.map((inst) => (
          <InstallationItem
            key={inst.id}
            installation={inst}
            selectedId={selectedId}
            onSelectDevice={onSelectDevice}
          />
        ))}
      </ul>
      {(tree.errors > 0 || tree.warnings > 0) && (
        <footer>
          {tree.errors > 0 && (
            <div className="import-errors">
              {tree.errors} import error{tree.errors === 1 ? "" : "s"} — data may be missing or
              incorrect
            </div>
          )}
          {tree.warnings > 0 && (
            <div className="import-warnings">
              {tree.warnings} import warning{tree.warnings === 1 ? "" : "s"}
            </div>
          )}
        </footer>
      )}
    </div>
  );
}
