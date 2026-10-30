/** Verifies EN/DE refusal notices, deliberate links and private local-copy boundaries. */
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

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
let root: Root | undefined;
let host: HTMLDivElement | undefined;
async function render(guidance: SupportGuidance) {
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
  await act(async () => root!.render(<DownloadSupportNotice guidance={guidance} />));
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
      await act(async () => root!.render(<DownloadSupportNotice guidance={guidanceForMark({ kind: "unsupported", category: "modules", detail: "SECOND" })} />));
      const field = host!.querySelector<HTMLTextAreaElement>("textarea")!;
      field.setSelectionRange(1, 2);
      await act(async () => outcome === "copied" ? resolve() : reject(new Error("clipboard blocked")));
      expect(host!.textContent).not.toContain(translateFor(language, `deviceDownload.help.${outcome}`));
      expect(field.selectionStart).toBe(1); expect(field.selectionEnd).toBe(2);
    });
  }

  it(`${language}: instructions, unsent issue and email stay fixed despite private or malicious details`, async () => {
    locale.value = language;
    const privateText = "CUSTOMER_SECRET 1.1.42 /home/private token=secret <img src='https://evil.test/pixel'>";
    await render(guidanceForMark({ kind: "unsupported", category: "modules", detail: privateText }));
    expect(host!.querySelector("[data-next-action=requestSupport]")!.textContent).toBe(translateFor(language, "deviceDownload.help.action.requestSupport"));
    const links = [...host!.querySelectorAll<HTMLAnchorElement>("a")];
    expect(links.map((link) => link.getAttribute("href"))).toEqual(Object.values(supportTargets(language)));
    expect(new URL(links[1].href).searchParams.get("template")).toBe("analysis.yml");
    expect(links[2].href).toBe("mailto:issue@knxbench.com?subject=KNXBench%20device%20programming%20support%20gap");
    for (const link of links) {
      expect(link.href).not.toContain("CUSTOMER_SECRET");
      expect(link.href).not.toContain("1.1.42");
      expect(link.href).not.toContain("private");
      expect(link.href).not.toContain("secret");
    }
    expect(links[0].rel).toBe("noopener noreferrer");
    expect(links[1].rel).toBe("noopener noreferrer");
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
