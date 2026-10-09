/** Rejects malformed or execution-claiming procedure diagnostics at the HTTP boundary. */
import { afterEach, expect, it, vi } from "vitest";
import { analyzeContribution } from "./contributionApi";
afterEach(() => vi.unstubAllGlobals());
const base = { formatVersion: 1, kind: "product", status: "complete", findings: [], checks: [], members: [], structure: [], metrics: [] };
async function response(procedureResolutions: unknown) {
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(JSON.stringify({ ...base, procedureResolutions }), { status: 200 })));
  return analyzeContribution(new File(["synthetic"], "source.knxprod"));
}
it.each([{}, [null], [{ status: "expanded", executable: true }], [{ status: "invented", executable: false }]])("refuses invalid procedure diagnostics without promoting execution", async value => {
  await expect(response(value)).rejects.toThrow("unsupported analysis response");
});
const valid = { formatVersion: 1, programId: "Synthetic-AP1", mask: "MV-07B0", style: "MergedProcedure", variant: "ap1", status: "expanded", executable: false,
  sources: [{ source: "program", sha256: "a".repeat(64), namespace: "http://knx.org/xml/project/20" }], templateAttributes: {}, steps: [], merges: [], issues: [] };
it("accepts structurally expanded diagnostics but never execution claims", async () => {
  await expect(response([valid])).resolves.toMatchObject({ procedureResolutions: [valid] });
  await expect(response([{ ...valid, executable: true }])).rejects.toThrow("unsupported analysis response");
});
it.each(["status", "style"])("does not coerce arrays into a valid %s", async key => {
  await expect(response([{ ...valid, [key]: [valid[key as "status" | "style"]] }])).rejects.toThrow("unsupported analysis response");
});
it("accepts an empty optional sequence result without changing older analysis responses", async () => {
  await expect(response([])).resolves.toMatchObject({ procedureResolutions: [] });
});
