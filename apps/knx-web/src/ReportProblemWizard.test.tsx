/** Report problem wizard: defaults, preview, consent gate, save, mail draft and deletion. */
// @vitest-environment happy-dom
import { act, StrictMode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { BundlePreview, BundleQuery, OfferView, SectionKind } from "./diagnosticBundleApi";
const bundle = vi.hoisted(() => ({ previewBundle: vi.fn(), sectionText: vi.fn(), exportBundle: vi.fn(), deleteRecordings: vi.fn() }));
vi.mock("./diagnosticBundleApi", async (importOriginal) => ({ ...(await importOriginal<typeof import("./diagnosticBundleApi")>()), ...bundle }));
const saved = vi.hoisted(() => vi.fn());
vi.mock("./localJsonDownload", () => ({ downloadLocalBlob: saved }));
import ReportProblemWizard from "./ReportProblemWizard";
import { subscribeAchievementEvents, type AchievementEvent } from "./achievementEvents";
import { defaultOffer, defaultSelection, type ReportProblemRequest } from "./reportProblem";
import { messages as en } from "./messages/en";
import type { ToastNotification } from "./toast";
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const MARKERS = { addresses: true, names: true, parameterValues: false, networkAddresses: true };
const RECORDING = { id: "20261029T100000Z-download-aa", sizeBytes: 4096, formatVersion: 1, writerVersion: "0.1.0", operationKind: "download",
  startedAt: "2026-10-29T10:00:00Z", target: {}, gateway: "203.0.113.250:3671", related: { downloadId: "7" }, outcome: "failed", active: false, tornTail: false };
const OLDER = { ...RECORDING, id: "20261028T100000Z-download-bb", startedAt: "2026-10-28T10:00:00Z", outcome: "succeeded" };

/** The server's answer for `query`: offers per O1, selected ones built, one marked unavailable. */
function answer(query: BundleQuery, extra: Partial<BundlePreview> = {}): BundlePreview {
  const wanted = new Set(query.sections ?? defaultSelection(query.problemType));
  const offers: OfferView[] = [];
  const sections: BundlePreview["sections"] = [];
  for (const kind of ["environment", "sessionLog", "operation", "earlierOperation", "device", "legacyReports", "importReport", "importFile",
    "catalogue", "project", "busMonitor", "clientState"] as SectionKind[]) {
    const offer = defaultOffer(query.problemType, kind);
    if (offer === "notOffered") continue;
    const unavailable = kind === "busMonitor" ? "the bus monitor is not running" : kind === "importFile" ? "some new server reason" : null;
    const selected = (offer === "fixed" || wanted.has(kind)) && unavailable === null;
    offers.push({ kind, offer, selected, unavailable });
    if (selected) sections.push({ key: kind, kind, label: null, bytes: 1000 + offers.length, entries: offers.length,
      markers: kind === "environment" ? { addresses: false, names: false, parameterValues: false, networkAddresses: false } : MARKERS,
      notes: kind === "sessionLog" ? ["a server note"] : [], files: [`${kind}.json`] });
  }
  const totalBytes = sections.reduce((sum, s) => sum + s.bytes, 0);
  return { problemType: query.problemType, recordingEnabled: true, operationRecordingId: query.problemType.startsWith("device") ? RECORDING.id : null,
    deviceId: query.deviceId ?? null, offers, sections, totalBytes, exceedsMailHint: false,
    fileName: "knxbench-diagnostics-0.1.0-20261029100000.zip",
    mailDraft: "mailto:issue@knxbench.com?subject=KNXBench&body=Please%20attach%20the%20saved%20file%20knxbench-diagnostics-0.1.0-20261029100000.zip%20to%20this%20mail.",
    recordings: [RECORDING, OLDER], ...extra };
}

let host: HTMLDivElement; let root: Root; let onClose: ReturnType<typeof vi.fn<() => void>>;
const events: AchievementEvent[] = []; let unsubscribe: () => void;
beforeEach(() => {
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
  for (const mock of Object.values(bundle)) mock.mockReset();
  saved.mockReset(); onClose = vi.fn<() => void>(); events.length = 0;
  unsubscribe = subscribeAchievementEvents((event) => events.push(event));
  bundle.previewBundle.mockImplementation(async (query: BundleQuery) => answer(query));
});
afterEach(async () => { unsubscribe(); await act(async () => root.unmount()); host.remove(); document.body.innerHTML = ""; });

const flush = () => act(async () => { for (let i = 0; i < 6; i++) await Promise.resolve(); });
async function render(request: ReportProblemRequest = {}, notifications: ToastNotification[] = []) {
  await act(async () => root.render(<ReportProblemWizard key={JSON.stringify(request)} request={request} tree={null} notifications={notifications} onClose={onClose} />));
  await flush();
}
const $ = <T extends Element = HTMLElement>(selector: string) => document.querySelector<T>(selector);
const button = (label: string) => [...document.querySelectorAll<HTMLButtonElement>(".report-problem button")].find((b) => b.textContent === label)!;
async function click(target: HTMLElement | null) { expect(target).not.toBeNull(); await act(async () => target!.click()); await flush(); }
/** A step pill; its text is the step number followed by the label. */
const step = (label: string) => [...document.querySelectorAll<HTMLButtonElement>(".project-wizard-step-link")].find((b) => b.textContent!.endsWith(label))!;
const next = () => click(button(en["projectWizard.next"]));
const lastQuery = () => bundle.previewBundle.mock.calls.at(-1)![0] as BundleQuery;
const box = (kind: string) => $<HTMLInputElement>(`[data-report-section=${kind}] input[type=checkbox]`)!;

it("starts at the problem type from the menu and asks the server nothing there", async () => {
  await render();
  const radios = [...document.querySelectorAll<HTMLInputElement>("[data-report-type]")];
  expect(radios.map((r) => r.value)).toEqual(["deviceProgramming", "deviceReading", "projectImport", "catalogue", "userInterface"]);
  expect(radios.find((r) => r.checked)!.value).toBe("userInterface");
  expect(document.activeElement).toBe($(".project-wizard-step-title"));
  expect(bundle.previewBundle).not.toHaveBeenCalled();
  expect(step(en["reportProblem.step.preview"]).disabled).toBe(true);
});

for (const type of ["deviceProgramming", "deviceReading", "projectImport", "catalogue", "userInterface"] as const) {
  it(`${type}: preselects the O1 defaults and keeps the fixed row`, async () => {
    await render();
    await click($(`[data-report-type=${type}]`));
    await next();
    expect(lastQuery().sections).toEqual(defaultSelection(type));
    const ticked = [...document.querySelectorAll<HTMLElement>("[data-report-section]")]
      .filter((row) => row.querySelector<HTMLInputElement>("input[type=checkbox]:not([data-report-earlier])")?.checked)
      .map((row) => row.dataset.reportSection);
    expect(ticked).toEqual(defaultSelection(type));
    expect(box("environment").disabled).toBe(true);
    expect($("[data-report-section=environment]")!.textContent).toContain(en["reportProblem.fixed"]);
    expect(lastQuery().client !== undefined).toBe(type === "userInterface");
  });
}

it("shows unavailable sections disabled with the reason, translated when known", async () => {
  await render({ problemType: "deviceReading", deviceId: 4 });
  expect(box("busMonitor").disabled).toBe(true);
  expect(box("busMonitor").checked).toBe(false);
  const reason = $("[data-report-section=busMonitor] [data-report-unavailable]")!;
  expect(reason.textContent).toBe(en["reportProblem.unavailable"].replace("{reason}", en["reportProblem.reason.busMonitorOff"]));
  expect(box("busMonitor").getAttribute("aria-describedby")).toBe(reason.id);
  await render({ problemType: "projectImport" });
  expect($("[data-report-section=importFile] [data-report-unavailable]")!.textContent).toContain("some new server reason");
});

it("a shortcut opens at the contents with its operation, device and earlier picks", async () => {
  await render({ problemType: "deviceProgramming", deviceId: 4, operation: { relatedKey: "downloadId", relatedValue: "7" } });
  expect(document.querySelector("[aria-current=step]")!.textContent).toContain(en["reportProblem.step.contents"]);
  expect(lastQuery()).toMatchObject({ operation: { relatedKey: "downloadId", relatedValue: "7" }, deviceId: 4 });
  expect($<HTMLSelectElement>("[data-report-operation]")!.value).toBe(RECORDING.id);
  await click($(`[data-report-earlier="${OLDER.id}"]`));
  expect(lastQuery().earlierRecordingIds).toEqual([OLDER.id]);
  expect(lastQuery().sections).toContain("earlierOperation");
  // The current operation is not offered again as an earlier one.
  expect($(`[data-report-earlier="${RECORDING.id}"]`)).toBeNull();
});

it("says when the shortcut's operation has no recording", async () => {
  bundle.previewBundle.mockImplementation(async (query: BundleQuery) => answer(query, { operationRecordingId: null }));
  await render({ problemType: "deviceProgramming", operation: { relatedKey: "operationId", relatedValue: "gone" } });
  expect($("[data-report-operation-missing]")!.textContent).toBe(en["reportProblem.operation.notFound"]);
});

it("states plainly when recording is off and when the ZIP exceeds common mail limits", async () => {
  bundle.previewBundle.mockImplementation(async (query: BundleQuery) => answer(query, { recordingEnabled: false, exceedsMailHint: true, totalBytes: 11 * 1024 * 1024 }));
  await render({ problemType: "deviceReading" });
  expect($("[data-report-recording-off]")!.textContent).toBe(en["reportProblem.recordingOff"]);
  expect($("[data-report-mail-hint]")!.textContent).toBe(en["reportProblem.mailHint"]);
  expect($("[data-report-total]")!.textContent).toBe("Total: 11 MiB");
});

it("previews markers, notes and a section's text on request", async () => {
  bundle.sectionText.mockResolvedValue("===== session-log/session-log.json =====\n[]\n");
  await render({ problemType: "userInterface" });
  await next();
  const log = $("[data-report-preview=sessionLog]")!;
  expect([...log.querySelectorAll("[data-report-markers] li")].map((li) => li.textContent)).toEqual([
    en["reportProblem.marker.addresses"], en["reportProblem.marker.names"], en["reportProblem.marker.networkAddresses"],
  ]);
  expect($("[data-report-preview=environment] [data-report-markers]")!.textContent).toBe(en["reportProblem.preview.noMarkers"]);
  expect(log.textContent).toContain("a server note");
  expect($("[data-report-removed]")!.textContent).toBe(en["reportProblem.removed"]);
  expect($("[data-report-not-removed]")!.textContent).toBe(en["reportProblem.notRemoved"]);
  expect(bundle.sectionText).not.toHaveBeenCalled();
  await click(log.querySelector("[data-report-show]"));
  expect(bundle.sectionText).toHaveBeenCalledExactlyOnceWith(lastQuery(), "sessionLog");
  expect(log.querySelector("pre[data-report-text]")!.textContent).toContain("session-log.json");
  await click(log.querySelector("[data-report-show]"));
  expect(log.querySelector("pre")).toBeNull();
  expect(bundle.exportBundle).not.toHaveBeenCalled();
});

it("exports nothing before consent, then saves the server's file and enables the mail draft", async () => {
  bundle.exportBundle.mockResolvedValue({ blob: new Blob(["PK"]), fileName: "knxbench-diagnostics-0.1.0-20261029100007.zip" });
  const errors = Array.from({ length: 60 }, (_, i): ToastNotification => ({ id: i, entry: { timestamp: "2026-10-29T10:00:00Z", severity: "error",
    source: "ui:toast:error", message: `boom ${i}`, detail: null, location: null } }));
  await render({ problemType: "userInterface" }, errors);
  const client = lastQuery().client as Record<string, unknown>;
  expect(client).toMatchObject({ uiLanguage: "en", shell: "browser", uiScale: 1, viewport: { width: window.innerWidth, height: window.innerHeight } });
  expect((client.recentErrors as unknown[]).length).toBe(50);
  await next(); await next();
  const save = $<HTMLButtonElement>("[data-report-save]")!;
  expect(save.disabled).toBe(true);
  const mail = () => $<HTMLElement>("[data-report-mail]")!;
  expect(mail().tagName).toBe("BUTTON");
  expect((mail() as HTMLButtonElement).disabled).toBe(true);
  expect(document.querySelector(".report-problem a[href^='mailto:']")).toBeNull();
  await act(async () => save.click());
  expect(bundle.exportBundle).not.toHaveBeenCalled();
  const textarea = $<HTMLTextAreaElement>("[data-report-description]")!;
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")!.set!.call(textarea, "  Tab froze  ");
    textarea.dispatchEvent(new Event("input", { bubbles: true }));
  });
  await click($("[data-report-consent]"));
  expect(save.disabled).toBe(false);
  await click(save);
  expect(bundle.exportBundle).toHaveBeenCalledExactlyOnceWith({ ...lastQuery(), description: "Tab froze" }, "knxbench-diagnostics-0.1.0-20261029100000.zip");
  expect(saved).toHaveBeenCalledExactlyOnceWith(expect.any(Blob), "knxbench-diagnostics-0.1.0-20261029100007.zip");
  expect($("[data-report-saved]")!.textContent).toBe("Saved as knxbench-diagnostics-0.1.0-20261029100007.zip.");
  const link = mail() as HTMLAnchorElement;
  expect(link.tagName).toBe("A");
  expect(link.getAttribute("href")).toContain("knxbench-diagnostics-0.1.0-20261029100007.zip");
  expect(link.getAttribute("href")).not.toContain("Tab froze");
  expect(document.activeElement).toBe(link);
  expect(events).toEqual([{ type: "debugReportCreated" }]);
  expect($("[data-report-mail-note]")!.textContent).toBe(en["reportProblem.mailNote"]);
  const issue = $<HTMLAnchorElement>("[data-report-issue]")!;
  expect(issue.rel).toBe("noopener noreferrer");
  expect(new URL(issue.href).searchParams.get("body")).not.toContain("boom");
});

it("a changed selection asks again and withdraws consent and the saved state", async () => {
  bundle.exportBundle.mockResolvedValue({ blob: new Blob(["PK"]), fileName: "knxbench-diagnostics-0.1.0-20261029100000.zip" });
  await render({ problemType: "deviceReading" });
  await click(step(en["reportProblem.step.send"]));
  await click($("[data-report-consent]"));
  await click($("[data-report-save]"));
  expect($("[data-report-mail]")!.tagName).toBe("A");
  await click(step(en["reportProblem.step.contents"]));
  const calls = bundle.previewBundle.mock.calls.length;
  await click(box("project"));
  expect(bundle.previewBundle.mock.calls.length).toBe(calls + 1);
  expect(lastQuery().sections).toContain("project");
  await click(step(en["reportProblem.step.send"]));
  expect($<HTMLInputElement>("[data-report-consent]")!.checked).toBe(false);
  expect($("[data-report-mail]")!.tagName).toBe("BUTTON");
});

it("a refused export is shown and focused, and nothing is saved", async () => {
  bundle.exportBundle.mockRejectedValue(new Error("bundleSectionNotOffered"));
  await render({ problemType: "catalogue" });
  await click(step(en["reportProblem.step.send"]));
  await click($("[data-report-consent]"));
  await click($("[data-report-save]"));
  expect(saved).not.toHaveBeenCalled();
  expect(document.activeElement).toBe($("[data-report-save-failed]"));
  expect($("[data-report-save-failed]")!.textContent).toContain("bundleSectionNotOffered");
  expect(events).toEqual([]);
});

it("deletes recordings only after confirmation and reports what was kept", async () => {
  bundle.deleteRecordings.mockResolvedValue({ removed: 1, keptActive: ["a", "b"] });
  await render({ problemType: "deviceReading" });
  await click($("[data-report-delete]"));
  expect($("[data-report-delete-confirm]")!.getAttribute("role")).toBe("alert");
  await click(button(en["reportProblem.delete.keep"]));
  expect($("[data-report-delete-confirm]")).toBeNull();
  expect(bundle.deleteRecordings).not.toHaveBeenCalled();
  await click($("[data-report-delete]"));
  const calls = bundle.previewBundle.mock.calls.length;
  await click(button(en["reportProblem.delete.confirm"]));
  expect(bundle.deleteRecordings).toHaveBeenCalledTimes(1);
  expect($("[data-report-deleted]")!.textContent).toBe("Deleted 1 recording. 2 recordings are still being written and were kept.");
  expect(document.activeElement).toBe($("[data-report-deleted]"));
  expect(bundle.previewBundle.mock.calls.length).toBe(calls + 1);
});

it("drops an older preview answer that arrives after a newer one", async () => {
  let release!: (value: BundlePreview) => void;
  await render({ problemType: "deviceReading" });
  bundle.previewBundle.mockImplementationOnce((query: BundleQuery) => new Promise((resolve) => { release = () => resolve(answer(query, { totalBytes: 999999 })); }));
  await click(box("project"));
  await click(box("project"));
  await act(async () => release(answer({ problemType: "deviceReading" })));
  await flush();
  expect($("[data-report-total]")!.textContent).not.toContain("976.6 KiB");
});

it("closes through the Close button without exporting", async () => {
  await render({ problemType: "deviceProgramming" });
  await click(button(en["reportProblem.close"]));
  expect(onClose).toHaveBeenCalledTimes(1);
  expect(bundle.exportBundle).not.toHaveBeenCalled();
});

it("keeps working under React's development double mount (StrictMode)", async () => {
  bundle.exportBundle.mockResolvedValue({ blob: new Blob(["PK"]), fileName: "knxbench-diagnostics-0.1.0-20261029100000.zip" });
  bundle.sectionText.mockResolvedValue("text of the catalogue");
  await act(async () => root.render(<StrictMode><ReportProblemWizard request={{ problemType: "catalogue" }} tree={null} notifications={[]} onClose={onClose} /></StrictMode>));
  await flush();
  await click(step(en["reportProblem.step.preview"]));
  await click($("[data-report-preview=catalogue] [data-report-show]"));
  expect($("[data-report-preview=catalogue] pre")!.textContent).toBe("text of the catalogue");
  await click(step(en["reportProblem.step.send"]));
  await click($("[data-report-consent]"));
  await click($("[data-report-save]"));
  expect(saved).toHaveBeenCalledTimes(1);
  expect($("[data-report-mail]")!.tagName).toBe("A");
});
