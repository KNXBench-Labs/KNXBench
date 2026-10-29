/** Diagnostic bundle client: strict admission, consent in the export body, plain file names. */
import { afterEach, expect, it, vi } from "vitest";
import { admitPreview, deleteRecordings, dispositionFileName, exportBundle, previewBundle, sectionText } from "./diagnosticBundleApi";

const PREVIEW = {
  problemType: "deviceReading", recordingEnabled: true, operationRecordingId: null, deviceId: null,
  offers: [{ kind: "environment", offer: "fixed", selected: true, unavailable: null }],
  sections: [{ key: "environment", kind: "environment", label: null, bytes: 12, entries: 1,
    markers: { addresses: false, names: false, parameterValues: false, networkAddresses: false }, notes: [], files: ["environment.json"] }],
  totalBytes: 12, exceedsMailHint: false, fileName: "knxbench-diagnostics-1.zip", mailDraft: "mailto:issue@knxbench.com?subject=x", recordings: [],
};
const fetchMock = vi.fn();
afterEach(() => { vi.unstubAllGlobals(); fetchMock.mockReset(); });
function reply(status: number, body: unknown, headers: Record<string, string> = { "content-type": "application/json" }) {
  vi.stubGlobal("fetch", fetchMock);
  fetchMock.mockResolvedValue(new Response(typeof body === "string" || body instanceof Blob ? body : JSON.stringify(body), { status, headers }));
}

it("admits the server's preview and refuses wrong shapes", async () => {
  expect(admitPreview(PREVIEW)).toBe(PREVIEW);
  for (const broken of [
    { ...PREVIEW, mailDraft: "https://example.invalid" },
    { ...PREVIEW, offers: [{ kind: "unknownKind", offer: "fixed", selected: true, unavailable: null }] },
    { ...PREVIEW, offers: [{ kind: "device", offer: "notOffered", selected: false, unavailable: null }] },
    { ...PREVIEW, sections: [{ ...PREVIEW.sections[0], bytes: -1 }] },
    { ...PREVIEW, recordings: [{ id: "r" }] },
    { ...PREVIEW, totalBytes: "12" },
  ]) expect(() => admitPreview(broken)).toThrow("diagnostic-bundle-response-invalid");
  reply(200, PREVIEW);
  await expect(previewBundle({ problemType: "deviceReading" })).resolves.toEqual(PREVIEW);
  const [path, init] = fetchMock.mock.calls[0] as [string, RequestInit];
  expect(path).toBe("/api/diagnostics/bundle/preview");
  expect(init.method).toBe("POST");
  expect(JSON.parse(init.body as string)).toEqual({ problemType: "deviceReading" });
});

it("asks for one section's text by key and checks the answer names it", async () => {
  reply(200, { key: "sessionLog", text: "===== session-log/session-log.json =====\n[]\n" });
  await expect(sectionText({ problemType: "userInterface" }, "sessionLog")).resolves.toContain("session-log.json");
  expect(JSON.parse((fetchMock.mock.calls[0][1] as RequestInit).body as string)).toEqual({ problemType: "userInterface", key: "sessionLog" });
  reply(200, { key: "other", text: "" });
  await expect(sectionText({ problemType: "userInterface" }, "sessionLog")).rejects.toThrow("diagnostic-bundle-response-invalid");
});

it("deletes recordings and reports the kept active ones", async () => {
  reply(200, { removed: 3, keptActive: ["20261029T100000Z-download-1"] });
  await expect(deleteRecordings()).resolves.toEqual({ removed: 3, keptActive: ["20261029T100000Z-download-1"] });
  expect((fetchMock.mock.calls[0][1] as RequestInit).method).toBe("DELETE");
  reply(200, { removed: 1, kept_active: [] });
  await expect(deleteRecordings()).rejects.toThrow("diagnostic-bundle-response-invalid");
});

it("exports only with consent in the body and takes the server's plain file name", async () => {
  reply(200, new Blob(["PK"]), { "content-type": "application/zip", "content-disposition": 'attachment; filename="knxbench-diagnostics-0.1.0-op-1.zip"' });
  const saved = await exportBundle({ problemType: "deviceProgramming", description: "it broke" }, "fallback.zip");
  expect(saved.fileName).toBe("knxbench-diagnostics-0.1.0-op-1.zip");
  expect(await saved.blob.text()).toBe("PK");
  const [path, init] = fetchMock.mock.calls[0] as [string, RequestInit];
  expect(path).toBe("/api/diagnostics/bundle");
  expect(JSON.parse(init.body as string)).toEqual({ problemType: "deviceProgramming", description: "it broke", consent: true });
  reply(200, new Blob(["PK"]), { "content-type": "application/zip", "content-disposition": 'attachment; filename="../evil.zip"' });
  expect((await exportBundle({ problemType: "catalogue" }, "fallback.zip")).fileName).toBe("fallback.zip");
});

it("surfaces a refusal and rejects a body that is not a ZIP", async () => {
  reply(400, { error: "bundleConsentRequired" });
  await expect(exportBundle({ problemType: "catalogue" }, "f.zip")).rejects.toMatchObject({ message: "bundleConsentRequired", status: 400 });
  reply(200, { not: "zip" });
  await expect(exportBundle({ problemType: "catalogue" }, "f.zip")).rejects.toThrow("diagnostic-bundle-export-invalid");
});

it("reads only plain names from Content-Disposition", () => {
  expect(dispositionFileName('attachment; filename="a-1.zip"')).toBe("a-1.zip");
  expect(dispositionFileName('attachment; filename="a/b.zip"')).toBeNull();
  expect(dispositionFileName(null)).toBeNull();
});
