/** Plug-in data adoption: refusal wording, error classes and pair filtering. */
import { describe, expect, it } from "vitest";
import { translateFor } from "./i18n";
import {
  addressDifference, classifyAdoptionError, pairKey, refusalMessage, REFUSAL_CODES, visiblePairs,
  type AdoptionCandidate, type AdoptionPreview, type RefusalDetail,
} from "./pluginData";

const en = (key: Parameters<typeof translateFor>[1], params?: Parameters<typeof translateFor>[2]) =>
  translateFor("en", key, params);
const de = (key: Parameters<typeof translateFor>[1], params?: Parameters<typeof translateFor>[2]) =>
  translateFor("de", key, params);

/** What `api.ts`'s `request()` throws: an `Error` with `status` and `body`. */
function httpError(status: number, body: Record<string, unknown>): Error {
  return Object.assign(new Error(String(body.error ?? status)), { status, body });
}

const device = (id: number, name: string, address: string | null = "1.1.1") => ({ id, name, etsId: `P-0001-0_DI-${id}`, address });
const candidate = (source: number, target: number, refusal: RefusalDetail | null = null): AdoptionCandidate => ({
  sourceDevice: device(source, `Source ${source}`), targetDevice: device(target, `Target ${target}`),
  refusal: refusal?.message ?? null, refusalDetail: refusal, replacements: refusal ? 0 : 2,
});

describe("refusalMessage", () => {
  it("words every refusal class the server names, in English and German, with its facts", () => {
    const facts: Record<string, Partial<RefusalDetail>> = {
      unknownDevice: { side: "destination", id: 42 },
      noProjectDirectory: { side: "source", device: "Sensor EG" },
      productReferenceDiffers: { sourceValue: "M-0001_H-1_P-2", targetValue: "M-0001_H-1_P-1" },
      applicationReferenceDiffers: { sourceValue: "HP-2", targetValue: "HP-1" },
      danglingGroupLink: { side: "source" },
      entryMissing: { side: "destination", path: "P-0001/BinaryData/x.dat" },
      entryAmbiguous: { side: "project", path: "P-0001/ExtraData/7.*" },
      unreopenable: { reason: "encode failed" },
    };
    expect(REFUSAL_CODES).toHaveLength(14);
    for (const code of REFUSAL_CODES) {
      const detail: RefusalDetail = { code, message: `server text for ${code}`, ...facts[code] };
      for (const t of [en, de]) {
        const text = refusalMessage(t, detail);
        expect(text, code).not.toBe(detail.message);
        expect(text, code).not.toMatch(/\{[a-z]+\}/);
        for (const value of [detail.id, detail.device, detail.path, detail.sourceValue, detail.targetValue, detail.reason]) {
          if (value !== undefined) expect(text, code).toContain(String(value));
        }
      }
    }
    expect(refusalMessage(de, { code: "entryMissing", message: "x", side: "destination", path: "a/b" })).toContain("Zielprojekt");
    expect(refusalMessage(en, { code: "entryMissing", message: "x", side: "source", path: "a/b" })).toContain("source project");
  });

  it("keeps the server's text for a code this client does not know yet", () => {
    expect(refusalMessage(en, { code: "somethingNew", message: "the server's own words" })).toBe("the server's own words");
  });
});

describe("classifyAdoptionError", () => {
  it("names each server refusal kind instead of a generic failure", () => {
    expect(classifyAdoptionError(httpError(409, { kind: "pluginDataStale", error: "stale" })).kind).toBe("stale");
    expect(classifyAdoptionError(httpError(409, { kind: "pluginDataSourceChanged", error: "x" })).kind).toBe("sourceChanged");
    expect(classifyAdoptionError(httpError(422, { kind: "pluginDataNativeRequired", error: "x" })).kind).toBe("nativeRequired");
    expect(classifyAdoptionError(httpError(422, { kind: "pluginDataRollbackUnavailable", error: "limit" })).kind).toBe("rollbackUnavailable");
    expect(classifyAdoptionError(httpError(409, { kind: "projectHistoryStale", error: "x" })).kind).toBe("historyStale");
    expect(classifyAdoptionError(httpError(422, { kind: "pluginDataSource", error: "bad zip" }))).toEqual({ kind: "source", message: "bad zip" });
    expect(classifyAdoptionError(httpError(422, { kind: "pluginDataTarget", error: "x" })).kind).toBe("target");
    const refusal = { code: "groupLinksDiffer", message: "links differ" };
    expect(classifyAdoptionError(httpError(422, { kind: "pluginDataRefused", error: "links differ", refusal }))).toEqual({ kind: "refused", refusal, message: "links differ" });
  });

  it("treats a 409 of another kind, a malformed refusal and network errors as plain failures", () => {
    expect(classifyAdoptionError(httpError(409, { kind: "somethingElse", error: "other" }))).toEqual({ kind: "other", message: "other" });
    expect(classifyAdoptionError(httpError(422, { kind: "pluginDataRefused", error: "no detail" }))).toEqual({ kind: "other", message: "no detail" });
    expect(classifyAdoptionError(new TypeError("Failed to fetch"))).toEqual({ kind: "other", message: "Failed to fetch" });
    expect(classifyAdoptionError("text")).toEqual({ kind: "other", message: "text" });
  });
});

describe("visiblePairs", () => {
  const all = [candidate(1, 10), candidate(2, 20, { code: "groupLinksDiffer", message: "m" }), candidate(3, 10, { code: "alreadyIdentical", message: "m" })];
  it("shows only the chosen destination device's pairs unless all are asked for", () => {
    expect(visiblePairs(all, 10, false)).toEqual({ shown: [all[0], all[2]], hidden: 1 });
    expect(visiblePairs(all, 10, true)).toEqual({ shown: all, hidden: 0 });
    expect(visiblePairs(all, null, false)).toEqual({ shown: all, hidden: 0 });
    expect(visiblePairs(all, 99, false)).toEqual({ shown: [], hidden: 3 });
  });
  it("keys a pair by both device ids", () => {
    expect(pairKey(all[0])).toBe("1:10");
    expect(pairKey(all[2])).not.toBe(pairKey(all[0]));
  });
});

describe("addressDifference", () => {
  const preview = (source: string | null, target: string | null): AdoptionPreview => ({
    sourceHash: "a".repeat(64), request: { sourceDevice: 1, targetDevice: 10 },
    sourceDevice: device(1, "S", source), targetDevice: device(10, "T", target),
    unchangedPaths: [], replacements: [], notes: [],
  });
  it("reports a different individual address, including a missing one, and nothing when equal", () => {
    expect(addressDifference(preview("1.1.2", "1.1.1"))).toEqual({ source: "1.1.2", target: "1.1.1" });
    expect(addressDifference(preview(null, "1.1.1"))).toEqual({ source: null, target: "1.1.1" });
    expect(addressDifference(preview("1.1.1", "1.1.1"))).toBeNull();
  });
});
