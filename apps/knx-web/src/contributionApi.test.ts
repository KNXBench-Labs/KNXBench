/** Regression tests for consented community evidence. */
// @vitest-environment happy-dom
import { afterEach, expect, it, vi } from "vitest";
import { analyzeContribution, exportContribution, previewContribution } from "./contributionApi";
afterEach(() => vi.unstubAllGlobals());
it("posts only to the own-instance boundary with multipart and no GitHub/install traffic", async () => {
  const fetch = vi.fn().mockResolvedValue(new Response(JSON.stringify({ formatVersion: 1, kind: "product", status: "refused", findings: [], checks: [], members: [], structure: [], metrics: [] }), { headers: { "content-type": "application/json" } }));
  vi.stubGlobal("fetch", fetch);
  const file = new File(["synthetic"], "source.knxprod");
  await analyzeContribution(file);
  expect(fetch).toHaveBeenCalledTimes(1);
  const [url, init] = fetch.mock.calls[0];
  expect(url).toBe("/api/contributions/analyze");
  expect(init.body).toBeInstanceOf(FormData);
  expect(init.headers).toBeUndefined();
  expect(init.body.get("file")).toBe(file);
});
it("does not interpret future evidence versions", async () => {
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue(new Response(JSON.stringify({ formatVersion: 2, findings: [], checks: [], members: [], structure: [] }))));
  await expect(analyzeContribution(new File(["x"], "source.knxprod"))).rejects.toThrow("unsupported analysis response");
});
it("rejects non-ZIP exports and preserves server refusals", async () => {
  const fetch = vi.fn().mockResolvedValueOnce(new Response("<html>login</html>", { headers: { "content-type": "text/html" } }))
    .mockResolvedValueOnce(new Response(JSON.stringify({ error: "review again" }), { status: 400 }));
  vi.stubGlobal("fetch", fetch);
  const file = new File(["x"], "source.knxprod");
  const o = { audience: "public" as const, sampleIds: [], includeOriginal: false, originalConsent: false, consent: true };
  await expect(exportContribution(file, o)).rejects.toThrow("expected evidence ZIP");
  await expect(previewContribution(file, o)).rejects.toThrow("review again");
});
