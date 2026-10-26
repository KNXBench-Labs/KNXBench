/** DIAG-02: a failed download's cause in the real history parent, in EN and DE. */
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
import cases from "./downloadFailure.cases.json";
import { messages as en } from "./messages/en";
import { messages as de } from "./messages/de";
import { resetSettingsForTests, setSetting } from "./settingsStore";
import { resetUiLanguageForTests, UI_LANGUAGE_STORAGE_KEY } from "./uiLanguage";
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let host: HTMLDivElement; let root: Root;

function pageOf(...documents: Record<string, unknown>[]) {
  return {
    format: 2, coverage: "partial", durability: "persistent", hasMore: false, nextCursor: documents.length, untracked: [],
    entries: documents.map((document, index) => ({
      sequence: index + 1, serverIncarnation: "synthetic-server", interrupted: false, ...structuredClone(document), id: index + 1,
    })),
  };
}
function legacy() {
  const document = structuredClone(cases.base) as Record<string, unknown> & { downloadEvidence: Record<string, unknown> };
  delete document.downloadEvidence.failure;
  return document;
}
async function show(language: "en" | "de", body: unknown) {
  setSetting(UI_LANGUAGE_STORAGE_KEY, language);
  vi.stubGlobal("fetch", vi.fn().mockResolvedValue({ ok: true, status: 200, statusText: "", headers: new Headers(), json: async () => body }));
  host = document.createElement("div"); document.body.appendChild(host); root = createRoot(host);
  await act(async () => root.render(<BusDiagnosticsPanel project={null} onTreeUpdate={vi.fn()} />));
  const title = (language === "de" ? de : en)["activityHistory.title"];
  const button = [...host.querySelectorAll("button")].find((node) => node.textContent === title)!;
  await act(async () => button.click());
  await act(async () => { for (let i = 0; i < 8; i++) await Promise.resolve(); });
}
afterEach(async () => {
  if (root) await act(async () => root.unmount());
  host?.remove(); vi.unstubAllGlobals(); resetSettingsForTests(); resetUiLanguageForTests();
});

for (const [language, m] of [["en", en], ["de", de]] as const) {
  describe(`history in ${language}`, () => {
    it("shows the recorded cause as an alert and an older row's cause as not recorded", async () => {
      await show(language, pageOf(cases.base, legacy()));
      const [recorded, older] = [...host.querySelectorAll<HTMLElement>(".activity-history-entry .download-failure")];
      expect(recorded.dataset.failure).toBe("timeout");
      expect(recorded.getAttribute("role")).toBe("alert");
      expect(recorded.querySelector("h4")!.textContent).toBe(m["downloadFailure.title.failed"]);
      const text = recorded.textContent!;
      expect(text).toContain(m["downloadFailure.category.timeout"]);
      expect(text).toContain(m["downloadFailure.phase.inStep"].replace("{number}", "8").replace("{of}", "16")
        .replace("{summary}", "write memory 4000h..4003h, 4 octets"));
      expect(text).toContain(m["downloadFailure.restart.notSent"]);
      expect(recorded.querySelector("details"), "a stored row has no raw text").toBeNull();
      expect(older.dataset.failure).toBe("notRecorded");
      expect(older.textContent).toContain(m["downloadFailure.notRecorded"]);
      expect(older.textContent).not.toContain(m["downloadFailure.category.timeout"]);
    });

    it("refuses the whole page when a stored cause is malformed", async () => {
      const broken = structuredClone(cases.base);
      broken.downloadEvidence.failure.category = "internal";
      await show(language, pageOf(broken));
      expect(host.querySelector(".download-failure")).toBeNull();
      expect(host.querySelector("[role=alert]")!.textContent).toBe(m["activityHistory.malformed"]);
    });
  });
}
