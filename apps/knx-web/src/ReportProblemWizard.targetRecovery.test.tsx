/** Report problem wizard: recovery from a refused device target and withdrawn review authority. */
// @vitest-environment happy-dom
// Intended UI (the selectors below are the contract the correction must render):
//  - `[data-report-target-mismatch]`: notice shown for a typed 409 `bundleTargetMismatch`;
//  - `[data-report-without-operation]`: button that asks again with `operation: { selection: "none" }`;
//  - `[data-report-device]`: stays reachable in that state, so the user can also pick the run's device;
//  - `[data-report-refresh]`: offered only for conflicts a plain repeat can cure, never for a target mismatch.
import { act, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { BundlePreview, BundleQuery, OfferView, SectionKind } from "./diagnosticBundleApi";
const bundle = vi.hoisted(() => ({ previewBundle: vi.fn(), sectionText: vi.fn(), exportBundle: vi.fn(), deleteRecordings: vi.fn() }));
vi.mock("./diagnosticBundleApi", async (importOriginal) => ({ ...(await importOriginal<typeof import("./diagnosticBundleApi")>()), ...bundle }));
const saved = vi.hoisted(() => vi.fn());
vi.mock("./localJsonDownload", () => ({ downloadLocalBlob: saved }));
vi.mock("./diagnosticApi", () => ({
  IDENTITY_PROFILE: "generic-identity-v1", MEMORY_PROFILE: "system2-declared-segments-v1", DiagnosticRefusal: class extends Error {},
  previewDiagnostics: vi.fn(), startDiagnostics: vi.fn(), pollDiagnostics: vi.fn(), cancelDiagnostics: vi.fn(), saveDiagnostics: vi.fn(),
}));
import DeviceDiagnosticsEntry from "./DeviceDiagnosticsEntry";
import { ReportProblemProvider } from "./ReportProblemEntry";
import ReportProblemWizard from "./ReportProblemWizard";
import { subscribeAchievementEvents, type AchievementEvent } from "./achievementEvents";
import { messages as en } from "./messages/en";
import { messages as de } from "./messages/de";
import { defaultOffer, defaultSelection, type ReportProblemRequest } from "./reportProblem";
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const DEVICE_A = 9;
const DEVICE_B = 10;
const CONTEXT = { serverIncarnation: "server-a", projectIncarnation: 1, revision: 2 };
const REVIEW = { fingerprint: "a".repeat(64), context: CONTEXT };
const MARKERS = { addresses: true, names: true, parameterValues: false, networkAddresses: true };
const FILE_NAME = "knxbench-diagnostics-0.1.0-20261029100000.zip";
/** The latest finished diagnostic run; it read device A, in the same server, project and revision as the tree. */
const RUN_A = { id: "20261029T100000Z-device-diagnostics-aa", sizeBytes: 2048, formatVersion: 1 as const,
  active: false, tornTail: false, end: { state: "known" as const, outcome: "succeeded" as const } };

const device = (id: number, address: string) => ({ id, name: `Synthetic device ${id}`, address, description: null, com_object_count: 0 });
/** One server, one project, one revision; two devices, so only the chosen device differs between requests. */
const TREE: ProjectTree = {
  schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false, group_address_style: "ThreeLevel",
  installations: [{ id: 1, name: "Installation", topology: [{ id: 2, name: "Area", address: 1, lines: [{ id: 3, name: "Line", address: 1,
    devices: [device(DEVICE_A, "1.1.9"), device(DEVICE_B, "1.1.10")] }] }], buildings: [], unassigned: [], group_addresses: [], group_ranges: [] }],
  server_incarnation: CONTEXT.serverIncarnation, project_incarnation: CONTEXT.projectIncarnation, snapshot_revision: CONTEXT.revision,
};

/** The real client error shape: `request()` throws `Error(error-string)` with `status` and the parsed `body`, no other code field. */
const conflict = (code: string) => Object.assign(new Error(code), { status: 409, body: { error: code } });

/** The server's answer for `query`, resolving the recording the way the request asks for it. */
function answer(query: BundleQuery, operationRecordingId: string | null): BundlePreview {
  const wanted = new Set(query.sections ?? defaultSelection(query.problemType));
  const offers: OfferView[] = [];
  const sections: BundlePreview["sections"] = [];
  for (const kind of ["environment", "sessionLog", "operation", "earlierOperation", "device", "legacyReports", "importReport", "importFile",
    "catalogue", "project", "busMonitor", "clientState"] as SectionKind[]) {
    const offer = defaultOffer(query.problemType, kind);
    if (offer === "notOffered") continue;
    const selected = offer === "fixed" || wanted.has(kind);
    offers.push({ kind, offer, selected, unavailable: null });
    if (selected) sections.push({ key: kind, kind, label: null, bytes: 1000 + offers.length, entries: offers.length, markers: MARKERS, notes: [], files: [`${kind}.json`] });
  }
  return { review: REVIEW, problemType: query.problemType, recordingEnabled: true, operationRecordingId, deviceId: query.deviceId ?? null,
    offers, sections, totalBytes: sections.reduce((sum, s) => sum + s.bytes, 0), exceedsMailHint: false, fileName: FILE_NAME,
    mailDraft: `mailto:issue@knxbench.com?subject=KNXBench&body=Please%20attach%20the%20saved%20file%20${FILE_NAME}%20to%20this%20mail.`,
    recordings: [RUN_A] };
}

/**
 * The backend guard, unchanged: with no operation named the newest run (A) is adopted, and a device other
 * than that run's target is refused as a typed 409. An explicit recording or `selection: none` is accepted.
 */
async function guardedPreview(query: BundleQuery): Promise<BundlePreview> {
  const automatic = query.operation === undefined;
  if (automatic && query.deviceId !== undefined && query.deviceId !== DEVICE_A) throw conflict("bundleTargetMismatch");
  const operation = query.operation;
  return answer(query, automatic ? RUN_A.id : operation && "recordingId" in operation ? operation.recordingId : null);
}

let host: HTMLDivElement; let root: Root; let onClose: ReturnType<typeof vi.fn<() => void>>;
const events: AchievementEvent[] = []; let unsubscribe: () => void;
beforeEach(() => {
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
  for (const mock of Object.values(bundle)) mock.mockReset();
  saved.mockReset(); onClose = vi.fn<() => void>(); events.length = 0;
  unsubscribe = subscribeAchievementEvents((event) => events.push(event));
  bundle.previewBundle.mockImplementation(guardedPreview);
});
afterEach(async () => { unsubscribe(); await act(async () => root.unmount()); host.remove(); document.body.innerHTML = ""; });

const flush = () => act(async () => { for (let i = 0; i < 6; i++) await Promise.resolve(); });
async function render(request: ReportProblemRequest) {
  await act(async () => root.render(<ReportProblemWizard key={JSON.stringify(request)} request={request} tree={TREE} notifications={[]} onClose={onClose} />));
  await flush();
}
const $ = <T extends Element = HTMLElement>(selector: string) => document.querySelector<T>(selector);
async function click(target: HTMLElement | null) { expect(target).not.toBeNull(); await act(async () => target!.click()); await flush(); }
const step = (label: string) => [...document.querySelectorAll<HTMLButtonElement>(".project-wizard-step-link")].find((b) => b.textContent!.endsWith(label))!;
const goTo = (label: string) => click(step(label));
const lastQuery = () => bundle.previewBundle.mock.calls.at(-1)![0] as BundleQuery;
const box = (kind: string) => $<HTMLInputElement>(`[data-report-section=${kind}] input[type=checkbox]`)!;
async function chooseDevice(id: number) {
  await act(async () => {
    const select = $<HTMLSelectElement>("[data-report-device]")!;
    select.value = String(id);
    select.dispatchEvent(new Event("change", { bubbles: true }));
  });
  await flush();
}
/** The first request of a device report: reading, device B, no operation named. */
const FOR_B = { problemType: "deviceReading", sections: defaultSelection("deviceReading"), deviceId: DEVICE_B, context: CONTEXT } as const;
const FOR_A = { ...FOR_B, deviceId: DEVICE_A };
const NO_OPERATION = { selection: "none" } as const;

/** Refused review: no step beyond the contents, no consent, no text, no mail, nothing exported. */
function expectNoAuthority() {
  expect(step(en["reportProblem.step.preview"]).disabled).toBe(true);
  expect(step(en["reportProblem.step.send"]).disabled).toBe(true);
  expect($("[data-report-next]")?.hasAttribute("disabled") ?? true).toBe(true);
  expect($("[data-report-consent]")).toBeNull();
  expect($("[data-report-save]")).toBeNull();
  expect($("[data-report-file-confirm]")).toBeNull();
  expect($("[data-report-requested]")).toBeNull();
  expect($("a[data-report-mail]")).toBeNull();
  expect($("[data-report-text]")).toBeNull();
}
/** The typed target refusal leaves only corrections: the explicit no-operation reset and the device choice. */
function expectTargetRecovery() {
  expect($("[data-report-device]")).not.toBeNull();
  expect($("[data-report-without-operation]")).not.toBeNull();
  const notice = $("[data-report-target-mismatch]");
  expect(notice).not.toBeNull();
  expect(notice!.textContent!.trim()).not.toBe("");
  expect($<HTMLButtonElement>("[data-report-without-operation]")!.tagName).toBe("BUTTON");
  expect($<HTMLSelectElement>("[data-report-device]")!.value).toBe(String(DEVICE_B));
  // A plain repeat would send the same refused query again.
  expect($("[data-report-refresh]")).toBeNull();
  expectNoAuthority();
}
async function sendReport() {
  await goTo(en["reportProblem.step.send"]);
  await click($("[data-report-consent]"));
  await click($("[data-report-save]"));
  await click($("[data-report-file-confirm]"));
}

describe("a refused device target", () => {
  it("on the first preview stays correctable: no operation resets to selection none and keeps device B", async () => {
    await render({ problemType: "deviceReading", deviceId: DEVICE_B });
    expect(bundle.previewBundle).toHaveBeenCalledExactlyOnceWith(FOR_B);
    expectTargetRecovery();
    // The refused query is not repeated by itself.
    await flush();
    expect(bundle.previewBundle).toHaveBeenCalledTimes(1);
    expect(bundle.sectionText).not.toHaveBeenCalled(); expect(bundle.exportBundle).not.toHaveBeenCalled();

    await click($("[data-report-without-operation]"));
    expect(bundle.previewBundle).toHaveBeenCalledTimes(2);
    expect(lastQuery()).toEqual({ ...FOR_B, operation: NO_OPERATION });
    expect($("[data-report-target-mismatch]")).toBeNull();
    expect($("[data-report-without-operation]")).toBeNull();
    expect($<HTMLSelectElement>("[data-report-device]")!.value).toBe(String(DEVICE_B));
    // Accepted for real: the wizard is open on its contents and the later steps are reachable without closing.
    expect(document.querySelector("[aria-current=step]")!.textContent).toContain(en["reportProblem.step.contents"]);
    expect(step(en["reportProblem.step.preview"]).disabled).toBe(false);
    expect(step(en["reportProblem.step.send"]).disabled).toBe(false);
    expect(onClose).not.toHaveBeenCalled();
  });

  it("on the first preview can also be corrected by choosing the device the run read", async () => {
    await render({ problemType: "deviceReading", deviceId: DEVICE_B });
    expectTargetRecovery();
    await chooseDevice(DEVICE_A);
    expect(lastQuery()).toEqual(FOR_A);
    expect($("[data-report-target-mismatch]")).toBeNull();
    expect($<HTMLSelectElement>("[data-report-operation]")!.value).toBe(RUN_A.id);
    expect(step(en["reportProblem.step.send"]).disabled).toBe(false);
  });

  it("after a recovered first preview needs fresh consent and exports exactly the corrected, reviewed query", async () => {
    bundle.exportBundle.mockResolvedValue({ blob: new Blob(["PK"]), fileName: FILE_NAME });
    await render({ problemType: "deviceReading", deviceId: DEVICE_B });
    await click($("[data-report-without-operation]"));
    await goTo(en["reportProblem.step.send"]);
    expect($<HTMLInputElement>("[data-report-consent]")!.checked).toBe(false);
    expect($<HTMLButtonElement>("[data-report-save]")!.disabled).toBe(true);
    expect($("[data-report-mail]")!.tagName).toBe("BUTTON");
    expect(bundle.exportBundle).not.toHaveBeenCalled();
    await click($("[data-report-consent]")); await click($("[data-report-save]"));
    expect(bundle.exportBundle).toHaveBeenCalledExactlyOnceWith({ ...FOR_B, operation: NO_OPERATION, review: REVIEW }, FILE_NAME);
    expect(bundle.previewBundle).toHaveBeenCalledTimes(2);
    expect(events).toEqual([]);
  });

  it("after an ordinary run A, changing the dropdown to B withdraws the finished flow and stays correctable", async () => {
    bundle.exportBundle.mockResolvedValue({ blob: new Blob(["PK"]), fileName: FILE_NAME });
    await render({ problemType: "deviceReading", deviceId: DEVICE_A });
    expect(lastQuery()).toEqual(FOR_A);
    expect($<HTMLSelectElement>("[data-report-operation]")!.value).toBe(RUN_A.id);
    await sendReport();
    expect($("a[data-report-mail]")).not.toBeNull();
    expect(saved).toHaveBeenCalledTimes(1);
    expect(events).toEqual([{ type: "debugReportCreated" }]);

    await goTo(en["reportProblem.step.contents"]);
    await chooseDevice(DEVICE_B);
    expect(lastQuery()).toEqual(FOR_B);
    expectTargetRecovery();
    expect(bundle.previewBundle).toHaveBeenCalledTimes(2);
    await flush();
    expect(bundle.previewBundle).toHaveBeenCalledTimes(2);
    expect(bundle.exportBundle).toHaveBeenCalledTimes(1);
    expect(events).toEqual([{ type: "debugReportCreated" }]);

    await click($("[data-report-without-operation]"));
    expect(lastQuery()).toEqual({ ...FOR_B, operation: NO_OPERATION });
    expect($<HTMLSelectElement>("[data-report-device]")!.value).toBe(String(DEVICE_B));
    await goTo(en["reportProblem.step.send"]);
    // The earlier consent, download and acknowledgement belonged to run A's contents.
    expect($<HTMLInputElement>("[data-report-consent]")!.checked).toBe(false);
    expect($<HTMLButtonElement>("[data-report-save]")!.disabled).toBe(true);
    expect($("[data-report-requested]")).toBeNull();
    expect($("[data-report-file-confirm]")).toBeNull();
    expect($("[data-report-mail]")!.tagName).toBe("BUTTON");
    await click($("[data-report-consent]")); await click($("[data-report-save]"));
    expect(bundle.exportBundle).toHaveBeenCalledTimes(2);
    expect(bundle.exportBundle.mock.calls[1][0]).toEqual({ ...FOR_B, operation: NO_OPERATION, review: REVIEW });
  });

  it("after an ordinary run A, going to B and back to A recovers without the reset", async () => {
    await render({ problemType: "deviceReading", deviceId: DEVICE_A });
    await chooseDevice(DEVICE_B);
    expectTargetRecovery();
    await chooseDevice(DEVICE_A);
    expect(lastQuery()).toEqual(FOR_A);
    expect($("[data-report-target-mismatch]")).toBeNull();
    expect($<HTMLSelectElement>("[data-report-operation]")!.value).toBe(RUN_A.id);
    expect(step(en["reportProblem.step.send"]).disabled).toBe(false);
  });
});

describe("the device Diagnostics report button", () => {
  /** The real producer and provider: the button opens the one wizard for this device through the workbench host. */
  function Workbench() {
    const [request, setRequest] = useState<ReportProblemRequest | null>(null);
    return <ReportProblemProvider request={request} onOpen={setRequest} onClose={() => setRequest(null)} tree={TREE} notifications={[]}>
      <DeviceDiagnosticsEntry project={TREE} deviceId={DEVICE_B} />
    </ReportProblemProvider>;
  }

  it("asks for no operation, so the newest run of another device is never adopted for it", async () => {
    await act(async () => root.render(<Workbench />));
    expect(bundle.previewBundle).not.toHaveBeenCalled();
    await click($("[data-device-report]"));
    expect(bundle.previewBundle).toHaveBeenCalledExactlyOnceWith({ ...FOR_B, operation: NO_OPERATION });
    expect($("[data-report-target-mismatch]")).toBeNull();
    expect($<HTMLSelectElement>("[data-report-device]")!.value).toBe(String(DEVICE_B));
    expect(step(en["reportProblem.step.send"]).disabled).toBe(false);
  });
});

describe("an explicit older reference", () => {
  const older = { ...RUN_A, id: "20261029T090000Z-device-diagnostics-bb" };
  const references = [
    { recordingId: older.id },
    { relatedKey: "diagnosticsToken", relatedValue: "synthetic-older-retained-result" },
  ] as const;

  for (const reference of references) {
    it(`explains a conflict without claiming the newest run for ${JSON.stringify(reference)}`, async () => {
      bundle.previewBundle.mockImplementation(async (query: BundleQuery) => {
        expect(query.operation).toEqual(reference);
        if (query.deviceId === DEVICE_B) throw conflict("bundleTargetMismatch");
        const recordingId = "recordingId" in reference ? reference.recordingId : null;
        return { ...answer(query, recordingId), deviceId: query.deviceId ?? DEVICE_A, recordings: [RUN_A, older] };
      });
      await render({ problemType: "deviceReading", deviceId: DEVICE_B, operation: reference });
      expect(lastQuery().operation).toEqual(reference);
      expectTargetRecovery();
      expect($("[data-report-target-mismatch]")!.textContent).not.toMatch(/\bnewest\b/i);

      // Choosing the operation's target automatically must retain the explicitly selected reference.
      await act(async () => {
        const select = $<HTMLSelectElement>("[data-report-device]")!;
        select.value = "";
        select.dispatchEvent(new Event("change", { bubbles: true }));
      });
      await flush();
      expect(lastQuery()).not.toHaveProperty("deviceId");
      expect(lastQuery().operation).toEqual(reference);
      expect($("[data-report-target-mismatch]")).toBeNull();
      expect(onClose).not.toHaveBeenCalled();
      await goTo(en["reportProblem.step.send"]);
      expect($<HTMLInputElement>("[data-report-consent]")!.checked).toBe(false);
      expect($<HTMLButtonElement>("[data-report-save]")!.disabled).toBe(true);
      expect(bundle.exportBundle).not.toHaveBeenCalled();
      expect(saved).not.toHaveBeenCalled();
      expect(events).toEqual([]);
    });
  }

  it("does not attribute the German refusal to the newest recorded operation", () => {
    expect(de["reportProblem.targetMismatch"]).toContain("Vorgang");
    expect(de["reportProblem.targetMismatch"]).toContain("Gerät");
    expect(de["reportProblem.targetMismatch"]).not.toMatch(/\bneuest(?:e|en|er|es)\b/i);
  });
});

describe("any other 409 on a preview", () => {
  for (const code of ["bundlePreviewChanged", "someNewConflict"]) {
    it(`${code} on the first preview withdraws, offers only a refresh and repeats the same query`, async () => {
      bundle.previewBundle.mockRejectedValueOnce(conflict(code));
      await render({ problemType: "deviceReading", deviceId: DEVICE_A });
      expect($("[role=alert]")!.textContent).toContain(en["reportProblem.reviewChanged"]);
      expect($("[data-report-target-mismatch]")).toBeNull();
      expect($("[data-report-without-operation]")).toBeNull();
      expect($("[data-report-refresh]")).not.toBeNull();
      expectNoAuthority();
      await click($("[data-report-refresh]"));
      expect(bundle.previewBundle).toHaveBeenCalledTimes(2);
      expect(bundle.previewBundle.mock.calls[1][0]).toEqual(bundle.previewBundle.mock.calls[0][0]);
      expect(step(en["reportProblem.step.send"]).disabled).toBe(false);
    });

    it(`${code} on a later preview withdraws the finished flow and needs fresh consent`, async () => {
      bundle.exportBundle.mockResolvedValue({ blob: new Blob(["PK"]), fileName: FILE_NAME });
      await render({ problemType: "deviceReading", deviceId: DEVICE_A });
      await sendReport();
      expect(events).toEqual([{ type: "debugReportCreated" }]);
      await goTo(en["reportProblem.step.contents"]);
      bundle.previewBundle.mockRejectedValueOnce(conflict(code));
      await click(box("project"));
      expect($("[data-report-target-mismatch]")).toBeNull();
      expect($("[data-report-without-operation]")).toBeNull();
      expectNoAuthority();
      const refused = lastQuery();
      await click($("[data-report-refresh]"));
      expect(lastQuery()).toEqual(refused);
      await goTo(en["reportProblem.step.send"]);
      expect($<HTMLInputElement>("[data-report-consent]")!.checked).toBe(false);
      expect($("[data-report-mail]")!.tagName).toBe("BUTTON");
      expect(events).toEqual([{ type: "debugReportCreated" }]);
    });
  }
});

describe("a 409 while reviewing, saving or after a finished export", () => {
  for (const code of ["bundlePreviewChanged", "bundleTargetMismatch", "someNewConflict"]) {
    /** Whatever the typed code, authority is gone; the user is left with a way to ask again. */
    const recover = () => $("[data-report-refresh]") ?? $("[data-report-without-operation]");

    it(`${code} on a second ZIP request withdraws consent, download state, acknowledgement and mail`, async () => {
      bundle.exportBundle.mockResolvedValueOnce({ blob: new Blob(["PK"]), fileName: FILE_NAME });
      await render({ problemType: "deviceReading", deviceId: DEVICE_A });
      await sendReport();
      expect($("a[data-report-mail]")).not.toBeNull();
      bundle.exportBundle.mockRejectedValueOnce(conflict(code));
      await click($("[data-report-save]"));
      expect(bundle.exportBundle).toHaveBeenCalledTimes(2);
      expect(saved).toHaveBeenCalledTimes(1);
      expect(events).toEqual([{ type: "debugReportCreated" }]);
      expect($("[data-report-save-failed]")).toBeNull();
      expectNoAuthority();
      expect(recover()).not.toBeNull();
      await click(recover());
      await goTo(en["reportProblem.step.send"]);
      expect($<HTMLInputElement>("[data-report-consent]")!.checked).toBe(false);
      expect($<HTMLButtonElement>("[data-report-save]")!.disabled).toBe(true);
      expect($("[data-report-requested]")).toBeNull();
      expect($("[data-report-mail]")!.tagName).toBe("BUTTON");
    });

    it(`${code} on section text withdraws authority and a ZIP finishing late cannot resurrect it`, async () => {
      let release!: (value: { blob: Blob; fileName: string }) => void;
      bundle.exportBundle.mockReturnValue(new Promise((resolve) => { release = resolve; }));
      bundle.sectionText.mockRejectedValueOnce(conflict(code));
      await render({ problemType: "deviceReading", deviceId: DEVICE_A });
      await goTo(en["reportProblem.step.send"]);
      await click($("[data-report-consent]")); await click($("[data-report-save]"));
      expect(bundle.exportBundle).toHaveBeenCalledTimes(1);
      await goTo(en["reportProblem.step.preview"]);
      await click($("[data-report-show=sessionLog]"));
      expectNoAuthority();
      await act(async () => release({ blob: new Blob(["PK"]), fileName: "late.zip" }));
      await flush();
      expect(saved).not.toHaveBeenCalled(); expect(events).toEqual([]);
      expectNoAuthority();
      await click(recover());
      await goTo(en["reportProblem.step.send"]);
      expect($<HTMLInputElement>("[data-report-consent]")!.checked).toBe(false);
      expect($("[data-report-requested]")).toBeNull();
      expect($("[data-report-mail]")!.tagName).toBe("BUTTON");
    });

    it(`${code} on the ZIP withdraws authority and section text finishing late cannot resurrect it`, async () => {
      let release!: (value: string) => void;
      bundle.sectionText.mockReturnValue(new Promise((resolve) => { release = resolve; }));
      bundle.exportBundle.mockRejectedValueOnce(conflict(code));
      await render({ problemType: "deviceReading", deviceId: DEVICE_A });
      await goTo(en["reportProblem.step.preview"]);
      await click($("[data-report-show=sessionLog]"));
      await goTo(en["reportProblem.step.send"]);
      await click($("[data-report-consent]")); await click($("[data-report-save]"));
      expectNoAuthority();
      await act(async () => release("late old text"));
      await flush();
      expectNoAuthority();
      expect(saved).not.toHaveBeenCalled(); expect(events).toEqual([]);
      await click(recover());
      await goTo(en["reportProblem.step.preview"]);
      expect($("[data-report-text]")).toBeNull();
      expect($("[data-report-show=sessionLog]")!.getAttribute("aria-expanded")).toBe("false");
    });
  }
});
