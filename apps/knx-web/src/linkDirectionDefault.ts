/** Suggests a new group link's direction from the object, never from the address name (#47). */
import type { ComObjectNode } from "./bindings/ComObjectNode";

export type SuggestedDirection = "Send" | "Receive";
/** Why a direction was suggested. */
export type DirectionReason = "alreadyLinked" | "hasSender" | "firstSends";

/**
 * The first group address of an object is its sending one: ETS writes it so
 * (Project Schema23, `ComObjectInstanceRef/@Links`), the import reads it so,
 * and the device transmits and answers reads on it; legacy tables without one
 * store FEh and the object can never answer. An object sends on exactly one
 * address (KNX §4.17.9.4.1; a download refuses a second), so once it has one,
 * further links receive. The flags do not decide: a Write-only actuator object
 * still answers reads on its sending address.
 *
 * With `gaId`, an object already linked to that address keeps its existing
 * direction, so the form shows the existing refusal instead of proposing the
 * same object on one address twice (group-table builders refuse that).
 */
export function suggestLinkDirection(object: Pick<ComObjectNode, "links">, gaId: number | null = null):
  { direction: SuggestedDirection; reason: DirectionReason } {
  const here = gaId === null ? [] : object.links.filter((link) => link.ga_id === gaId);
  if (here.length > 0) {
    return { direction: here.some((link) => link.direction === "Send") ? "Send" : "Receive", reason: "alreadyLinked" };
  }
  if (object.links.some((link) => link.direction === "Send")) return { direction: "Receive", reason: "hasSender" };
  return { direction: "Send", reason: "firstSends" };
}
