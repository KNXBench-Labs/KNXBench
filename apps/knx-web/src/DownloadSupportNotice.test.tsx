/** Verifies EN/DE refusal notices, the report shortcut, fixed links and local-copy boundaries. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
const locale = vi.hoisted(() => ({ value: "en" }));
vi.mock("./uiLanguage", () => ({ useUiLanguage: () => [locale.value] }));
vi.mock("./i18n", async (importOriginal) => {
  const original = await importOriginal<typeof import("./i18n")>();
  return { ...original, useTranslate: () => (key: Parameters<typeof original.translateFor>[1]) => original.translateFor(locale.value, key) };
});
import DownloadSupportNotice, { supportTargets } from "./DownloadSupportNotice";
import { guidanceForError, guidanceForMark, type SupportGuidance } from "./downloadSupportGuidance";
import { translateFor } from "./i18n";
import { ReportProblemProvider } from "./ReportProblemEntry";
import type { ReportProblemRequest } from "./reportProblem";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
let root: Root | undefined;
let host: HTMLDivElement | undefined;
const REPORT: ReportProblemRequest = { problemType: "deviceProgramming", deviceId: 42, operation: null };
const opened = vi.fn();
function notice(guidance: SupportGuidance) {
  return <ReportProblemProvider request={null} onOpen={opened} onClose={() => {}} tree={null} notifications={[]}>
    <DownloadSupportNotice guidance={guidance} report={REPORT} />
  </ReportProblemProvider>;
}
async function render(guidance: SupportGuidance) {
  opened.mockReset();
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
  await act(async () => root!.render(notice(guidance)));
}
afterEach(async () => { if (root) await act(async () => root!.unmount()); host?.remove(); vi.unstubAllGlobals(); });

for (const language of ["en", "de"]) {
  it(`${language}: customer-adjustable refusal has its own explanation and only a support action`, async () => {
    locale.value = language;
    const detail = "customer-adjustable-parameters: 1 uninterpreted declaration (root parameters: 1); no regular download scope is offered";
    const error = Object.assign(new Error(detail), { status: 422, body: { kind: "downloadPreparationRefused", category: "customer-adjustable-parameters" } });
    await render(guidanceForError(error));
    expect(host!.querySelector("[role=status]")!.textContent).toBe(translateFor(language, "deviceDownload.readiness.reason.customerAdjustableParameters"));
    expect(host!.querySelector("[data-next-action=requestSupport]")!.textContent).toBe(translateFor(language, "deviceDownload.help.action.requestSupport"));
    expect(host!.querySelectorAll("[data-next-action]")).toHaveLength(1);
    expect(host!.querySelector("textarea")!.value).toBe(`downloadPreparationRefused\ncustomer-adjustable-parameters\n${detail}`);
    for (const privateValue of ["privateSentinel", "P-991", "ModuleDef("]) expect(host!.innerHTML).not.toContain(privateValue);
    expect([...host!.querySelectorAll<HTMLAnchorElement>("a")].map((link) => link.getAttribute("href"))).toEqual(Object.values(supportTargets(language)));
  });
  it(`${language}: recognized invalid-address refusal names the concrete reason`, async () => {
    locale.value = language;
    const error = Object.assign(new Error("PRIVATE invalid target"), { status: 400, body: { kind: "downloadInvalidAddress" } });
    await render(guidanceForError(error));
    expect(host!.querySelector("[role=status]")!.textContent).toBe(language === "de"
      ? "Die Individualadresse des Zielgeräts ist ungültig."
      : "The target device's individual address is invalid.");
    expect(host!.querySelector("[data-next-action=fixConfiguration]")).not.toBeNull();
  });

  for (const outcome of ["copied", "failed"] as const) {
    it(`${language}: late ${outcome} result cannot label or select another target's details`, async () => {
      locale.value = language;
      let resolve!: () => void; let reject!: (error: Error) => void;
      const pending = new Promise<void>((ok, fail) => { resolve = ok; reject = fail; });
      Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText: vi.fn(() => pending) } });
      await render(guidanceForMark({ kind: "unsupported", category: "modules", detail: "FIRST" }));
      await act(async () => host!.querySelector<HTMLButtonElement>("[data-copy-technical]")!.click());
      await act(async () => root!.render(notice(guidanceForMark({ kind: "unsupported", category: "modules", detail: "SECOND" }))));
      const field = host!.querySelector<HTMLTextAreaElement>("textarea")!;
      field.setSelectionRange(1, 2);
      await act(async () => outcome === "copied" ? resolve() : reject(new Error("clipboard blocked")));
      expect(host!.textContent).not.toContain(translateFor(language, `deviceDownload.help.${outcome}`));
      expect(field.selectionStart).toBe(1); expect(field.selectionEnd).toBe(2);
    });
  }

  it(`${language}: instructions stay fixed and the report shortcut opens the wizard, despite private details`, async () => {
    locale.value = language;
    const privateText = "CUSTOMER_SECRET 1.1.42 /home/private token=secret <img src='https://evil.test/pixel'>";
    await render(guidanceForMark({ kind: "unsupported", category: "modules", detail: privateText }));
    expect(host!.querySelector("[data-next-action=requestSupport]")!.textContent).toBe(translateFor(language, "deviceDownload.help.action.requestSupport"));
    const links = [...host!.querySelectorAll<HTMLAnchorElement>("a")];
    expect(links.map((link) => link.getAttribute("href"))).toEqual(Object.values(supportTargets(language)));
    // DIAG-03: the mail and issue links became one shortcut into the report wizard.
    expect(host!.innerHTML).not.toContain("mailto:");
    const shortcut = host!.querySelector<HTMLButtonElement>("[data-report-problem-open=support-notice]")!;
    expect(shortcut.textContent).toBe(translateFor(language, "reportProblem.button"));
    expect(opened).not.toHaveBeenCalled();
    await act(async () => shortcut.click());
    expect(opened).toHaveBeenCalledExactlyOnceWith(REPORT);
    expect(document.querySelector(".report-problem")).toBeNull();
    for (const link of links) {
      expect(link.href).not.toContain("CUSTOMER_SECRET");
      expect(link.href).not.toContain("1.1.42");
      expect(link.href).not.toContain("private");
      expect(link.href).not.toContain("secret");
    }
    expect(links[0].rel).toBe("noopener noreferrer");
    expect(host!.querySelector("img")).toBeNull();
    expect(host!.querySelector<HTMLTextAreaElement>("textarea")!.value).toContain(privateText);
    expect(host!.textContent).toContain(translateFor(language, "deviceDownload.help.privacy"));
  });

  it(`${language}: technical copy requires a click; clipboard failure offers manual selection`, async () => {
    locale.value = language;
    const copy = vi.fn().mockRejectedValue(new Error("clipboard blocked"));
    Object.defineProperty(navigator, "clipboard", { configurable: true, value: { writeText: copy } });
    await render(guidanceForMark({ kind: "unsupported", category: "byte-order", detail: "PRIVATE" }));
    expect(copy).not.toHaveBeenCalled();
    await act(async () => host!.querySelector<HTMLButtonElement>("[data-copy-technical]")!.click());
    expect(copy).toHaveBeenCalledExactlyOnceWith("byte-order\nPRIVATE");
    expect(host!.querySelector('[role="alert"]')!.textContent).toBe(translateFor(language, "deviceDownload.help.failed"));
    const field = host!.querySelector<HTMLTextAreaElement>("textarea")!;
    expect(field.selectionStart).toBe(0); expect(field.selectionEnd).toBe(field.value.length);
  });

  it(`${language}: null and future categories remain understandable without a plan`, async () => {
    locale.value = language;
    await render(guidanceForMark({ kind: "unsupported", category: null }));
    expect(host!.querySelector("[data-next-action=reportUnknown]")!.textContent).toBe(translateFor(language, "deviceDownload.help.action.reportUnknown"));
    expect(host!.querySelector("textarea")!.textContent).toBe(translateFor(language, "deviceDownload.help.noDetail"));
    expect(host!.querySelector<HTMLButtonElement>("[data-copy-technical]")!.disabled).toBe(true);
  });
}
