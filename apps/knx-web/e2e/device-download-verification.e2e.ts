/** ADR-0121 in a browser: plan warnings, then a finished download's check, report and hand-over. */
import { expect, test, type Locator, type Page } from "@playwright/test";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";
// Hand-written check records the Rust serializer admits and re-emits unchanged
// (`the_web_fixtures_are_the_serializer_vocabulary`); synthetic, not live evidence.
import checkFixtures from "../src/postDownloadCheck.fixtures.json" with { type: "json" };

const device = (id: number, address: string, name: string) => ({ id, name, address, description: null, com_object_count: 0 });
const tree = {
  schema_version: 11, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false,
  group_address_style: "ThreeLevel", server_incarnation: "download-verification", snapshot_revision: 1,
  installations: [{
    id: 1, name: "Installation 1", buildings: [], unassigned: [], group_addresses: [], group_ranges: [],
    topology: [{ id: 2, name: "Area", address: 1, lines: [{ id: 3, name: "Line", address: 1,
      devices: [device(10, "1.1.67", "Fictional actuator")] }] }],
  }],
};
const LOW_FIRST = "base-image-low-octet-first: the program declares no ParameterByteOrder (the schema default is BigEndian), so KNXBench writes multi-octet values high octet first; its own base image holds 3 multi-octet default(s) low octet first and none high octet first. Whether the device reads them high octet first is not known from the product data: check the device after the download (KNOWN_LIMITATIONS §170, ADR-0120)";
const INFERENCE = "union-later-member: UP-33_R-33 is written after UP-32_R-32 (RESEARCH.md §19.20)";
const plan = {
  planId: 7, address: "1.1.67", deviceId: 10, deviceName: "Fictional actuator",
  programId: "M-0002_A-A060-14-0000", maskVersion: 0x0701, manufacturer: 0x0002,
  parameterValues: 3, groupLinks: 1,
  segments: [{ id: "M-0002_A-A060-14-0000_AS-4400", address: 0x4400, size: 32, written: 32 }],
  dataOctets: 32, steps: ["connect", "check the mask", "write 4400h"],
  confirmationPhrase: "I confirm download to 1.1.67", accessKey: "none",
  support: { level: "untested", evidence: null, inferences: [INFERENCE] },
  untestedAcknowledgement: "I accept an untested download to 1.1.67",
  partial: false, notWritten: [], warnings: [LOW_FIRST],
};
const finished = {
  downloadId: 7, address: "1.1.67", deviceName: "Fictional actuator", steps: 3, dataOctets: 32,
  status: { state: "finished", written: "yes", restart: "acknowledged", restartNote: null },
  nextSince: 32, events: Array.from({ length: 32 }, (_, index) => ({ kind: "dataWritten", number: 3,
    address: 0x4400 + index, octets: [index], written: index + 1, of: 32 })), backupFile: "device-backups/1.1.67_fixture.json",
};
const report = {
  report: {
    kind: "knxbench-download-report", format: 1, knxbenchVersion: "0.1.0-alpha.10+gfixture",
    program: { id: "M-0002_A-A060-14-0000", name: "Fictional program", applicationVersion: "20", sourceSha256: "ab".repeat(32), manufacturer: "0002", mask: "0701" },
    scope: "complete", support: "untested", inferences: [INFERENCE], warnings: [LOW_FIRST],
    plan: { steps: 3, dataOctets: 32, groupLinks: 1 },
    outcome: { completed: true, written: "yes", restart: "acknowledged", stoppedInStep: null },
    parameterValues: { "P-W16_R-1": "300" },
  },
  fileName: "knxbench-download-report-M-0002_A-A060-14-0000.json",
  summary: "KNXBench download report (format 1)\nProgram: M-0002_A-A060-14-0000 — Fictional program (version 20)",
  issueUrl: "https://github.com/KNXBench-Labs/KNXBench/issues/new?template=download-report.yml&title=%5BDownload%5D%20M-0002_A-A060-14-0000&program=M-0002_A-A060-14-0000",
  mailto: "mailto:issue@knxbench.com?subject=KNXBench%20download%20report%3A%20M-0002_A-A060-14-0000&body=KNXBench",
};

/** What the intercepted server answers once the download ran; the default is an older server's. */
interface Served { status: unknown; report: unknown; history?: unknown }

async function open(page: Page, language: "en" | "de", served: Served = { status: finished, report }) {
  const m = language === "en" ? en : de;
  const state = { started: false, starts: 0, writes: [] as string[], reportReads: 0 };
  // Like the real server: no project until the test creates one (ADR-0127).
  let projectOpen = false;
  await page.route("**/api/**", (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    const method = request.method();
    const json = (body: unknown, status = 200) => route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
    if (path === "/api/auth/status") return json({ required: false, authenticated: true });
    if (path === "/api/settings" && method === "GET") return json({ schemaVersion: 1, status: "ok", settings: { uiLanguage: language } });
    if (path === "/api/project" && method === "GET") return projectOpen ? json(tree) : json({ error: "no project open" }, 400);
    if (path === "/api/project/new" && method === "POST") { projectOpen = true; return json(tree); }
    if (path === "/api/bus/discover") return json({ interfaces: [] });
    if (path === "/api/bus/activity" && method === "GET") return json({ serverIncarnation: "fixture", coverage: "partial", historyState: "disabled", sessions: [], oneShot: [], oneShotDropped: 0, busyLocks: [], untracked: [] });
    if (path === "/api/device-readiness" && method === "GET") return json({ devices: [], counts: {} });
    if (path === "/api/device-download/plan" && method === "POST") return json(plan);
    if (path === "/api/device-download/start" && method === "POST") {
      const body = request.postDataJSON() as { acceptUntested?: string };
      if (body.acceptUntested !== plan.untestedAcknowledgement) return json({ error: "acknowledgement" }, 400);
      state.started = true;
      state.starts += 1;
      return json({ downloadId: 7 });
    }
    if (path === "/api/device-download/status" && method === "GET") {
      return state.started ? json(served.status) : json({ error: "no download to a device yet" }, 404);
    }
    if (path === "/api/device-download/report" && method === "GET") {
      state.reportReads += 1;
      return json(served.report);
    }
    if (path === "/api/bus/history" && method === "GET" && served.history !== undefined) return json(served.history);
    if (method === "GET") return json([]);
    state.writes.push(`${method} ${path}`);
    return json({ error: "not in this fixture" }, 404);
  });
  await page.goto("/");
  await page.getByRole("button", { name: m["toolbar.newProject"], exact: true }).first().click();
  await page.getByRole("dialog", { name: m["newProject.title"], exact: true }).getByRole("button", { name: m["newProject.create"], exact: true }).click();
  await page.getByRole("dialog", { name: m["projectWizard.done.title"], exact: true }).getByRole("button", { name: m["projectWizard.done.close"], exact: true }).click();
  const toasts = page.locator(".toast:not(.toast--leaving)").getByRole("button", { name: m["toast.dismiss"], exact: true });
  for (let round = 0; round < 20 && await toasts.count() > 0; round++) await toasts.first().click();
  await page.keyboard.press("Control+Shift+P");
  await page.getByRole("combobox").fill(m["toolbar.busMonitor"]);
  await page.getByRole("combobox").press("Enter");
  await page.getByRole("button", { name: m["deviceDownload.title"], exact: true }).click();
  return state;
}

for (const language of ["en", "de"] as const) {
  for (const width of [1440, 400]) {
    test(`${language} ${width}: warnings before the write, then a report the tester hands over by hand`, async ({ page }) => {
      const m = language === "en" ? en : de;
      await page.setViewportSize({ width, height: 900 });
      const state = await open(page, language);
      const panel = page.locator(".device-download-panel");
      await panel.locator("select[data-download-device]").selectOption({ label: "1.1.67 — Fictional actuator" });
      await panel.locator("input[data-download-gateway]").fill("192.0.2.10:3671");
      await panel.getByRole("button", { name: m["deviceDownload.preparePlan"], exact: true }).click();

      const shown = panel.getByRole("region", { name: m["deviceDownload.planTitle"] });
      const warnings = shown.getByRole("region", { name: m["deviceDownload.warnings.title"] });
      await expect(warnings.locator('[data-plan-warning="base-image-low-octet-first"]')).toHaveText(m["deviceDownload.warning.baseImageLowOctetFirst"]);
      await expect(shown.getByRole("region", { name: m["deviceDownload.inferences.title"] })).toContainText(INFERENCE);
      await expect(shown.locator("[data-verify-hint]")).toHaveText(m["deviceDownload.verify.planHint"]);
      await expect(panel.locator(".device-download-verify")).toHaveCount(0);
      if (process.env.KNX_VERIFY_CAPTURE_DIR) {
        await warnings.screenshot({ path: join(process.env.KNX_VERIFY_CAPTURE_DIR, `${language}-${width}-warnings.png`) });
      }

      await shown.locator("input[data-untested-acknowledgement]").fill(plan.untestedAcknowledgement);
      await shown.getByRole("button", { name: m["deviceDownload.start"].replace("{address}", "1.1.67"), exact: true }).click();
      await page.locator(".programming-consent-dialog").getByRole("button", { name: m["programmingConsent.confirm"], exact: true }).click();

      const blocks = panel.locator('[data-table-id="download-blocks"]');
      await expect(blocks.locator("tbody tr")).toHaveCount(32);
      const geometryBlocks = await blocks.locator("table").evaluate(table => ({
        display: getComputedStyle(table).display, layout: getComputedStyle(table).tableLayout,
        viewport: table.parentElement!.classList.contains("device-download-blocks-scroll"),
        height: table.parentElement!.clientHeight, scroll: table.parentElement!.scrollHeight,
      }));
      expect(geometryBlocks.display).toBe("table"); expect(geometryBlocks.layout).toBe("fixed");
      expect(geometryBlocks.viewport).toBe(true); expect(geometryBlocks.height).toBeLessThanOrEqual(320);
      expect(geometryBlocks.scroll).toBeGreaterThan(geometryBlocks.height);
      const verify = panel.getByRole("region", { name: m["deviceDownload.verify.title"] });
      await expect(verify).toContainText(m["deviceDownload.verify.intro.untested"]);
      await expect(verify.locator(".device-download-verify-steps li")).toHaveCount(5);
      // An older server's status has no check: it reads as not checked.
      const legacyCheck = panel.locator(".post-download-check");
      await expect(legacyCheck).toHaveAttribute("data-check", "notChecked");
      await expect(legacyCheck).toContainText(m["postDownloadCheck.notChecked"]);
      expect(state.reportReads, "the report is created on request only").toBe(0);
      await verify.getByRole("button", { name: m["deviceDownload.verify.create"], exact: true }).click();
      await expect(verify.locator("[data-check]")).toHaveCount(4);
      await expect(verify.locator('[data-check="multiOctet"]')).toContainText(m["deviceDownload.verify.check.multiOctet"]);
      await expect(verify.locator("a[data-verify-issue]")).toHaveAttribute("href", report.issueUrl);
      await expect(verify.locator("a[data-verify-issue]")).toHaveAttribute("rel", "noopener noreferrer");
      await expect(verify.locator("a[data-verify-mail]")).toHaveAttribute("href", report.mailto);

      const download = page.waitForEvent("download");
      await verify.getByRole("button", { name: m["deviceDownload.verify.save"], exact: true }).click();
      const saved = await download;
      expect(saved.suggestedFilename()).toBe(report.fileName);
      expect(JSON.parse(await readFile(await saved.path(), "utf8"))).toEqual(report.report);

      await verify.scrollIntoViewIfNeeded();
      if (process.env.KNX_VERIFY_CAPTURE_DIR) {
        await verify.screenshot({ path: join(process.env.KNX_VERIFY_CAPTURE_DIR, `${language}-${width}-verify.png`) });
      }
      const geometry = await verify.evaluate((element) => {
        const box = element.getBoundingClientRect();
        const children = [...element.querySelectorAll("button, a, label, li, p")].map((node) => node.getBoundingClientRect());
        return {
          left: box.left, right: box.right,
          inside: children.every((child) => child.left >= box.left - 1 && child.right <= box.right + 1),
          overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
        };
      });
      expect(geometry.left).toBeGreaterThanOrEqual(0);
      expect(geometry.right).toBeLessThanOrEqual(width);
      expect(geometry.inside, "every control stays inside the section").toBe(true);
      expect(geometry.overflow, "no horizontal scrolling").toBeLessThanOrEqual(1);
      expect(state.writes, "only the start was sent").toEqual([]);
    });
  }
}

type CheckFixture = keyof typeof checkFixtures;
type Messages = typeof en;
type Reading = { machine: string; expected: string | null; observed: { result: string; loadState?: string } };
/** A finished download whose status carries the check, with the restart it reports. */
const CHECKS: { fixture: CheckFixture; restart: "acknowledged" | "unconfirmed"; lines: (keyof Messages)[] }[] = [
  { fixture: "matched", restart: "acknowledged",
    lines: ["postDownloadCheck.reach.reached", "postDownloadCheck.identity.match", "postDownloadCheck.verdict.matchesPlan"] },
  { fixture: "partialReading", restart: "acknowledged",
    lines: ["postDownloadCheck.reach.reached", "postDownloadCheck.identity.match", "postDownloadCheck.verdict.unknown"] },
  { fixture: "identityMismatch", restart: "unconfirmed",
    lines: ["postDownloadCheck.reach.reached", "postDownloadCheck.identity.mismatch.application"] },
  { fixture: "runStateRunning", restart: "acknowledged",
    lines: ["postDownloadCheck.reach.reached", "postDownloadCheck.identity.match", "postDownloadCheck.verdict.matchesPlan",
      "postDownloadCheck.runState.state.running", "postDownloadCheck.runState.scope"] },
  { fixture: "runStateUndefined", restart: "acknowledged",
    lines: ["postDownloadCheck.identity.match", "postDownloadCheck.runState.undefined"] },
];

function readingsOf(check: unknown): Reading[] {
  const reach = (check as { observation: { reach?: { identity?: { loadStates?: Reading[] } } } }).observation.reach;
  return reach?.identity?.loadStates ?? [];
}

/** Whether the fixture's own evidence carries a run-state record (a legacy record has none). */
function hasRunStateRecord(check: unknown): boolean {
  const reach = (check as { observation: { reach?: { identity?: { runState?: unknown } } } }).observation.reach;
  return reach?.identity?.runState !== undefined;
}

/**
 * The unknown-runtime paragraph a fixture's evidence calls for: none when a state is known, the
 * "no defined run state observed" wording beside a run-state record, the legacy wording without one.
 * Both wordings are read from the message catalogue, never from the view's own selection.
 */
async function expectUnknownRuntime(scope: Locator, m: Messages, check: { runtime: string }) {
  const unknown = check.runtime === "unknown";
  const recorded = hasRunStateRecord(check);
  await expect(scope.locator("p", { hasText: m["postDownloadCheck.runtimeUnknown.noDefinedState"] }))
    .toHaveCount(unknown && recorded ? 1 : 0);
  await expect(scope.locator("p", { hasText: m["postDownloadCheck.runtimeUnknown"] }))
    .toHaveCount(unknown && !recorded ? 1 : 0);
}

for (const { fixture, restart, lines } of CHECKS) {
  const check = checkFixtures[fixture];
  const status = {
    ...finished,
    status: { state: "finished", written: "yes", restart,
      restartNote: restart === "unconfirmed" ? "no T_ACK for A_Restart" : null, postDownloadCheck: check },
    events: [...finished.events, { kind: "postDownloadCheckStarted" }],
    nextSince: finished.events.length + 1,
  };
  // The report field order the Rust serializer writes: the check after the outcome.
  const { parameterValues, ...head } = report.report;
  const checked = { ...report, report: { ...head, outcome: { ...head.outcome, restart }, postDownloadCheck: check, parameterValues } };
  // An intercepted history page carrying the same check: a prepared API fixture, not a persisted
  // row. It exercises the real history UI consumer only; that the check survives a SQLite reopen
  // is established by the Rust CLI and server reopen tests, not by this browser lane.
  const history = {
    format: 2, coverage: "partial", durability: "persistent", hasMore: false, nextCursor: 1,
    untracked: ["deviceIdentify", "groupWrite", "serialAddress", "deviceDownload", "addressProgramming", "busMonitor", "lineScan"],
    entries: [{
      sequence: 1, serverIncarnation: "fixture", interrupted: false, id: 1, kind: "deviceDownload", address: "1.1.67",
      state: "finished", startedAt: "2026-10-26T09:00:00.000Z", finishedAt: "2026-10-26T09:00:05.000Z",
      writeEvidence: { backupRecorded: true, sendPossible: true },
      downloadEvidence: { sessionId: 7, written: "yes", restart, cleanup: "returnedOk", postDownloadCheck: check },
    }],
  };
  for (const language of ["en", "de"] as const) {
    for (const width of [1440, 400]) {
      test(`${language} ${width}: the ${fixture} check stands apart from the transfer and travels in the report`, async ({ page }) => {
        const m = language === "en" ? en : de;
        expect(checked.report.outcome.restart, "status and saved report describe the same restart").toBe(status.status.restart);
        await page.setViewportSize({ width, height: 900 });
        const state = await open(page, language, { status, report: checked, history });
        const panel = page.locator(".device-download-panel");
        await panel.locator("select[data-download-device]").selectOption({ label: "1.1.67 — Fictional actuator" });
        await panel.locator("input[data-download-gateway]").fill("192.0.2.10:3671");
        await panel.getByRole("button", { name: m["deviceDownload.preparePlan"], exact: true }).click();
        const shown = panel.getByRole("region", { name: m["deviceDownload.planTitle"] });
        await shown.locator("input[data-untested-acknowledgement]").fill(plan.untestedAcknowledgement);
        await shown.getByRole("button", { name: m["deviceDownload.start"].replace("{address}", "1.1.67"), exact: true }).click();
        await page.locator(".programming-consent-dialog").getByRole("button", { name: m["programmingConsent.confirm"], exact: true }).click();

        // The transfer and its restart, each its own line outside the check.
        const progress = panel.locator(".device-download-progress");
        const section = progress.locator(".post-download-check");
        await expect(section).toHaveAttribute("data-check", "attempted");
        await expect(section).toHaveAttribute("data-reach", "reached");
        const writtenLine = m["deviceDownload.written.yes"].replace("{count}", "32");
        const restartLine = restart === "acknowledged" ? m["deviceDownload.restartAcknowledged"] : m["deviceDownload.restartUnconfirmed"];
        await expect(progress.locator(":scope > p", { hasText: writtenLine })).toHaveCount(1);
        await expect(progress.locator(":scope > p", { hasText: restartLine })).toHaveCount(1);
        await expect(section).not.toContainText(writtenLine);
        await expect(section).not.toContainText(restartLine);

        // The check: its own facts; runtime unknown unless matching run-state evidence (worded by whether a run-state record exists), no proof of the exemplar.
        await expect(section.getByRole("heading", { name: m["postDownloadCheck.title"], exact: true })).toBeVisible();
        for (const key of lines) await expect(section).toContainText(m[key]);
        await expect(section).not.toContainText(m["postDownloadCheck.notChecked"]);
        await expectUnknownRuntime(section, m, check);
        const readings = readingsOf(check);
        await expect(section.locator("p", { hasText: m["postDownloadCheck.exemplar"] })).toHaveCount(readings.length > 0 ? 1 : 0);
        const rows = section.locator("tbody tr");
        await expect(rows).toHaveCount(readings.length);
        for (const [index, reading] of readings.entries()) {
          const row = rows.nth(index);
          await expect(row).toHaveAttribute("data-machine", reading.machine);
          const expected = reading.expected === null ? m["postDownloadCheck.expected.unresolved"]
            : m[`postDownloadCheck.state.${reading.expected}` as keyof Messages];
          const observed = reading.observed.result === "observed"
            ? m[`postDownloadCheck.state.${reading.observed.loadState}` as keyof Messages]
            : m[`postDownloadCheck.observed.${reading.observed.result}` as keyof Messages];
          await expect(row.locator("td")).toHaveText([m[`postDownloadCheck.machine.${reading.machine}` as keyof Messages], expected, observed]);
        }
        await expect(section.locator("button, a, input")).toHaveCount(0);

        // The manual checklist stays; the saved report carries the very check the server gave.
        const verify = panel.getByRole("region", { name: m["deviceDownload.verify.title"] });
        await expect(verify.locator(".device-download-verify-steps li")).toHaveCount(5);
        await verify.getByRole("button", { name: m["deviceDownload.verify.create"], exact: true }).click();
        const expectedChecks = [
          ...(restart === "unconfirmed" ? ["restart"] : []),
          "groupAddresses", "status", "parameters", "multiOctet",
        ];
        const checklist = verify.locator(".device-download-verify-checklist");
        await expect(checklist.locator("[data-check]")).toHaveCount(expectedChecks.length);
        expect(await checklist.locator("[data-check]").evaluateAll((nodes) =>
          nodes.map((node) => node.getAttribute("data-check")))).toEqual(expectedChecks);
        for (const id of expectedChecks) {
          await expect(checklist.locator(`[data-check="${id}"]`))
            .toHaveText(m[`deviceDownload.verify.check.${id}` as keyof Messages]);
        }
        const download = page.waitForEvent("download");
        await verify.getByRole("button", { name: m["deviceDownload.verify.save"], exact: true }).click();
        const text = await readFile(await (await download).path(), "utf8");
        expect(text).toBe(`${JSON.stringify(checked.report, null, 2)}\n`);
        expect(JSON.stringify(JSON.parse(text).postDownloadCheck)).toBe(JSON.stringify(check));

        await section.scrollIntoViewIfNeeded();
        if (process.env.KNX_VERIFY_CAPTURE_DIR) {
          await section.screenshot({ path: join(process.env.KNX_VERIFY_CAPTURE_DIR, `${language}-${width}-check-${fixture}.png`) });
        }
        const geometry = await section.evaluate((element) => {
          const box = element.getBoundingClientRect();
          const children = [...element.querySelectorAll("p, table, h4")].map((node) => node.getBoundingClientRect());
          return {
            left: box.left, right: box.right,
            inside: children.every((child) => child.left >= box.left - 1 && child.right <= box.right + 1),
            overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
          };
        });
        expect(geometry.left).toBeGreaterThanOrEqual(0);
        expect(geometry.right).toBeLessThanOrEqual(width);
        expect(geometry.inside, "the check stays inside its section").toBe(true);
        expect(geometry.overflow, "no horizontal scrolling").toBeLessThanOrEqual(1);
        // The history parent shows the same facts from the intercepted history fixture's record.
        await page.getByRole("button", { name: m["activityHistory.title"], exact: true }).click();
        await expect(panel).toHaveCount(0);
        const stored = page.locator(".activity-history-entry .post-download-check");
        await expect(stored).toHaveAttribute("data-check", "attempted");
        for (const key of lines) await expect(stored).toContainText(m[key]);
        await expectUnknownRuntime(stored, m, check);
        await expect(stored.locator("tbody tr")).toHaveCount(readings.length);
        for (const [index, reading] of readings.entries()) {
          await expect(stored.locator("tbody tr").nth(index)).toHaveAttribute("data-machine", reading.machine);
        }
        expect(state.starts, "one start, no second download").toBe(1);
        expect(state.writes, "only the start was sent").toEqual([]);
      });
    }
  }
}

// ---- DIAG-02: a failed run keeps its cause in the panel, the history and the report ----

const LOST = {
  category: "connectionLost", code: "connectionLost", phase: "inStep",
  step: { number: 3, of: 3, kind: "writeMemory", summary: "write memory 4400h..441Fh, 32 octets" },
  restart: "notInPlan", readBackOctets: 16,
};
const failedStatus = {
  ...finished,
  status: { state: "failed", written: "partially", stoppedInStep: 3, error: "step 2: the connection to 1.1.67 was lost during A_Memory_Write", hint: null, failure: LOST },
  events: finished.events.slice(0, 16), nextSince: 16,
};
const failedReport = (() => {
  const { parameterValues, ...head } = report.report;
  return { ...report, report: { ...head, outcome: { completed: false, written: "partially", restart: null, stoppedInStep: 3 },
    postDownloadCheck: { observation: { status: "notChecked" }, runtime: "unknown" }, failure: LOST, parameterValues } };
})();
const failedHistory = {
  format: 2, coverage: "partial", durability: "persistent", hasMore: false, nextCursor: 1,
  untracked: ["deviceIdentify", "groupWrite", "serialAddress", "deviceDownload", "addressProgramming", "busMonitor", "lineScan"],
  entries: [{
    sequence: 1, serverIncarnation: "fixture", interrupted: false, id: 1, kind: "deviceDownload", address: "1.1.67",
    state: "failed", startedAt: "2026-10-26T09:00:00.000Z", finishedAt: "2026-10-26T09:00:05.000Z",
    writeEvidence: { backupRecorded: true, sendPossible: true },
    downloadEvidence: { sessionId: 7, written: "partially", restart: null, cleanup: "returnedOk", failure: LOST },
  }],
};

for (const language of ["en", "de"] as const) {
  test(`${language}: a failed download stays failed, with its cause, after the panel is closed and reopened`, async ({ page }) => {
    const m = language === "en" ? en : de;
    const fill = (key: keyof typeof en, values: Record<string, string | number>) =>
      Object.entries(values).reduce((text, [name, value]) => text.replace(`{${name}}`, String(value)), m[key]);
    const state = await open(page, language, { status: failedStatus, report: failedReport, history: failedHistory });
    const panel = page.locator(".device-download-panel");
    await panel.locator("select[data-download-device]").selectOption({ label: "1.1.67 — Fictional actuator" });
    await panel.locator("input[data-download-gateway]").fill("192.0.2.10:3671");
    await panel.getByRole("button", { name: m["deviceDownload.preparePlan"], exact: true }).click();
    const shown = panel.getByRole("region", { name: m["deviceDownload.planTitle"] });
    await shown.locator("input[data-untested-acknowledgement]").fill(plan.untestedAcknowledgement);
    await shown.getByRole("button", { name: m["deviceDownload.start"].replace("{address}", "1.1.67"), exact: true }).click();
    await page.locator(".programming-consent-dialog").getByRole("button", { name: m["programmingConsent.confirm"], exact: true }).click();

    const expectFailure = async () => {
      const alert = panel.getByRole("alert").filter({ has: page.getByRole("heading", { name: m["downloadFailure.title.failed"], exact: true }) });
      await expect(alert).toHaveCount(1);
      await expect(alert).toHaveAttribute("data-failure", "connectionLost");
      await expect(alert).toContainText(m["downloadFailure.category.connectionLost"]);
      await expect(alert).toContainText(fill("downloadFailure.phase.inStep", { number: 3, of: 3, summary: LOST.step.summary }));
      await expect(alert).toContainText(m["downloadFailure.restart.notInPlan"]);
      await expect(alert).toContainText(fill("downloadFailure.readBack", { count: 16 }));
      await expect(panel.locator("[data-download-state=failed]")).toHaveText(fill("deviceDownload.failed", { step: 3 }));
    };
    await expectFailure();

    // Closing the panel: the intercepted history fixture's entry shows the same cause.
    await page.getByRole("button", { name: m["activityHistory.title"], exact: true }).click();
    await expect(panel).toHaveCount(0);
    const history = page.locator(".activity-history-entry .download-failure");
    await expect(history).toHaveAttribute("data-failure", "connectionLost");
    await expect(history).not.toHaveAttribute("role", /.*/);
    await expect(history).toContainText(m["downloadFailure.partialCaveat"]);
    await expect(history).toContainText(m["downloadFailure.category.connectionLost"]);

    // Reopening reads the server-held run again, without a new download.
    await page.getByRole("button", { name: m["deviceDownload.title"], exact: true }).click();
    await expectFailure();
    const verify = panel.getByRole("region", { name: m["deviceDownload.verify.title"] });
    await verify.getByRole("button", { name: m["deviceDownload.verify.create"], exact: true }).click();
    const download = page.waitForEvent("download");
    await verify.getByRole("button", { name: m["deviceDownload.verify.save"], exact: true }).click();
    const saved = JSON.parse(await readFile(await (await download).path(), "utf8"));
    expect(saved.failure).toEqual(LOST);
    expect(JSON.stringify(saved)).not.toContain("A_Memory_Write");
    expect(state.starts, "one start, no second download").toBe(1);
    expect(state.writes, "only the start was sent").toEqual([]);
  });
}
