/** Suggests a new group link's direction from the object, never from the address name (#47). */
import type { ComObjectNode } from "./bindings/ComObjectNode";

export type SuggestedDirection = "Send" | "Receive";
/** Why a direction was suggested; `fallback` means the object gave no hint. */
export type DirectionReason = "hasSender" | "transmitOnly" | "writeOnly" | "fallback";

/**
 * An object transmits on exactly one sending address (KNX §4.17.9.4.1;
 * a download refuses a second, `SeveralSendingAddresses`), so once it has
 * one, the next link receives. Otherwise the communication flags decide:
 * Transmit without Write sends, Write without Transmit receives. Anything
 * else (both, neither, flags unknown) keeps the earlier default, Send.
 */
export function suggestLinkDirection(object: Pick<ComObjectNode, "links" | "transmit" | "write">):
  { direction: SuggestedDirection; reason: DirectionReason } {
  if (object.links.some((link) => link.direction === "Send")) return { direction: "Receive", reason: "hasSender" };
  if (object.transmit && !object.write) return { direction: "Send", reason: "transmitOnly" };
  if (object.write && !object.transmit) return { direction: "Receive", reason: "writeOnly" };
  return { direction: "Send", reason: "fallback" };
}
