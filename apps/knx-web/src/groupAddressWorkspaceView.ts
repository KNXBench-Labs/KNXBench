/** Typed shared state and actions for the four group-address renderers. */
import type { InstallationNode } from "./bindings/InstallationNode";
import type { MultiSelection, Selection } from "./selection";
import type { ItemClickHandler } from "./multiSelection";
import type { Translate } from "./i18n";
import type { GroupAddressPresentation, GroupAddressPresentationGroup, GroupAddressPresentationRow } from "./groupAddressPresentation";

export interface GroupAddressWorkspaceViewProps {
  installation: InstallationNode;
  index: GroupAddressPresentation;
  rows: GroupAddressPresentationRow[];
  baseRows: GroupAddressPresentationRow[];
  scope: string | null;
  onScope: (scope: string | null) => void;
  selection: Selection | null;
  multiSelection: MultiSelection | null;
  onItemClick: ItemClickHandler;
  limit: number;
  onLimit: (limit: number) => void;
  showColumnReset?: boolean;
}

export function groupAddressGroupLabel(t: Translate, group: GroupAddressPresentationGroup): string {
  return group.range ? (group.range.name || t("gaWorkspace.unnamedRange", { id: group.range.id }))
    : t(group.kind === "unassigned" ? "addressTable.noRange" : "gaWorkspace.unresolved");
}
