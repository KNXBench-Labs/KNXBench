/** Pins refusal guidance to structured codes, never private error prose. */
import { describe, expect, it } from "vitest";
import { guidanceForError, guidanceForMark } from "./downloadSupportGuidance";

function error(status?: number, body?: unknown): Error {
  return Object.assign(new Error("PRIVATE customer 1.1.42 /home/private secret=value"), { status, body });
}

describe("support guidance", () => {
  it("offers installation, configuration and support-gap actions separately", () => {
    expect(guidanceForMark({ kind: "unsupported", category: "product-data", detail: "original" }).action).toBe("installData");
    expect(guidanceForMark({ kind: "unsupported", category: "configuration" }).action).toBe("fixConfiguration");
    expect(guidanceForMark({ kind: "excluded" }).action).toBe("respectExclusion");
    for (const category of ["byte-order", "modules", "not-memory-mapped", "procedure-style", "unmodelled-step", "procedure-shape", "procedure-contents", "image-structure"]) {
      expect(guidanceForMark({ kind: "unsupported", category }).action).toBe("requestSupport");
    }
    for (const category of ["parameter-value", "parameter-evaluation"]) {
      expect(guidanceForMark({ kind: "unsupported", category }).action).toBe("checkParameters");
    }
  });

  it("sends a declared vendor download plug-in (KL-006) to ETS, never to a support request", () => {
    expect(guidanceForMark({ kind: "unsupported", category: "vendor-download-plugin" }).action).toBe("useVendorTool");
    expect(guidanceForError(error(422, { kind: "downloadPreparationRefused", category: "vendor-download-plugin" }))).toMatchObject({
      kind: "unsupported", category: "vendor-download-plugin", code: "downloadPreparationRefused", action: "useVendorTool",
    });
  });

  it("retains unknown and missing categories and full local detail", () => {
    for (const category of [null, "warp-drive", "__proto__"]) {
      const guidance = guidanceForMark({ kind: "unsupported", category, detail: "PRIVATE" });
      expect(guidance).toMatchObject({ category, detail: "PRIVATE", action: "reportUnknown" });
    }
  });

  it("uses the additive plan refusal envelope and preserves old byte-order kind", () => {
    expect(guidanceForError(error(422, { kind: "downloadPreparationRefused", category: "modules" }))).toMatchObject({
      kind: "unsupported", category: "modules", code: "downloadPreparationRefused", action: "requestSupport",
    });
    expect(guidanceForError(error(422, { kind: "unsupportedByteOrder" })).category).toBe("byte-order");
    expect(guidanceForError(error(409, { kind: "downloadProductDataMissing" })).action).toBe("installData");
    expect(guidanceForError(error(409, { kind: "downloadProjectMissing" })).action).toBe("openProject");
    expect(guidanceForError(error(400, { kind: "downloadExcluded" })).action).toBe("respectExclusion");
  });

  it("gives existing start safety refusals their specific correction instead of compatibility guesses", () => {
    expect(guidanceForError(error(400, { kind: "downloadGatewayInvalid" })).action).toBe("fixGateway");
    expect(guidanceForError(error(409, { kind: "downloadPlanStale" })).action).toBe("prepareAgain");
    expect(guidanceForError(error(400, { kind: "downloadConfirmationRefused" })).action).toBe("confirmRequest");
    expect(guidanceForError(error(422, { kind: "downloadKeyRefused" })).action).toBe("checkKey");
    expect(guidanceForError(error(409, { kind: "gatewayBusy" })).action).toBe("releaseResource");
  });

  it("never interprets network/server failures as device incompatibility", () => {
    for (const status of [undefined, 500, 502, 503]) {
      expect(guidanceForError(error(status, { category: "modules" }))).toMatchObject({ kind: "transient", action: "retryService" });
    }
  });

  it("never mines unstructured errors for a category or loses technical text", () => {
    const refusal = error(422);
    expect(guidanceForError(refusal)).toMatchObject({ kind: "unknown", category: null, action: "reportUnknown", detail: refusal.message });
    expect(guidanceForError(error(422, { kind: "futureReason", category: 42 }))).toMatchObject({ code: "futureReason", category: null });
  });
});
