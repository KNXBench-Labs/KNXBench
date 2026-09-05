import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { DeviceDetail } from "./bindings/DeviceDetail";
import type { ComObjectNode } from "./bindings/ComObjectNode";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import type { BuildingNode } from "./bindings/BuildingNode";
import type { Selection } from "./selection";
import { findBuildingPart, findGroupAddress } from "./treeUtils";

function AddressField(props: { detail: DeviceDetail; onApplied: (tree: ProjectTree) => void }) {
  const { detail, onApplied } = props;
  const [value, setValue] = useState(detail.address ?? "");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setValue(detail.address ?? "");
    setError(null);
  }, [detail.address]);

  async function apply() {
    const current = detail.address ?? "";
    if (value === current) return;
    setError(null);
    try {
      const tree = await invoke<ProjectTree>("set_individual_address", {
        deviceId: detail.id,
        address: value === "" ? null : value,
      });
      onApplied(tree);
    } catch (e) {
      setError(String(e));
      setValue(current);
    }
  }

  return (
    <label className="inspector-field">
      Address
      <input
        value={value}
        placeholder="1.1.1"
        onChange={(e) => setValue(e.target.value)}
        onBlur={apply}
        onKeyDown={(e) => {
          if (e.key === "Enter") (e.target as HTMLInputElement).blur();
        }}
      />
      {error && <span className="field-error">{error}</span>}
    </label>
  );
}

function DptField(props: { com: ComObjectNode; onApplied: (tree: ProjectTree) => void }) {
  const { com, onApplied } = props;
  const [value, setValue] = useState(com.dpt ?? "");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setValue(com.dpt ?? "");
    setError(null);
  }, [com.dpt]);

  async function apply() {
    const current = com.dpt ?? "";
    if (value === current) return;
    setError(null);
    try {
      const tree = await invoke<ProjectTree>("set_com_object_dpt", {
        comObjectId: com.id,
        dpt: value === "" ? null : value,
      });
      onApplied(tree);
    } catch (e) {
      setError(String(e));
      setValue(current);
    }
  }

  return (
    <label className="inspector-field">
      DPT
      <input
        value={value}
        placeholder="DPST-9-1"
        onChange={(e) => setValue(e.target.value)}
        onBlur={apply}
        onKeyDown={(e) => {
          if (e.key === "Enter") (e.target as HTMLInputElement).blur();
        }}
      />
      {error && <span className="field-error">{error}</span>}
    </label>
  );
}

function DeviceInspector(props: { detail: DeviceDetail; onApplied: (tree: ProjectTree) => void }) {
  const { detail, onApplied } = props;
  return (
    <div className="inspector">
      <h2>{detail.name}</h2>
      {detail.description && <p className="inspector-description">{detail.description}</p>}
      <AddressField detail={detail} onApplied={onApplied} />
      <h3>Communication objects</h3>
      <ul className="com-object-list">
        {detail.com_objects.map((com) => (
          <li key={com.id}>
            <span className="com-object-label">
              {com.number}: {com.name ?? "(unnamed)"}
            </span>
            <DptField com={com} onApplied={onApplied} />
            {com.dpt_layer && <span className="provenance-badge">{com.dpt_layer}</span>}
          </li>
        ))}
      </ul>
    </div>
  );
}

function GroupAddressInspector(props: {
  ga: GroupAddressNode;
  onDeleted: (tree: ProjectTree) => void;
}) {
  const { ga, onDeleted } = props;
  const [error, setError] = useState<string | null>(null);

  async function remove() {
    setError(null);
    try {
      const tree = await invoke<ProjectTree>("delete_group_address", { id: ga.id });
      onDeleted(tree);
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <div className="inspector">
      <h2>{ga.name}</h2>
      <p className="inspector-address">{ga.address}</p>
      <button onClick={remove}>Delete</button>
      {error && <span className="field-error">{error}</span>}
    </div>
  );
}

function BuildingPartInspector(props: { node: BuildingNode; path: string }) {
  const { node, path } = props;
  return (
    <div className="inspector">
      <h2>{node.name}</h2>
      <p className="inspector-description">{node.kind}</p>
      <p className="inspector-path">{path}</p>
      <p>
        {node.devices.length} device{node.devices.length === 1 ? "" : "s"},{" "}
        {node.children.length} child part{node.children.length === 1 ? "" : "s"}
      </p>
    </div>
  );
}

// `deviceDetail` is `null` both before the async `device_detail` fetch
// lands and if it errored (App.tsx clears it either way) — in either case
// there is nothing to show yet, so this renders nothing rather than a
// half-populated panel. Group-address and building-part selections have
// no such gap: both resolve synchronously from `tree`, which is always
// already loaded by the time Inspector can render at all.
export default function Inspector(props: {
  selection: Selection;
  tree: ProjectTree;
  deviceDetail: DeviceDetail | null;
  onApplied: (tree: ProjectTree) => void;
  onDeleted: (tree: ProjectTree) => void;
}) {
  const { selection, tree, deviceDetail, onApplied, onDeleted } = props;

  if (selection.kind === "device") {
    if (!deviceDetail) return null;
    return <DeviceInspector detail={deviceDetail} onApplied={onApplied} />;
  }

  if (selection.kind === "group_address") {
    const ga = findGroupAddress(tree, selection.id);
    if (!ga) return null;
    return <GroupAddressInspector ga={ga} onDeleted={onDeleted} />;
  }

  const found = findBuildingPart(tree, selection.id);
  if (!found) return null;
  return <BuildingPartInspector node={found.node} path={found.path} />;
}
