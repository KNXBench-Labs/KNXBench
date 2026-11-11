/** Pins the object-based direction suggestion for a new group link (issue #47). */
import { describe, expect, it } from "vitest";
import { suggestLinkDirection } from "./linkDirectionDefault";

const object = (links: Array<[number, string]> = []) =>
  ({ links: links.map(([ga_id, direction]) => ({ ga_id, address: null, name: null, direction })) });

describe("suggestLinkDirection", () => {
  it("makes the first link the sending one, whatever the flags would say", () => {
    expect(suggestLinkDirection(object())).toEqual({ direction: "Send", reason: "firstSends" });
    expect(suggestLinkDirection(object([[4, "Receive"]]))).toEqual({ direction: "Send", reason: "firstSends" });
  });
  it("receives once the object sends on some address", () => {
    expect(suggestLinkDirection(object([[4, "Send"]]))).toEqual({ direction: "Receive", reason: "hasSender" });
    expect(suggestLinkDirection(object([[4, "Send"]]), 9)).toEqual({ direction: "Receive", reason: "hasSender" });
  });
  it("never proposes a second link of the object on the same address", () => {
    expect(suggestLinkDirection(object([[9, "Send"]]), 9)).toEqual({ direction: "Send", reason: "alreadyLinked" });
    // Sends elsewhere, receives here: suggesting Send would create a second sender.
    expect(suggestLinkDirection(object([[4, "Send"], [9, "Receive"]]), 9)).toEqual({ direction: "Receive", reason: "alreadyLinked" });
  });
});
