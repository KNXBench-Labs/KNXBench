/** Verify run receipts in the real diagnostics parent using shared EN/DE fixtures. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
vi.mock("./BusMonitorPanel", () => ({ default: () => <p>Monitor stub</p> }));
vi.mock("./LineScanPanel", () => ({ default: () => <p>Scan stub</p> }));
vi.mock("./DeviceInspectionPanel", () => ({ default: () => <p>Checks stub</p> }));
vi.mock("./DeviceDownloadPanel", () => ({ default: () => <p>Download stub</p> }));
vi.mock("./AddressProgrammingPanel", () => ({ default: () => <p>Address stub</p> }));
vi.mock("./ServiceControlPanel", () => ({ default: () => <p>Service stub</p> }));
import BusDiagnosticsPanel from "./BusDiagnosticsPanel";
import fixtures from "./downloadReceipt.fixtures.json";
import { messages as en } from "./messages/en";
import { messages as de } from "./messages/de";
import { resetSettingsForTests, setSetting } from "./settingsStore";
import { resetUiLanguageForTests, UI_LANGUAGE_STORAGE_KEY } from "./uiLanguage";
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

type FixtureName = keyof typeof fixtures;
let host: HTMLDivElement; let root: Root;

/** A strict page of the named shared fixture rows, in start order. */
function pageOf(...names: FixtureName[]) {
  const entries: Record<string, unknown>[] = names.map((name, index) => ({
    sequence: index + 1, serverIncarnation: "synthetic-server", interrupted: false,
    ...structuredClone(fixtures[name]), id: index + 1,
  }));
  return { format: 2, coverage: "partial", durability: "persistent", hasMore: false, nextCursor: names.length,
    untracked: ["deviceIdentify", "groupWrite", "serialAddress", "deviceDownload", "addressProgramming", "busMonitor", "lineScan"], entries };
}
function response(body: unknown) {
  return { ok: true, status: 200, statusText: "", headers: new Headers(), json: async () => body };
}
async function flush() { await act(async () => { for (let i = 0; i < 8; i++) await Promise.resolve(); }); }
async function show(language: "en" | "de", body: unknown) {
  setSetting(UI_LANGUAGE_STORAGE_KEY, language);
  const fetcher = vi.fn().mockResolvedValue(response(body)); vi.stubGlobal("fetch", fetcher);
  host = document.createElement("div"); document.body.appendChild(host); root = createRoot(host);
  await act(async () => root.render(<BusDiagnosticsPanel project={null} onTreeUpdate={vi.fn()} />));
  const title = (language === "de" ? de : en)["activityHistory.title"];
  const button = [...host.querySelectorAll("button")].find((node) => node.textContent === title);
  expect(button, "reachable history button").toBeDefined();
  await act(async () => button!.click()); await flush();
  return fetcher;
}
afterEach(async () => {
  if (root) await act(async () => root.unmount());
  host?.remove(); vi.unstubAllGlobals(); resetSettingsForTests(); resetUiLanguageForTests();
});
const receipts = () => [...host.querySelectorAll<HTMLElement>(".download-receipt")];
const settlement = (node: HTMLElement) => node.querySelector<HTMLElement>(".download-receipt-settlement");
function cells(node: HTMLElement, part: string) {
  return [...node.querySelectorAll(`tr[data-part="${part}"] td`)].map((cell) => cell.textContent);
}

for (const [language, m] of [["en", en], ["de", de]] as const) {
  describe(`receipt in ${language}`, () => {
    const yes = m["downloadReceipt.yes"]; const no = m["downloadReceipt.no"];

    it("reads a record without a receipt as not recorded, and a run that wrote nothing as not attempted", async () => {
      const fetcher = await show(language, pageOf("legacy", "notAttempted"));
      const [legacy, none] = receipts();
      expect(legacy.dataset.preparation).toBe("absent");
      expect(legacy.dataset.settlement).toBe("absent");
      expect(legacy.textContent).toContain(m["downloadReceipt.absent"]);
      expect(legacy.querySelector(".download-receipt-preparation, .download-receipt-settlement")).toBeNull();
      expect(none.dataset.settlement).toBe("notAttempted");
      expect(none.textContent).toContain(m["downloadReceipt.project.notAttempted"]);
      expect(none.textContent).not.toContain(m["downloadReceipt.absent"]);
      expect(none.querySelector("table")).toBeNull();
      expect(fetcher.mock.calls.map(([url]) => url)).toEqual(["/api/bus/history?after=0&limit=50"]);
      expect(host.textContent).toContain(m["downloadReceipt.boundary"]);
    });

    it("keeps a concrete scope when the plan summary is unavailable, and names every skipped project", async () => {
      await show(language, pageOf("unavailablePreparation", "noProject", "otherProject", "deviceMissing", "settlementUnavailable"));
      const nodes = receipts();
      expect(nodes[0].textContent).toContain(m["downloadReceipt.scope.partial-both"]);
      expect(nodes[0].textContent).toContain(m["downloadReceipt.plan.unavailable"]);
      expect(nodes[0].dataset.preparation).toBe("unavailable");
      expect(nodes.map((node) => node.dataset.settlement)).toEqual(["noProject", "noProject", "otherProject", "deviceMissing", "unavailable"]);
      for (const [index, key] of [[1, "noProject"], [2, "otherProject"], [3, "deviceMissing"], [4, "unavailable"]] as const) {
        expect(nodes[index].textContent).toContain(m[`downloadReceipt.project.${key}`]);
        expect(nodes[index].querySelector("table")).toBeNull();
        expect(nodes[index].querySelector("time")).toBeNull();
      }
    });

    it("tells applied-at-a-time from computed-not-applied and file-saved from memory-only", async () => {
      await show(language, pageOf("completePersisted", "completeSessionOnly", "persistenceFailed"));
      const [saved, memory, failed] = receipts().map((node) => settlement(node)!);
      expect(saved.textContent).toContain(m["downloadReceipt.project.persisted"]);
      expect(saved.textContent).toContain(m["downloadReceipt.partsApplied"]);
      expect(saved.querySelector("time")).not.toBeNull();
      expect(memory.textContent).toContain(m["downloadReceipt.project.sessionOnly"]);
      expect(memory.textContent).toContain(m["downloadReceipt.partsApplied"]);
      expect(memory.querySelector("time")).not.toBeNull();
      expect(failed.textContent).toContain(m["downloadReceipt.project.persistenceFailed"]);
      expect(failed.textContent).toContain(m["downloadReceipt.partsComputed"]);
      expect(failed.textContent).not.toContain(m["downloadReceipt.partsApplied"]);
      expect(failed.querySelector("time")).toBeNull();
      expect(failed.querySelector("[role=status]")).not.toBeNull();
    });

    it("shows programmed, pending and touched bits per part and the plan-only source wording", async () => {
      await show(language, pageOf("changedSource", "completeSessionOnly", "requestUnavailable"));
      const [changed, same, unreadable] = receipts().map((node) => settlement(node)!);
      expect(cells(changed, "address")).toEqual([m["downloadReceipt.part.address"], yes, no, yes]);
      expect(cells(changed, "application")).toEqual([m["downloadReceipt.part.application"], no, yes, yes]);
      expect(cells(changed, "groupAddresses")).toEqual([m["downloadReceipt.part.groupAddresses"], no, yes, yes]);
      expect(changed.textContent).toContain(m["downloadReceipt.source.changedOrUnknown"]);
      expect(cells(same, "parameters")).toEqual([m["downloadReceipt.part.parameters"], yes, no, yes]);
      expect(same.textContent).toContain(m["downloadReceipt.source.unchanged"]);
      expect(unreadable.textContent).toContain(m["downloadReceipt.request.unavailable"]);
      expect(cells(unreadable, "address")).toEqual([m["downloadReceipt.part.address"], no, yes, yes]);
    });

    it("shows planned values as planned and a bounded program identity, or says it is not recorded", async () => {
      await show(language, pageOf("completeSessionOnly", "noApplicationVersion", "programUnavailable"));
      const [full, noVersion, noProgram] = receipts();
      expect(full.textContent).toContain(m["downloadReceipt.plannedMask"]);
      expect(full.textContent).toContain(m["downloadReceipt.plannedManufacturer"]);
      expect(full.textContent).toContain(m["downloadReceipt.sha"]);
      expect(full.querySelectorAll("code").length).toBeGreaterThanOrEqual(5);
      expect(noVersion.textContent).toContain(m["downloadReceipt.version.none"]);
      expect(noProgram.textContent).toContain(m["downloadReceipt.program.unavailable"]);
    });

    it("shows a prepared run that never settled as not recorded, never as no change", async () => {
      await show(language, pageOf("preparationOnlyUnknown", "preparationOnlyFinished"));
      for (const node of receipts()) {
        expect(node.dataset.settlement).toBe("absent");
        expect(node.textContent).toContain(m["downloadReceipt.settlement.absent"]);
        expect(node.querySelector(".download-receipt-preparation")).not.toBeNull();
      }
    });

    it("keeps the receipt and an interrupted post-download check side by side as separate facts", async () => {
      await show(language, pageOf("appliedWithInterruptedCheck"));
      const entry = host.querySelector<HTMLElement>(".activity-history-entry")!;
      expect(entry.querySelector<HTMLElement>(".download-receipt")!.dataset.settlement).toBe("persisted");
      expect(entry.querySelector<HTMLElement>(".post-download-check")!.dataset.check).toBe("interrupted");
      expect(entry.querySelector(".download-receipt .post-download-check")).toBeNull();
      expect(entry.textContent).toContain(m["postDownloadCheck.runtimeUnknown"]);
    });

    it("refuses the whole page for a contradictory receipt instead of showing a prefix", async () => {
      const bad = pageOf("completeSessionOnly", "completeSessionOnly");
      (bad.entries[1].downloadEvidence as Record<string, unknown>).settlement = null;
      await show(language, bad);
      expect(host.querySelector(".activity-history-panel [role=alert]")?.textContent).toBe(m["activityHistory.malformed"]);
      expect(host.querySelectorAll(".activity-history-entry")).toHaveLength(0);
      expect(host.querySelector(".download-receipt")).toBeNull();
    });
  });
}
