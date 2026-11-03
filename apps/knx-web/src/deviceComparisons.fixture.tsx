/** Supplies explicit comparison owners to standalone editor and table regression fixtures. */
import type { ComponentProps } from "react";
import InspectorView, { DeviceWorkspace as DeviceEditor } from "./Inspector";
import DeviceTable from "./DevicesWorkspace";
import DeviceComparisonDialog from "./DeviceComparisonDialog";
import { useDeviceComparisons } from "./useDeviceComparisons";
import type { ProjectTree } from "./bindings/ProjectTree";

export function DeviceWorkspace(props: Omit<ComponentProps<typeof DeviceEditor>, "comparisons">) {
  const comparisons = useDeviceComparisons(props.tree);
  return <DeviceEditor {...props} comparisons={comparisons} />;
}
export function DevicesWorkspace(props: Omit<ComponentProps<typeof DeviceTable>, "comparisons"> & {
  onTreeUpdate?: (tree: ProjectTree) => void | Promise<void>;
}) {
  const comparisons = useDeviceComparisons(props.tree);
  return <><DeviceTable {...props} comparisons={comparisons} />
    <DeviceComparisonDialog controller={comparisons} onTreeUpdate={props.onTreeUpdate} snapshotRevision={props.tree.snapshot_revision} /></>;
}
export function Inspector(props: Omit<ComponentProps<typeof InspectorView>, "comparisons">) {
  const comparisons = useDeviceComparisons(props.tree);
  return props.propertiesOnly ? <InspectorView {...props} propertiesOnly />
    : <InspectorView {...props} propertiesOnly={false} comparisons={comparisons} />;
}
