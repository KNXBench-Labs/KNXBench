/** ADR-0121 in a browser: plan warnings, then a finished download's report and its hand-over. */
import { expect, test, type Page } from "@playwright/test";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { messages as en } from "../src/messages/en";
import { messages as de } from "../src/messages/de";

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
  nextSince: 0, events: [], backupFile: "device-backups/1.1.67_fixture.json",
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

async function open(page: Page, language: "en" | "de") {
  const m = language === "en" ? en : de;
  const state = { started: false, writes: [] as string[], reportReads: 0 };
  await page.route("**/api/**", (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    const method = request.method();
    const json = (body: unknown, status = 200) => route.fulfill({ status, contentType: "application/json", body: JSON.stringify(body) });
    if (path === "/api/auth/status") return json({ required: false, authenticated: true });
    if (path === "/api/settings" && method === "GET") return json({ schemaVersion: 1, status: "ok", settings: { uiLanguage: language } });
    if (path === "/api/project" && method === "GET") return json(tree);
    if (path === "/api/project/new" && method === "POST") return json(tree);
    if (path === "/api/bus/discover") return json({ interfaces: [] });
    if (path === "/api/bus/activity" && method === "GET") return json({ serverIncarnation: "fixture", coverage: "partial", historyState: "disabled", sessions: [], oneShot: [], oneShotDropped: 0, busyLocks: [], untracked: [] });
    if (path === "/api/device-readiness" && method === "GET") return json({ devices: [], counts: {} });
    if (path === "/api/device-download/plan" && method === "POST") return json(plan);
    if (path === "/api/device-download/start" && method === "POST") {
      const body = request.postDataJSON() as { acceptUntested?: string };
      if (body.acceptUntested !== plan.untestedAcknowledgement) return json({ error: "acknowledgement" }, 400);
      state.started = true;
      return json({ downloadId: 7 });
    }
    if (path === "/api/device-download/status" && method === "GET") {
      return state.started ? json(finished) : json({ error: "no download to a device yet" }, 404);
    }
    if (path === "/api/device-download/report" && method === "GET") {
      state.reportReads += 1;
      return json(report);
    }
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

      const verify = panel.getByRole("region", { name: m["deviceDownload.verify.title"] });
      await expect(verify).toContainText(m["deviceDownload.verify.intro.untested"]);
      await expect(verify.locator(".device-download-verify-steps li")).toHaveCount(5);
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
