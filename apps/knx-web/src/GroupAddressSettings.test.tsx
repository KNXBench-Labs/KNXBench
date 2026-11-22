/** Tests for the group-address settings section and its last-download marks preference. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { messages as de } from "./messages/de";
import { messages as en } from "./messages/en";
import { resetSettingsForTests, settingsStorage } from "./settingsStore";
import { resetUiLanguageForTests } from "./uiLanguage";
import { GA_LAST_DOWNLOAD_KEY, gaLastDownloadMarksEnabled } from "./deviceLastDownloads";
import GroupAddressSettings from "./GroupAddressSettings";

let host: HTMLDivElement | undefined;
beforeEach(() => { resetSettingsForTests(); settingsStorage.setItem("uiLanguage", "en"); resetUiLanguageForTests(); });
afterEach(() => { host?.remove(); host = undefined; });

async function mount() {
  host = document.createElement("div"); document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => { root.render(<GroupAddressSettings />); });
  return root;
}

describe("gaLastDownloadMarksEnabled", () => {
  it("is off without a value and for anything but true/1", () => {
    expect(gaLastDownloadMarksEnabled()).toBe(false);
    for (const value of ["false", "0", "", "yes"]) { settingsStorage.setItem(GA_LAST_DOWNLOAD_KEY, value); expect(gaLastDownloadMarksEnabled(), value).toBe(false); }
    for (const value of ["true", "1"]) { settingsStorage.setItem(GA_LAST_DOWNLOAD_KEY, value); expect(gaLastDownloadMarksEnabled(), value).toBe(true); }
  });
});

describe("GroupAddressSettings", () => {
  it("starts unchecked on a fresh profile and toggles the stored preference", async () => {
    const root = await mount();
    const box = host!.querySelector<HTMLInputElement>("input[type=checkbox]")!;
    expect(box.checked).toBe(false);
    expect(host!.textContent).toContain(en["settings.gaLastDownloadMarks"]);
    await act(async () => box.click());
    expect(box.checked).toBe(true);
    expect(gaLastDownloadMarksEnabled()).toBe(true);
    await act(async () => box.click());
    expect(gaLastDownloadMarksEnabled()).toBe(false);
    await act(async () => root.unmount());
  });

  it("is translated to German", async () => {
    settingsStorage.setItem("uiLanguage", "de"); resetUiLanguageForTests();
    const root = await mount();
    expect(host!.textContent).toContain(de["settings.gaLastDownloadMarks"]);
    expect(host!.querySelector("h2")!.textContent).toBe(de["settings.section.groupAddresses"]);
    await act(async () => root.unmount());
  });
});
