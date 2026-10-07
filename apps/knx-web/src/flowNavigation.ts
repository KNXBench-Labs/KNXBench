/** Resolves flow evidence to exact entities in the current main editor; never guesses by label. */
import type { ProjectTree } from "./bindings/ProjectTree";
import type { FlowModel, FlowNode } from "./flowModel";
import type { Selection } from "./selection";

export interface FlowTarget { kind: "device" | "group"; id: number; generation: string }
export interface FlowNavigation {
  available(target: FlowTarget): boolean;
  open(target: FlowTarget): Promise<boolean>;
}
export function nodeFlowTarget(node: FlowNode): FlowTarget | null {
  if (!node.generation) return null;
  if (node.kind === "device" && node.deviceId !== undefined) return { kind: "device", id: node.deviceId, generation: node.generation };
  if (node.kind === "group" && !node.ambiguous && node.gaRaw !== undefined) return { kind: "group", id: node.gaRaw, generation: node.generation };
  return null;
}
export function validFlowTarget(value: unknown): value is FlowTarget {
  if (!value || typeof value !== "object") return false;
  const v = value as FlowTarget;
  return (v.kind === "device" || v.kind === "group") && Number.isSafeInteger(v.id) && v.id >= 0 && v.id <= (v.kind === "group" ? 65535 : 0xffffffff) && typeof v.generation === "string" && /^(0|[1-9][0-9]*)$/.test(v.generation);
}
export function resolveFlowNavigation(model: FlowModel, target: FlowTarget, tree: ProjectTree | null, scope?: string): Selection | null {
  if (!tree || !scope || !validFlowTarget(target)) return null;
  const context = model.contexts.get(target.generation);
  if (context?.kind !== "participants" || context.projectScope !== scope || !context.complete) return null;
  if (tree.server_incarnation && tree.server_incarnation !== model.identity.serverIncarnation) return null;
  if (target.kind === "device") {
    const evidence = context.devicesById.get(target.id);
    if (!evidence || evidence.installationId === null) return null;
    const matches = tree.installations.flatMap(i => [...i.unassigned, ...i.topology.flatMap(a => a.lines.flatMap(l => l.devices))].map(d => ({ d, installation: i.id })));
    const own = matches.filter(({ d }) => d.id === target.id);
    if (own.length !== 1) return null;
    const { d, installation } = own[0];
    const raw = evidence.individualAddressRaw;
    const address = raw === null ? null : `${raw >> 12}.${(raw >> 8) & 15}.${raw & 255}`;
    if (d.name !== evidence.name || d.address !== address || installation !== evidence.installationId) return null;
    if (address !== null && matches.filter(m => m.d.address === address).length !== 1) return null;
    return { kind: "device", id: target.id };
  }
  const groups = context.groupsByAddress.get(target.id);
  if (!groups || groups.length !== 1 || context.groupAddressStyle !== tree.group_address_style) return null;
  const evidence = groups[0];
  const raw = evidence.gaRaw;
  const address = tree.group_address_style === "Free" ? String(raw) : tree.group_address_style === "TwoLevel" ? `${raw >> 11}/${raw & 2047}` : `${raw >> 11}/${(raw >> 8) & 7}/${raw & 255}`;
  const matches = tree.installations.flatMap(i => i.group_addresses.map(g => ({ g, installation: i.id })));
  const own = matches.filter(m => m.g.id === evidence.gaId);
  if (own.length !== 1 || matches.filter(m => m.g.address === address).length !== 1) return null;
  if (own[0].installation !== evidence.installationId || own[0].g.address !== address || own[0].g.name !== evidence.name) return null;
  return { kind: "group_address", id: evidence.gaId };
}
