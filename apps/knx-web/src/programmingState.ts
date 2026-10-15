/** The parts of a device's programming the project still owes it, as the tree reports them. */
import type { DeviceNode } from "./bindings/DeviceNode";
import type { PendingProgramming } from "./bindings/PendingProgramming";
import type { MessageKey } from "./i18n";
import type { AddressProgrammingStatus, DeviceDownloadStatus } from "./api";

export type ProgrammingPart = keyof PendingProgramming;

/** Display order, the order ETS uses: Adr · App · Par · GA. */
export const PROGRAMMING_PARTS: readonly ProgrammingPart[] = ["address", "application", "parameters", "group_addresses"];

/** Short chip labels and full names, per part. */
export const PART_LABELS: Record<ProgrammingPart, { short: MessageKey; long: MessageKey }> = {
  address: { short: "programming.part.address.short", long: "programming.part.address" },
  application: { short: "programming.part.application.short", long: "programming.part.application" },
  parameters: { short: "programming.part.parameters.short", long: "programming.part.parameters" },
  group_addresses: { short: "programming.part.groupAddresses.short", long: "programming.part.groupAddresses" },
};

/**
 * The pending parts in display order. A device without the field (an older
 * server, a hand-written tree) has none: the client never guesses.
 */
export function pendingParts(device: Pick<DeviceNode, "pending_programming">): ProgrammingPart[] {
  const state = device.pending_programming;
  return state ? PROGRAMMING_PARTS.filter((part) => state[part]) : [];
}

export function hasPendingProgramming(device: Pick<DeviceNode, "pending_programming">): boolean {
  return pendingParts(device).length > 0;
}

/** Admits the optional wire field: absent, or exactly the four booleans. */
export function isPendingProgramming(value: unknown): value is PendingProgramming {
  if (value === null || typeof value !== "object") return false;
  const keys = Object.keys(value);
  return keys.length === PROGRAMMING_PARTS.length
    && PROGRAMMING_PARTS.every((part) => typeof (value as Record<string, unknown>)[part] === "boolean");
}

/**
 * Whether a download's end changed the project's belief on the server, so the
 * editor must reload its tree: every end that may have written to the device.
 */
export function downloadEndChangesProject(status: DeviceDownloadStatus): boolean {
  return status.state !== "running" && status.written !== "no";
}

/** The same for an individual-address programming. */
export function addressEndChangesProject(status: AddressProgrammingStatus): boolean {
  if (status.state === "finished") return status.written !== "no";
  if (status.state === "failed") return status.written === "yes" || status.written === "unconfirmed";
  return false;
}
