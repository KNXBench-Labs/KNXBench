/** Pure identity and admission rules for project-local name editing. */
import type { ProjectTree } from "./bindings/ProjectTree";
import { collectDevices } from "./treeUtils";
export type RenameKind = "device" | "group_address";
export interface RenameTarget { kind: RenameKind; id: number; name: string }
export type NameProblem = "blank" | "long" | "control";
export function nameProblem(name: string): NameProblem | null {
  if (/^\p{White_Space}*$/u.test(name)) return "blank";
  if ([...name].length > 1024) return "long";
  if (/[\u0000-\u001f\u007f-\u009f\u2028\u2029\ud800-\udfff]/u.test(name)) return "control";
  return null;
}
export function renameTarget(tree: ProjectTree, kind: RenameKind, id: number): RenameTarget | null {
  if (kind === "device") {
    const device = collectDevices(tree).get(id);
    return device ? { kind, id, name: device.name } : null;
  }
  const matches = tree.installations.flatMap((i) => i.group_addresses).filter((ga) => ga.id === id);
  return matches.length === 1 ? { kind, id, name: matches[0].name } : null;
}
export function hasRenameContext(tree: ProjectTree): boolean {
  return typeof tree.server_incarnation === "string" && tree.server_incarnation.length > 0
    && Number.isSafeInteger(tree.snapshot_revision) && tree.snapshot_revision! >= 0
    && Number.isSafeInteger(tree.project_incarnation) && tree.project_incarnation! >= 0;
}
