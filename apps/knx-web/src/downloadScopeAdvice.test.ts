/** Pending parts a download scope leaves unwritten (#39), from the server's flag advice. */
import { describe, expect, it } from "vitest";
import type { DeviceDownloadScopeAdvice } from "./api";
import { uncoveredParts } from "./downloadScopeAdvice";

const advice = (write: DeviceDownloadScopeAdvice["write"]): DeviceDownloadScopeAdvice =>
  ({ basis: "projectFlags", write, address: "current", mediumPending: false, fullyProgrammed: false });

describe("uncoveredParts", () => {
  it("names what a partial scope leaves pending", () => {
    expect(uncoveredParts("parameters", advice("both"))).toEqual(["groupAddresses"]);
    expect(uncoveredParts("groupAddresses", advice("both"))).toEqual(["parameters"]);
    expect(uncoveredParts("groupAddresses", advice("parameters"))).toEqual(["parameters"]);
    expect(uncoveredParts("both", advice("complete"))).toEqual(["application"]);
    expect(uncoveredParts("parameters", advice("complete"))).toEqual(["application"]);
  });
  it("is empty when the scope covers what is pending", () => {
    for (const scope of ["complete", "parameters", "groupAddresses", "both"] as const) {
      expect(uncoveredParts(scope, advice("nothing"))).toEqual([]);
      expect(uncoveredParts("complete", advice(scope))).toEqual([]);
    }
    expect(uncoveredParts("both", advice("parameters"))).toEqual([]);
    expect(uncoveredParts("both", advice("groupAddresses"))).toEqual([]);
    expect(uncoveredParts("parameters", advice("parameters"))).toEqual([]);
    // An unknown scope code from a newer server names nothing instead of throwing.
    expect(uncoveredParts("everything" as never, advice("both"))).toEqual([]);
  });
});
