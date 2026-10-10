/** Tests readable localized archive-size refusals through the real toast hook and view. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import ToastStack from "./Toast";
import { useToasts } from "./toast";
import { resetUiLanguageForTests, saveUiLanguage } from "./uiLanguage";
import { resetSettingsForTests, settingsStorage } from "./settingsStore";

const MIB = 1024 * 1024;
const refusal = (total: number, limit: number) =>
  `the archive declares ${total} uncompressed bytes, more than the ${limit}-byte limit`;
let host: HTMLDivElement;
let root: Root;
let queue: ReturnType<typeof useToasts>;
function Harness() {
  queue = useToasts();
  return <ToastStack toasts={queue.toasts} onDismiss={queue.dismiss} />;
}

beforeEach(() => {
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
  vi.useFakeTimers();
  vi.spyOn(Math, "random").mockReturnValue(0);
  host = document.createElement("div");
  document.body.appendChild(host);
  root = createRoot(host);
});
afterEach(async () => {
  await act(async () => root.unmount());
  host.remove();
  vi.clearAllTimers();
  vi.useRealTimers();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  resetSettingsForTests();
  resetUiLanguageForTests();
});
async function show(message: string, language: string, options?: { serverText?: boolean }) {
  saveUiLanguage(settingsStorage, language);
  resetUiLanguageForTests();
  await act(async () => root.render(<Harness />));
  await act(async () => queue.pushError(message, options));
  return host.querySelector('[role="alert"]')!;
}

describe("readable archive-size refusal", () => {
  it("shows German size and the actual old-server limit without an English disclosure or joke", async () => {
    const alert = await show(refusal(660 * MIB, 512 * MIB), "de");
    expect(alert.querySelector(".toast-title")!.textContent).toBe(
      "Die Datei ist entpackt mit 660 MiB zu groß. Das Limit liegt bei 512 MiB.",
    );
    expect(alert.querySelector(".toast-hint")).toBeNull();
    expect(queue.toasts[0].serverText).toBe(false);
  });
  it("uses the current server's limit rather than hard-coding 512 or 1024", async () => {
    const alert = await show(refusal(1280 * MIB, 1024 * MIB), "de");
    expect(alert.querySelector(".toast-title")!.textContent).toBe(
      "Die Datei ist entpackt mit 1280 MiB zu groß. Das Limit liegt bei 1024 MiB.",
    );
  });
  it("uses English when English is selected", async () => {
    const alert = await show(refusal(1280 * MIB, 1024 * MIB), "en");
    expect(alert.querySelector(".toast-title")!.textContent).toBe(
      "The file is too large when unpacked: 1280 MiB. The limit is 1024 MiB.",
    );
    expect(alert.querySelector(".toast-hint")).toBeNull();
  });
  it("falls back to English number formatting for a private-use language tag", async () => {
    const alert = await show(refusal(1024 * MIB + 1, 1024 * MIB), "x-knx");
    expect(alert.querySelector(".toast-title")!.textContent).toBe(
      "The file is too large when unpacked: 1024.01 MiB. The limit is 1024 MiB.",
    );
  });
  it("rounds an oversized total up so a one-byte excess is not displayed as equal to the limit", async () => {
    const alert = await show(refusal(1024 * MIB + 1, 1024 * MIB), "de");
    expect(alert.querySelector(".toast-title")!.textContent).toBe(
      "Die Datei ist entpackt mit 1024,01 MiB zu groß. Das Limit liegt bei 1024 MiB.",
    );
  });
  it.each([
    "unrelated server error",
    "prefix: the archive declares 660602880 uncompressed bytes, more than the 536870912-byte limit",
    refusal(512 * MIB, 512 * MIB),
    refusal(1, 0),
    refusal(Number.MAX_SAFE_INTEGER + 1, 512 * MIB),
    "the archive declares NaN uncompressed bytes, more than the 536870912-byte limit",
  ])("preserves unknown or invalid server messages and their disclosure: %s", async (message) => {
    const alert = await show(message, "de");
    expect(alert.querySelector(".toast-title")!.textContent).toContain(message);
    expect(alert.querySelector(".toast-hint")).not.toBeNull();
    expect(queue.toasts[0].serverText).toBe(true);
  });
  it("does not reinterpret a message explicitly marked as client text", async () => {
    const message = refusal(1280 * MIB, 1024 * MIB);
    const alert = await show(message, "de", { serverText: false });
    expect(alert.querySelector(".toast-title")!.textContent).toContain(message);
    expect(alert.querySelector(".toast-hint")).toBeNull();
  });
});
