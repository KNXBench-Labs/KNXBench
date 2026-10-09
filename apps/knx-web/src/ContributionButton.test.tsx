/** Regression tests for consented community evidence. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
const api = vi.hoisted(() => ({ analyzeContribution: vi.fn(), previewContribution: vi.fn(), exportContribution: vi.fn() }));
vi.mock("./contributionApi", () => api);
import ContributionButton from "./ContributionButton";
import { resetSettingsForTests, settingsStorage } from "./settingsStore";
import { resetUiLanguageForTests } from "./uiLanguage";
let root: ReturnType<typeof createRoot> | undefined;
let host: HTMLDivElement | undefined;
afterEach(async () => { if (root) await act(() => root!.unmount()); host?.remove(); vi.restoreAllMocks(); vi.clearAllMocks(); resetSettingsForTests(); resetUiLanguageForTests(); });
const report = { kind: "product", formatVersion: 1, scheme: 24, status: "refused", analyzerVersion: "test", sourceSize: 1, originalAllowed: true,
  findings: [{ id: "finding-1", stage: "product-import", category: "refused", name: null, sourcePath: null, xpath: null, occurrences: 1, detail: "Typed schema refused", sample: null }],
  checks: [{ name: "product-import", status: "refused", detail: "No typed import" }], members: [{ id: "member-1", path: "private.xml", size: 5, sampleAllowed: true, reason: "Unmodified context" }], structure: [] };
const preview = { manifest: { formatVersion: 1, analyzerVersion: "test", audience: "public", disclosure: "reduced", originalSha256: null, files: [], limitations: [] }, files: [{ path: "findings.json", text: "Outgoing exact preview" }] };
function button(label: string) { return [...document.querySelectorAll("button")].find(b => b.textContent === label)!; }
async function click(label: string) { await act(async () => button(label).click()); }
async function render() {
  host = document.createElement("div"); document.body.appendChild(host); root = createRoot(host);
  await act(() => root!.render(<ContributionButton />));
  await click("Analyze support gaps…");
}
async function choose(name = "sample.knxprod") {
  const input = document.querySelector<HTMLInputElement>('input[type="file"]')!;
  Object.defineProperty(input, "files", { configurable: true, value: [new File(["source"], name)] });
  await act(() => input.dispatchEvent(new Event("change", { bubbles: true })));
}
it("shows local ordered procedure provenance without offering bus execution", async () => {
  const node = (name: string) => ({ namespace: "http://knx.org/xml/project/20", name, attributes: {}, children: [], text: "", byteStart: 10, byteEnd: 20 });
  const source = { source: "program", sha256: "a".repeat(64), namespace: "http://knx.org/xml/project/20" };
  api.analyzeContribution.mockResolvedValue({ ...report, status: "complete", procedureResolutions: [{
    formatVersion: 1, programId: "Synthetic-AP1", mask: "MV-07B0", style: "MergedProcedure", variant: "ap1",
    status: "partial", executable: false, sources: [source], templateAttributes: {},
    steps: [{ node: node("LdCtrlConnect"), origin: { ...source, location: "master/template/step-1" } },
      { node: node("UnknownControl"), origin: { ...source, location: "program/merge-2/step-1" } }],
    merges: [], issues: [{ code: "uninterpreted-step", location: "program/merge-2/step-1" }],
  }] });
  await render(); await choose(); await click("Upload & analyze on this instance");
  expect(document.body.textContent).toContain("Offline procedure sequences");
  expect(document.body.textContent).toContain("Not a download plan");
  const section = document.querySelector('[data-procedure-resolutions]')!;
  expect(section).not.toBeNull();
  expect([...section.querySelectorAll('[data-procedure-step] code')].map(e => e.textContent)).toEqual(["LdCtrlConnect", "UnknownControl"]);
  expect(section.textContent).toContain("master/template/step-1");
  expect(section.textContent).toContain("uninterpreted-step");
  expect(section.querySelector("button")).toBeNull();
  expect(api.exportContribution).not.toHaveBeenCalled();
});

it("never uploads on file selection and requires preview and explicit consent before export", async () => {
  api.analyzeContribution.mockResolvedValue(report); api.previewContribution.mockResolvedValue(preview);
  await render(); await choose();
  expect(api.analyzeContribution).not.toHaveBeenCalled();
  await click("Upload & analyze on this instance");
  expect(document.body.textContent).toContain("Typed schema refused");
  expect(button("Download evidence ZIP").disabled).toBe(true);
  await click("Preview selected evidence");
  expect(document.body.textContent).toContain("Outgoing exact preview");
  expect(button("Download evidence ZIP").disabled).toBe(true);
  const consent = document.querySelector<HTMLInputElement>('input[data-consent="export"]')!;
  await act(() => consent.click());
  expect(button("Download evidence ZIP").disabled).toBe(false);
  // Changing disclosure invalidates the reviewed preview and its consent.
  const sample = document.querySelector<HTMLInputElement>('input[data-sample="member-1"]')!;
  await act(() => sample.click());
  expect(button("Download evidence ZIP").disabled).toBe(true);
  expect(document.body.textContent).not.toContain("Outgoing exact preview");
  expect(api.exportContribution).not.toHaveBeenCalled();
});
it("ignores pending results after close and does not mutate the open project", async () => {
  let resolve!: (r: unknown) => void;
  api.analyzeContribution.mockImplementation(() => new Promise(r => { resolve = r; }));
  await render(); await choose();
  await click("Upload & analyze on this instance");
  await click("Close");
  await act(async () => resolve(report));
  expect(document.querySelector('[role="dialog"]')).toBeNull();
  await click("Analyze support gaps…");
  expect(document.body.textContent).not.toContain("Typed schema refused");
});

it("offers a detailed beginner guide before file selection without uploading", async () => {
  await render();
  const guide = document.querySelector('details[data-contribution-guide]');
  expect(guide).not.toBeNull();
  expect(guide?.querySelectorAll("ol > li")).toHaveLength(6);
  expect(guide?.textContent).toContain("No Git, XML or KNX schema knowledge is needed");
  expect(guide?.textContent).toContain("issue number");
  expect(guide?.textContent).toContain("No evidence ZIP");
  expect(document.querySelector('[data-contribution-next]')?.textContent).toContain("Choose a project or product file");
  const link = guide?.querySelector<HTMLAnchorElement>('a');
  expect(link?.href).toBe("https://github.com/KNXBench-Labs/KNXBench/blob/main/docs/contribution-intake/README.md");
  expect(api.analyzeContribution).not.toHaveBeenCalled();
});
it("explains the next action through selection, analysis and preview without implying delivery", async () => {
  api.analyzeContribution.mockResolvedValue(report); api.previewContribution.mockResolvedValue(preview);
  await render(); await choose();
  const next = () => document.querySelector('[data-contribution-next]')?.textContent;
  expect(next()).toContain("Upload & analyze on this instance");
  await click("Upload & analyze on this instance");
  expect(next()).toContain("Preview selected evidence");
  await click("Preview selected evidence");
  expect(next()).toContain("permission checkbox");
  await act(() => document.querySelector<HTMLInputElement>('[data-consent=export]')!.click());
  expect(next()).toContain("Download evidence ZIP");
});
it("prefills only the analyzer version and explains manual attachment after download", async () => {
  api.analyzeContribution.mockResolvedValue(report); api.previewContribution.mockResolvedValue(preview);
  api.exportContribution.mockResolvedValue(new Blob(["synthetic ZIP"]));
  vi.spyOn(URL, "createObjectURL").mockReturnValue("blob:synthetic");
  vi.spyOn(URL, "revokeObjectURL").mockImplementation(() => {});
  vi.spyOn(HTMLAnchorElement.prototype, "click").mockImplementation(() => {});
  await render(); await choose(); await click("Upload & analyze on this instance");
  await click("Preview selected evidence");
  await act(() => document.querySelector<HTMLInputElement>('[data-consent=export]')!.click());
  await click("Download evidence ZIP");
  const href = [...document.querySelectorAll<HTMLAnchorElement>('a')].find(a => a.textContent === "Open public contribution form")!.href;
  const url = new URL(href);
  expect([...url.searchParams.keys()]).toEqual(["template", "analyzer-version"]);
  expect(url.searchParams.get("analyzer-version")).toBe("test");
  expect(href).not.toContain("private.xml");
  expect(document.querySelector('[data-contribution-next]')?.textContent).toContain("Attach the saved ZIP yourself");
});

it("renders the complete German guide using the actual German action labels", async () => {
  settingsStorage.setItem("uiLanguage", "de"); resetUiLanguageForTests();
  host = document.createElement("div"); document.body.appendChild(host); root = createRoot(host);
  await act(() => root!.render(<ContributionButton />));
  await click("Unterstützungslücken analysieren…");
  const guide = document.querySelector('[data-contribution-guide]')!;
  expect(guide.querySelectorAll("ol > li")).toHaveLength(6);
  expect(guide.textContent).toContain("Du brauchst weder Git- noch XML-");
  expect(guide.textContent).toContain("Auf diese Instanz hochladen & analysieren");
  expect(guide.textContent).toContain("Ausgewählte Evidenz in Vorschau prüfen");
  expect(guide.textContent).toContain("Evidenz-ZIP herunterladen");
  expect(guide.textContent).toContain("Öffentliches Beitragsformular öffnen");
  expect(guide.textContent).not.toMatch(/\{(analyze|preview|download|github)\}/);
  expect(guide.querySelector<HTMLAnchorElement>("a")?.href).toBe("https://github.com/KNXBench-Labs/KNXBench/blob/main/docs/contribution-intake/DEUTSCH.md");
  expect(document.querySelector('[data-contribution-next]')?.textContent).toContain("Nächster Schritt");
  expect(api.analyzeContribution).not.toHaveBeenCalled();
});

it("refuses unsupported file kinds before uploading", async () => {
  await render(); await choose("secret.knxkeys");
  expect(button("Upload & analyze on this instance").disabled).toBe(true);
  expect(api.analyzeContribution).not.toHaveBeenCalled();
});
