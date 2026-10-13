/** Issue #8: pure filtering and DPT agreement for the searchable group-address picker. */
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import { dptParts } from "./comObjectView";
import { dptDisplay } from "./dptLabels";
import { groupAddressSpellings } from "./gaNotation";

/** Rows rendered at once; the list says "showing N of M" beyond it. */
export const PICKER_WINDOW = 100;

/**
 * How an address's DPTs relate to the object's effective DPT.
 * `match`: same subtype, or a main-only reference on either side.
 * `subtype`: same main type (same encoding), different subtype (meaning).
 * `mismatch`: no DPT of the address shares the object's main type.
 * `unknown`: one side has no DPT or an unreadable reference; never a mismatch.
 * Several DPTs on the address: the best agreement counts.
 */
export type DptAgreement = "match" | "subtype" | "mismatch" | "unknown";

export function dptAgreement(objectDpt: string | null, addressDpts: readonly string[]): DptAgreement {
  const own = dptParts(objectDpt);
  if (own === null || addressDpts.length === 0) return "unknown";
  const others = addressDpts.map(dptParts);
  if (others.some(parts => parts === null)) return "unknown";
  const same = others.filter(parts => parts![0] === own[0]);
  if (same.length === 0) return "mismatch";
  return same.some(parts => parts!.length === 1 || own.length === 1 || parts![1] === own[1]) ? "match" : "subtype";
}

/** Every query word must appear in the address (either spelling), name or a DPT label/reference. Order is kept. */
export function filterPickerAddresses(addresses: readonly GroupAddressNode[], query: string,
  format: (address: string) => string, language: string): GroupAddressNode[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (words.length === 0) return [...addresses];
  return addresses.filter(ga => {
    const haystack = [format(ga.address), ...groupAddressSpellings(ga.address), ga.name,
      ...ga.dpts.flatMap(dpt => [dpt, dptDisplay(dpt, language)])].join("\n").toLowerCase();
    return words.every(word => haystack.includes(word));
  });
}
