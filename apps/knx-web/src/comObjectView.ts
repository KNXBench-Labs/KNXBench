/** Pure communication-object view derivation, independent of writes and UI state. */
import type { ComObjectNode } from "./bindings/ComObjectNode";
import type { ComObjectChannel } from "./bindings/ComObjectChannel";
import type { ComObjectProjectFolder } from "./bindings/ComObjectProjectFolder";
import { groupAddressSpellings } from "./gaNotation";
import { dptDisplay } from "./dptLabels";
export type ComSortColumn = "number" | "name" | "function" | "dpt" | "addresses" | "status" | "channel";
export interface ComFilters { query: string; status: string; link: "all" | "linked" | "unlinked"; dpt: string }
export interface ComSort { column: ComSortColumn | null; descending: boolean }
/** WP-06: what the grouped table groups by; the flat view is separate. */
export type ComGrouping = "channel" | "folder";
export interface ComGroup { key: string; channel: ComObjectChannel | null; folder: ComObjectProjectFolder | null; objects: ComObjectNode[]; total: number }
export const DEFAULT_FILTERS: ComFilters = { query: "", status: "all", link: "all", dpt: "all" };
export const ACTIVATION_ORDER = ["Active", "Inactive", "Undetermined", "NotEvaluated"];
export const effectiveDpt = (o: ComObjectNode): string | null => o.dpt ?? o.program_dpt;
export const evaluatedChannel = (o: ComObjectNode): ComObjectChannel | null => o.activation === "Active" ? o.channel ?? null : null;
export const channelKey = (o: ComObjectNode): string => evaluatedChannel(o) ? `channel:${evaluatedChannel(o)!.key}` : "unassigned";
/** WP-06a: saved-folder statuses that refuse a folder the tree would otherwise give. */
export const REFUSED_PROJECT_FOLDERS = ["Ambiguous", "UnidentifiedNode", "TreeUnresolved", "ModuleObject", "Inactive"] as const;
export type RefusedProjectFolder = typeof REFUSED_PROJECT_FOLDERS[number];
/** Declared `Folder` RefIds of an assigned saved project folder, outermost first; `null` otherwise. */
export function projectFolderIds(o: ComObjectNode): string[] | null {
  const folder = o.project_folder;
  return folder?.status === "Assigned" ? folder.path.filter(n => n.kind === "Folder").map(n => n.ref_id) : null;
}
/** Why the saved tree's folder is withheld; `null` for no folder, an assigned one or an unknown status. */
export function projectFolderRefusal(o: ComObjectNode): RefusedProjectFolder | null {
  const status = o.project_folder?.status;
  return REFUSED_PROJECT_FOLDERS.find(s => s === status) ?? null;
}
/** WP-06: the saved folder an object is grouped under: assigned, keyed and holding a Folder node. */
export function groupingFolder(o: ComObjectNode): ComObjectProjectFolder | null {
  const folder = o.project_folder;
  return folder?.status === "Assigned" && folder.key && folder.path.some(n => n.kind === "Folder") ? folder : null;
}
/** Folder groups join on the exact opaque key; everything else shares one fallback. */
export const folderKey = (o: ComObjectNode): string => { const f = groupingFolder(o); return f ? `folder:${f.key}` : "folder-none"; };
export const groupKey = (o: ComObjectNode, by: ComGrouping): string => by === "folder" ? folderKey(o) : channelKey(o);
export const hasFilters = (f: ComFilters): boolean => !!f.query.trim() || f.status !== "all" || f.link !== "all" || f.dpt !== "all";
/** `[main]` for `DPT-n`, `[main, sub]` for `DPST-n-m`, `null` for anything else. */
export function dptParts(value: string | null): number[] | null {
  const match = value?.match(/^(?:DPT-(\d+)|DPST-(\d+)-(\d+))$/);
  if (!match) return null;
  const parts = match[1] ? [Number(match[1])] : [Number(match[2]), Number(match[3])];
  return parts.every(Number.isSafeInteger) ? parts : null;
}
export function dptOptions(objects: ComObjectNode[]): string[] {
  const types = [...new Set(objects.map(effectiveDpt).filter((d): d is string => d !== null))];
  const families = [...new Set(types.map(dptParts).filter(p => p !== null).map(p => p[0]))].sort((a, b) => a - b);
  const collator = new Intl.Collator("en", { numeric: true });
  types.sort((a, b) => {
    const ap = dptParts(a), bp = dptParts(b);
    return ap && bp ? ap[0] - bp[0] || (ap[1] ?? -1) - (bp[1] ?? -1)
      : ap ? -1 : bp ? 1 : collator.compare(a, b);
  });
  return [...families.map(n => `family:${n}`), ...types.map(d => `exact:${d}`),
    ...(objects.some(o => effectiveDpt(o) === null) ? ["missing"] : [])];
}
function matchesDpt(o: ComObjectNode, filter: string): boolean {
  const dpt = effectiveDpt(o);
  if (filter === "all") return true;
  if (filter === "missing") return dpt === null;
  if (filter.startsWith("exact:")) return dpt === filter.slice(6);
  return filter.startsWith("family:") && dptParts(dpt)?.[0] === Number(filter.slice(7));
}
export function deriveComObjectView(objects: ComObjectNode[], filters: ComFilters, sort: ComSort, language: string, grouped: boolean, by: ComGrouping = "channel"): {
  objects: ComObjectNode[]; groups: ComGroup[]; descriptionMatches: Set<number>;
} {
  const collator = new Intl.Collator(language, { numeric: true, sensitivity: "base" });
  const needle = filters.query.trim().toLocaleLowerCase(language);
  const contains = (s: string | null | undefined) => !!s?.toLocaleLowerCase(language).includes(needle);
  const descriptionMatches = new Set<number>();
  const filtered = objects.filter(o => {
    const channel = evaluatedChannel(o);
    const fields = [String(o.number), o.name, o.function_text, effectiveDpt(o), o.dpt_text,
      effectiveDpt(o) === null ? null : dptDisplay(effectiveDpt(o), language),
      channel?.text, channel?.name, channel?.number, ...(projectFolderIds(o) ?? []), ...o.links.flatMap(l => [l.name, ...(l.address === null ? [] : groupAddressSpellings(l.address))])];
    const descriptionMatch = !!needle && contains(o.description) && !fields.some(contains);
    if (descriptionMatch) descriptionMatches.add(o.id);
    const state = ACTIVATION_ORDER.includes(o.activation) ? o.activation : "unknown";
    return (!needle || descriptionMatch || fields.some(contains))
      && (filters.status === "all" || state === filters.status)
      && (filters.link === "all" || (o.links.length > 0) === (filters.link === "linked"))
      && matchesDpt(o, filters.dpt);
  });
  const sourceIndex = new Map(objects.map((o, i) => [o.id, i]));
  const key = (o: ComObjectNode): string | number | null => {
    switch (sort.column) {
      case "number": return o.number;
      case "name": return o.name || null;
      case "function": return o.function_text || null;
      case "dpt": return effectiveDpt(o);
      case "addresses": return o.links.map(l => l.address).filter((a): a is string => a !== null).sort(collator.compare)[0] ?? null;
      case "status": { const rank = ACTIVATION_ORDER.indexOf(o.activation); return rank < 0 ? ACTIVATION_ORDER.length : rank; }
      case "channel": { const c = evaluatedChannel(o); return c?.text || c?.name || null; }
      default: return null;
    }
  };
  const compare = (a: ComObjectNode, b: ComObjectNode): number => {
    if (!sort.column) return sourceIndex.get(a.id)! - sourceIndex.get(b.id)!;
    const ak = key(a), bk = key(b);
    if (ak === null || bk === null) return ak === bk ? sourceIndex.get(a.id)! - sourceIndex.get(b.id)! : ak === null ? 1 : -1;
    let order: number;
    if (sort.column === "dpt") {
      const ap = dptParts(String(ak)), bp = dptParts(String(bk));
      // Valid canonical components precede verbatim unrecognized values.
      order = ap && bp ? ap[0] - bp[0] || (ap[1] ?? -1) - (bp[1] ?? -1)
        : ap ? -1 : bp ? 1 : collator.compare(String(ak), String(bk));
    } else order = typeof ak === "number" && typeof bk === "number" ? ak - bk : collator.compare(String(ak), String(bk));
    return (sort.descending ? -order : order) || sourceIndex.get(a.id)! - sourceIndex.get(b.id)!;
  };
  const groups = new Map<string, ComGroup & { order: number; first: number }>();
  objects.forEach((o, i) => {
    const key = groupKey(o, by), group = groups.get(key);
    if (group) group.total++;
    else if (by === "folder") {
      // First encounter decides folder order; the fallback always comes last.
      const folder = groupingFolder(o);
      groups.set(key, { key, channel: null, folder, total: 1, objects: [], order: folder ? 0 : 1, first: i });
    } else {
      const channel = evaluatedChannel(o);
      groups.set(key, { key, channel, folder: null, total: 1, objects: [], order: channel?.order ?? Number.MAX_SAFE_INTEGER, first: i });
    }
  });
  for (const o of filtered) groups.get(groupKey(o, by))!.objects.push(o);
  const ordered = [...groups.values()].sort((a, b) => a.order - b.order || a.first - b.first);
  ordered.forEach(g => g.objects.sort(compare));
  return { objects: grouped ? ordered.flatMap(g => g.objects) : [...filtered].sort(compare), groups: ordered, descriptionMatches };
}
