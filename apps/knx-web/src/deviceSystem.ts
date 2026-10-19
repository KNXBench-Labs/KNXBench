/** Declared system-family wire states (WP-20) and their translatable labels. */
import type { MessageKey } from "./i18n";

// The mask → family table lives in `knx-core` (`system_family.rs`); this is
// only the closed token vocabulary the server sends.
export const SYSTEM_FAMILIES = ["system1", "system2", "system7", "systemB", "system300", "coupler",
  "knxnetIpRouter", "usbInterface", "irDecoder", "rfBidirectional", "rfUnidirectional"] as const;
export type SystemFamilyToken = (typeof SYSTEM_FAMILIES)[number];
export type DeviceSystem =
  | { state: "noProduct" }
  | { state: "noApplicationProgram" }
  | { state: "applicationNotInDatabase" }
  | { state: "applicationNotOfProduct" }
  | { state: "noMaskVersion" }
  | { state: "invalidMask"; declared: string }
  | { state: "unknownMask"; mask: number }
  | { state: "family"; mask: number; family: SystemFamilyToken };

const FAMILY_LABELS: Record<SystemFamilyToken, MessageKey> = {
  system1: "devices.system.system1", system2: "devices.system.system2", system7: "devices.system.system7",
  systemB: "devices.system.systemB", system300: "devices.system.system300", coupler: "devices.system.coupler",
  knxnetIpRouter: "devices.system.knxnetIpRouter", usbInterface: "devices.system.usbInterface",
  irDecoder: "devices.system.irDecoder", rfBidirectional: "devices.system.rfBidirectional",
  rfUnidirectional: "devices.system.rfUnidirectional",
};

const isMask = (value: unknown): value is number =>
  Number.isSafeInteger(value) && (value as number) >= 0 && (value as number) <= 0xffff;

export function isDeviceSystem(value: unknown): value is DeviceSystem {
  if (value === null || typeof value !== "object") return false;
  const system = value as Record<string, unknown>;
  switch (system.state) {
    case "noProduct": case "noApplicationProgram": case "applicationNotInDatabase":
    case "applicationNotOfProduct": case "noMaskVersion": return true;
    case "invalidMask": return typeof system.declared === "string";
    case "unknownMask": return isMask(system.mask);
    case "family": return isMask(system.mask) && (SYSTEM_FAMILIES as readonly unknown[]).includes(system.family);
    default: return false;
  }
}

/** `0x0AFD` → `0AFDh`, the notation the KNX mask tables use. */
export function maskHex(mask: number): string {
  return `${mask.toString(16).toUpperCase().padStart(4, "0")}h`;
}

export interface SystemLabel { key: MessageKey; params?: Record<string, string> }

/** The cell text; `null` means the product itself did not resolve (show its resolution). */
export function systemLabel(system: DeviceSystem): SystemLabel | null {
  switch (system.state) {
    case "noProduct": return null;
    case "noApplicationProgram": return { key: "devices.system.noApplicationProgram" };
    case "applicationNotInDatabase": return { key: "devices.system.applicationNotInDatabase" };
    case "applicationNotOfProduct": return { key: "devices.system.applicationNotOfProduct" };
    case "noMaskVersion": return { key: "devices.system.noMaskVersion" };
    case "invalidMask": return { key: "devices.system.invalidMask", params: { declared: system.declared } };
    case "unknownMask": return { key: "devices.system.unknownMask", params: { mask: maskHex(system.mask) } };
    case "family": return { key: FAMILY_LABELS[system.family] };
  }
}
