/** Checks opaque scope generation without the secure-context-only randomUUID API. */
// @vitest-environment happy-dom
import { describe, expect, it } from "vitest";
import { createFlowScope } from "./flowIdentity";
import { validFlowOwner } from "./flowChannel";
describe("Flow presentation identities", () => {
  it("uses non-empty, channel-safe independent lifetimes on LAN HTTP", () => {
    const one = createFlowScope(); const two = createFlowScope();
    expect(one).toMatch(/^[a-f0-9]{32}$/);
    expect(validFlowOwner(one)).toBe(true);
    expect(two).not.toBe(one);
  });
});
