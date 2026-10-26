/** Tests the diagnostic dialog's pure helpers: plan summary, progress and refusal copy. */
import {describe, expect, it} from "vitest";
import {diagnosticProgress, hexWord, memoryPlan, refusalMessage} from "./diagnosticMemory";
import type {DiagnosticView, MemoryEvidence} from "./diagnosticApi";
import {messages as en} from "./messages/en";
import {messages as de} from "./messages/de";

const memory = {
  profile: "system2-declared-segments-v1", notice: "n", programId: "M-0001_A-0001-01-0000",
  hardwareTypeComparison: {objectIndex: 0, propertyId: 78, procedurePosition: 1, inlineData: "00010000005b"}, chunkOctets: 12,
  ranges: [
    {order: 1, address: 0x0116, length: 30, procedurePosition: 2, coverage: {read: 12, refusedAmbiguous: 0, partial: 0, error: 0, notRead: 18}},
    {order: 2, address: 0x3c00, length: 12, procedurePosition: 3, coverage: {read: 0, refusedAmbiguous: 0, partial: 0, error: 0, notRead: 12}},
  ],
  chunks: [
    {order: 1, range: 1, address: 0x0116, requestedOctets: 12, status: "read", receivedOctets: 12, bytes: "", attemptedAt: "t", completedAt: "t", error: null},
    {order: 2, range: 1, address: 0x0122, requestedOctets: 12, status: "not-read", receivedOctets: 0, bytes: "", attemptedAt: "t", completedAt: null, error: null},
    {order: 3, range: 1, address: 0x012e, requestedOctets: 6, status: "not-read", receivedOctets: 0, bytes: "", attemptedAt: null, completedAt: null, error: null},
    {order: 4, range: 2, address: 0x3c00, requestedOctets: 12, status: "not-read", receivedOctets: 0, bytes: "", attemptedAt: null, completedAt: null, error: null},
  ],
} satisfies MemoryEvidence;
const items = [0, 1, 2].map((i) => ({completedAt: i < 2 ? "t" : null}));

describe("diagnostic dialog helpers", () => {
  it("formats addresses like the CLI preview", () => {
    expect([hexWord(0x0116), hexWord(0x3c00), hexWord(0)]).toEqual(["0116h", "3C00h", "0000h"]);
  });
  it("summarises the declared ranges with total octets and read count", () => {
    expect(memoryPlan(memory)).toEqual({
      ranges: [{order: 1, address: "0116h", length: 30, procedurePosition: 2}, {order: 2, address: "3C00h", length: 12, procedurePosition: 3}],
      totalOctets: 42, reads: 4, chunkOctets: 12,
    });
  });
  it("counts completed identity reads and, separately, completed memory chunks", () => {
    expect(diagnosticProgress({report: {items}, memory} as unknown as DiagnosticView)).toEqual({identity: 2, memory: {chunks: 1, total: 4}});
    expect(diagnosticProgress({report: {items}, memory: null} as unknown as DiagnosticView)).toEqual({identity: 2, memory: null});
  });
  it("maps only known refusal codes to copy present in both catalogues", () => {
    for (const code of ["diagnostics-product-db-required", "diagnostics-identity-assertion-missing", "diagnostics-profile-unavailable", "diagnostics-address-ambiguous"]) {
      const key = refusalMessage(code);
      expect(key).not.toBeNull();
      expect(en[key!].length).toBeGreaterThan(0);
      expect(de[key!].length).toBeGreaterThan(0);
    }
    for (const code of [null, "diagnostics-something-new", "constructor", "toString"]) expect(refusalMessage(code)).toBeNull();
  });
});
