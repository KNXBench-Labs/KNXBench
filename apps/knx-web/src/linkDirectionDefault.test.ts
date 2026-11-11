/** Pins the object-based direction suggestion for a new group link (issue #47). */
import { describe, expect, it } from "vitest";
import { suggestLinkDirection } from "./linkDirectionDefault";

const object = (flags: { transmit: boolean; write: boolean }, links: string[] = []) =>
  ({ ...flags, links: links.map((direction, i) => ({ ga_id: i + 1, direction })) }) as Parameters<typeof suggestLinkDirection>[0];

describe("suggestLinkDirection", () => {
  it("receives once the object already sends, whatever its flags", () => {
    for (const flags of [{ transmit: true, write: false }, { transmit: true, write: true }, { transmit: false, write: false }]) {
      expect(suggestLinkDirection(object(flags, ["Send"]))).toEqual({ direction: "Receive", reason: "hasSender" });
    }
  });
  it("sends for Transmit without Write and receives for Write without Transmit", () => {
    expect(suggestLinkDirection(object({ transmit: true, write: false }))).toEqual({ direction: "Send", reason: "transmitOnly" });
    expect(suggestLinkDirection(object({ transmit: false, write: true }))).toEqual({ direction: "Receive", reason: "writeOnly" });
  });
  it("only receive links do not count as a sender", () => {
    expect(suggestLinkDirection(object({ transmit: true, write: false }, ["Receive", "Receive"])).direction).toBe("Send");
  });
  it("keeps Send when the flags give no hint", () => {
    expect(suggestLinkDirection(object({ transmit: true, write: true }))).toEqual({ direction: "Send", reason: "fallback" });
    expect(suggestLinkDirection(object({ transmit: false, write: false }))).toEqual({ direction: "Send", reason: "fallback" });
  });
});
