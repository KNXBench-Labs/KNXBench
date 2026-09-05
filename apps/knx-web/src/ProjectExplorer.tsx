import { useState } from "react";
import * as api from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { InstallationNode } from "./bindings/InstallationNode";
import type { AreaNode } from "./bindings/AreaNode";
import type { LineNode } from "./bindings/LineNode";
import type { BuildingNode } from "./bindings/BuildingNode";
import type { DeviceNode } from "./bindings/DeviceNode";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { Selection } from "./selection";

function TreeNode(props: {
  label: string;
  children?: React.ReactNode;
  selected?: boolean;
  onSelect?: () => void;
}) {
  const [open, setOpen] = useState(true);
  const hasChildren = props.children !== undefined;
  const labelClasses = ["tree-label"];
  if (props.selected) labelClasses.push("selected");
  if (props.onSelect) labelClasses.push("selectable");
  // A node with children but no `onSelect` (every non-leaf label except
  // BuildingItem) keeps the old whole-row-toggles behavior. A node with
  // `onSelect` (DeviceItem, BuildingItem) selects on the label instead —
  // BuildingItem still gets its own dedicated chevron to expand/collapse.
  const labelClick = props.onSelect ?? (hasChildren ? () => setOpen(!open) : undefined);
  return (
    <li>
      <span className="tree-row">
        {hasChildren && (
          <span className="tree-toggle" onClick={() => setOpen(!open)}>
            {open ? "▾" : "▸"}
          </span>
        )}
        <span className={labelClasses.join(" ")} onClick={labelClick}>
          {props.label}
        </span>
      </span>
      {hasChildren && open && <ul>{props.children}</ul>}
    </li>
  );
}

type SelectionProps = {
  selection: Selection | null;
  onSelect: (sel: Selection) => void;
};

function DeviceItem(props: { device: DeviceNode } & SelectionProps) {
  const { device, selection, onSelect } = props;
  const label = device.address ? `${device.address} ${device.name}` : device.name;
  return (
    <TreeNode
      label={label}
      selected={selection?.kind === "device" && selection.id === device.id}
      onSelect={() => onSelect({ kind: "device", id: device.id })}
    />
  );
}

function LineItem(props: { line: LineNode } & SelectionProps) {
  const { line, selection, onSelect } = props;
  return (
    <TreeNode label={`Line ${line.address}: ${line.name}`}>
      {line.devices.map((d) => (
        <DeviceItem key={d.id} device={d} selection={selection} onSelect={onSelect} />
      ))}
    </TreeNode>
  );
}

function AreaItem(props: { area: AreaNode } & SelectionProps) {
  const { area, selection, onSelect } = props;
  return (
    <TreeNode label={`Area ${area.address}: ${area.name}`}>
      {area.lines.map((l) => (
        <LineItem key={l.id} line={l} selection={selection} onSelect={onSelect} />
      ))}
    </TreeNode>
  );
}

function BuildingItem(props: { building: BuildingNode } & SelectionProps) {
  const { building, selection, onSelect } = props;
  return (
    <TreeNode
      label={`${building.name} (${building.kind})`}
      selected={selection?.kind === "building_part" && selection.id === building.id}
      onSelect={() => onSelect({ kind: "building_part", id: building.id })}
    >
      {building.children.map((c) => (
        <BuildingItem key={c.id} building={c} selection={selection} onSelect={onSelect} />
      ))}
      {building.devices.map((d) => (
        <DeviceItem key={d.id} device={d} selection={selection} onSelect={onSelect} />
      ))}
    </TreeNode>
  );
}

function GroupAddressItem(props: { ga: GroupAddressNode } & SelectionProps) {
  const { ga, selection, onSelect } = props;
  return (
    <TreeNode
      label={`${ga.address} ${ga.name}`}
      selected={selection?.kind === "group_address" && selection.id === ga.id}
      onSelect={() => onSelect({ kind: "group_address", id: ga.id })}
    />
  );
}

// The only affordance in the tree that creates a domain object rather than
// selecting one — kept as an inline row rather than a dialog, the same way
// `AddressField`/`DptField` (Inspector.tsx) edit inline rather than popping
// a modal. Only rendered under the first installation (`InstallationItem`'s
// `isFirst`): `Command::apply` only ever targets `installations[0]`
// (command.rs), so this is the only installation the affordance could
// honestly promise to create into.
function NewGroupAddressRow(props: { onCreated: (tree: ProjectTree) => void }) {
  const { onCreated } = props;
  const [address, setAddress] = useState("");
  const [name, setName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const canCreate = address.trim() !== "" && name.trim() !== "";

  async function create() {
    if (!canCreate) return;
    setError(null);
    try {
      const tree = await api.createGroupAddress(name, address);
      onCreated(tree);
      setAddress("");
      setName("");
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <li className="tree-new-row">
      <input
        value={address}
        placeholder="1/1/1"
        onChange={(e) => setAddress(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") void create();
        }}
      />
      <input
        value={name}
        placeholder="New group address"
        onChange={(e) => setName(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") void create();
        }}
      />
      <button onClick={create} disabled={!canCreate}>
        Add
      </button>
      {error && <span className="field-error">{error}</span>}
    </li>
  );
}

function InstallationItem(
  props: { installation: InstallationNode; isFirst: boolean; onTreeUpdate: (tree: ProjectTree) => void } & SelectionProps,
) {
  const { installation, isFirst, onTreeUpdate, selection, onSelect } = props;
  return (
    <TreeNode label={installation.name}>
      <TreeNode label="Topology">
        {installation.topology.map((a) => (
          <AreaItem key={a.id} area={a} selection={selection} onSelect={onSelect} />
        ))}
      </TreeNode>
      <TreeNode label="Buildings">
        {installation.buildings.map((b) => (
          <BuildingItem key={b.id} building={b} selection={selection} onSelect={onSelect} />
        ))}
      </TreeNode>
      {installation.unassigned.length > 0 && (
        <TreeNode label="Unassigned">
          {installation.unassigned.map((d) => (
            <DeviceItem key={d.id} device={d} selection={selection} onSelect={onSelect} />
          ))}
        </TreeNode>
      )}
      <TreeNode label="Group Addresses">
        {installation.group_addresses.map((ga) => (
          <GroupAddressItem key={ga.id} ga={ga} selection={selection} onSelect={onSelect} />
        ))}
        {isFirst && <NewGroupAddressRow onCreated={onTreeUpdate} />}
      </TreeNode>
    </TreeNode>
  );
}

export default function ProjectExplorer(
  props: { tree: ProjectTree; onTreeUpdate: (tree: ProjectTree) => void } & SelectionProps,
) {
  const { tree, onTreeUpdate, selection, onSelect } = props;
  return (
    <div className="project-explorer">
      <ul className="tree-root">
        {tree.installations.map((inst, idx) => (
          <InstallationItem
            key={inst.id}
            installation={inst}
            isFirst={idx === 0}
            onTreeUpdate={onTreeUpdate}
            selection={selection}
            onSelect={onSelect}
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
