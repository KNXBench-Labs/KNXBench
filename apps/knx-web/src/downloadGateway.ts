/** Download gateway: one value, its origin, and what a search may change. */
import type { BusDiscoveredInterface } from "./api";
import type { BusDiscoveryState } from "./busDiscovery";

/**
 * Where the gateway value came from. Only `none` may be filled in by the
 * shared interface search; `picked` and `typed` are the user's and are never
 * overwritten by a later search result. `preferred` comes from Settings and
 * is a stated preference, not a search result.
 */
export type GatewayOrigin = "none" | "preferred" | "auto" | "picked" | "typed";
export interface GatewayChoice { value: string; origin: GatewayOrigin }

export function initialGatewayChoice(preferred: string): GatewayChoice {
  const value = preferred.trim();
  return value === "" ? { value: "", origin: "none" } : { value, origin: "preferred" };
}

export const pickedGateway = (endpoint: string): GatewayChoice => ({ value: endpoint, origin: "picked" });
export const typedGateway = (value: string): GatewayChoice => ({ value, origin: "typed" });

/**
 * Exactly one interface answered and nobody chose or typed a gateway yet:
 * that one is the sensible default. Anything else leaves the value alone —
 * several answers need a choice, and an existing value is never replaced.
 */
export function withDiscoveryDefault(choice: GatewayChoice, discovery: BusDiscoveryState): GatewayChoice {
  if (choice.origin !== "none" || discovery.phase !== "done" || discovery.interfaces.length !== 1) return choice;
  return { value: discovery.interfaces[0].controlEndpoint, origin: "auto" };
}

export const isFound = (value: string, interfaces: readonly BusDiscoveredInterface[]) =>
  value.trim() !== "" && interfaces.some((iface) => iface.controlEndpoint === value.trim());

/**
 * What to say about the current value next to the field:
 * - `preferred` / `preferredFound`: from Settings; "found" only when the
 *   search actually returned that endpoint.
 * - `auto`: filled in because it was the only answer.
 * - `stale`: came from a search, but the latest finished search no longer
 *   lists it. It stays as chosen; the user decides.
 */
export type GatewayNote = "preferred" | "preferredFound" | "auto" | "stale" | null;

export function gatewayNote(choice: GatewayChoice, discovery: BusDiscoveryState): GatewayNote {
  const found = isFound(choice.value, discovery.interfaces);
  if (choice.origin === "preferred") return found ? "preferredFound" : "preferred";
  const fromSearch = choice.origin === "auto" || choice.origin === "picked";
  if (fromSearch && discovery.phase === "done" && !found) return "stale";
  return choice.origin === "auto" ? "auto" : null;
}

/** Option text that tells several interfaces apart: name, endpoint, interface address. */
export function gatewayOptionLabel(iface: BusDiscoveredInterface): string {
  return `${iface.friendlyName || "—"} — ${iface.controlEndpoint} · ${iface.individualAddress}`;
}
