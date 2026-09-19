/** Tests for the debug-report dialog: what it asks for, what it sends, and what it never does. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { resetUiLanguageForTests } from "./uiLanguage";

const apiMock = vi.hoisted(() => ({
  createDebugReport: vi.fn(),
}));

const pickerMock = vi.hoisted(() => ({
  pickSavePath: vi.fn(),
  isTauri: vi.fn(() => false),
}));

vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}));

vi.mock("./filePicker", () => pickerMock);

import DebugReportButton from "./DebugReportButton";
import { messages as en } from "./messages/en";

let host: HTMLDivElement | undefined;

afterEach(() => {
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
  window.localStorage.clear();
  resetUiLanguageForTests();
});

function report(overrides: Partial<Record<string, unknown>> = {}) {
  return {
    written: true,
    path: "/tmp/knxbench-debug-report.zip",
    files: [
      { name: "report.md", bytes: 100 },
      { name: "environment.json", bytes: 100 },
      { name: "log.json", bytes: 100 },
    ],
    reportMarkdown: "# KNXBench debug report\n\nsomething broke",
    ...overrides,
  };
}

async function render() {
  const onSummary = vi.fn();
  const onError = vi.fn();
  const onClearErrors = vi.fn();
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(
      <DebugReportButton onSummary={onSummary} onError={onError} onClearErrors={onClearErrors} />,
    );
  });
  return { root, onSummary, onError, onClearErrors };
}

function buttons(): HTMLButtonElement[] {
  return Array.from(document.querySelectorAll("button"));
}

function button(label: string): HTMLButtonElement {
  const found = buttons().find((b) => b.textContent?.trim() === label);
  if (!found) {
    throw new Error(
      `no button labelled "${label}"; present: ${buttons()
        .map((b) => b.textContent?.trim())
        .join(" | ")}`,
    );
  }
  return found;
}

async function click(label: string) {
  await act(async () => {
    button(label).click();
  });
}

async function openDialog() {
  const rendered = await render();
  await click(en["debugReport.button"]);
  return rendered;
}

function dialogText(): string {
  return document.querySelector(".debug-report-panel")?.textContent ?? "";
}

async function setDescription(text: string) {
  const area = document.querySelector<HTMLTextAreaElement>("textarea")!;
  const setter = Object.getOwnPropertyDescriptor(
    window.HTMLTextAreaElement.prototype,
    "value",
  )!.set!;
  await act(async () => {
    setter.call(area, text);
    area.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

function checkbox(label: string): HTMLInputElement {
  const option = Array.from(document.querySelectorAll<HTMLLabelElement>(".debug-report-option"))
    .find((el) => el.textContent?.includes(label));
  if (!option) throw new Error(`no option labelled "${label}"`);
  return option.querySelector<HTMLInputElement>('input[type="checkbox"]')!;
}

async function toggle(label: string) {
  const box = checkbox(label);
  await act(async () => {
    box.click();
  });
}

describe("DebugReportButton", () => {
  it("opens nothing until the menu entry is used", async () => {
    await render();
    expect(document.querySelector(".debug-report-panel")).toBeNull();
    await click(en["debugReport.button"]);
    expect(document.querySelector(".debug-report-panel")).not.toBeNull();
  });

  it("lists the two mandatory files and the session log, and nothing else, by default", async () => {
    await openDialog();
    const text = dialogText();
    expect(text).toContain("report.md");
    expect(text).toContain("environment.json");
    expect(text).toContain("log.json");
    expect(text).not.toContain("project-summary.json");
    expect(text).not.toContain("bus-telegrams.json");
  });

  it("adds a file to the list only once its option is switched on", async () => {
    await openDialog();
    await toggle(en["debugReport.include.projectSummary.label"]);
    expect(dialogText()).toContain("project-summary.json");
    expect(dialogText()).not.toContain("bus-telegrams.json");
    await toggle(en["debugReport.include.busTelegrams.label"]);
    expect(dialogText()).toContain("bus-telegrams.json");
  });

  it("warns that the telegrams keep their addresses, and only when they are included", async () => {
    await openDialog();
    expect(document.querySelector(".debug-report-warning")).toBeNull();
    await toggle(en["debugReport.include.busTelegrams.label"]);
    const warning = document.querySelector(".debug-report-warning");
    expect(warning).not.toBeNull();
    expect(warning!.textContent).toBe(en["debugReport.privacyTelegrams"]);
  });

  it("says what is redacted before anything can be written", async () => {
    await openDialog();
    expect(dialogText()).toContain(en["debugReport.privacyRedacted"]);
    // The promise is on screen at the same time as the buttons that act
    // on it — not behind a disclosure the user has to go looking for.
    expect(button(en["debugReport.save"])).not.toBeNull();
    expect(button(en["debugReport.openIssue"])).not.toBeNull();
  });

  it("sends the description and the chosen options to the server with the picked path", async () => {
    pickerMock.pickSavePath.mockResolvedValue("/tmp/out.zip");
    apiMock.createDebugReport.mockResolvedValue(report());
    const { onSummary, onClearErrors } = await openDialog();

    await setDescription("the tunnel closed");
    await toggle(en["debugReport.include.busTelegrams.label"]);
    await click(en["debugReport.save"]);

    expect(apiMock.createDebugReport).toHaveBeenCalledTimes(1);
    const sent = apiMock.createDebugReport.mock.calls[0][0];
    expect(sent.path).toBe("/tmp/out.zip");
    expect(sent.description).toBe("the tunnel closed");
    expect(sent.includeLog).toBe(true);
    expect(sent.includeProjectSummary).toBe(false);
    expect(sent.includeBusTelegrams).toBe(true);
    expect(sent.client.shell).toBe("browser");
    expect(onClearErrors).toHaveBeenCalled();
    expect(onSummary).toHaveBeenCalledWith("Debug report saved, 3 files.");
  });

  it("writes nothing when the save dialog is dismissed", async () => {
    pickerMock.pickSavePath.mockResolvedValue(null);
    await openDialog();
    await click(en["debugReport.save"]);
    expect(apiMock.createDebugReport).not.toHaveBeenCalled();
  });

  it("asks for a preview, not a file, when opening a GitHub issue", async () => {
    apiMock.createDebugReport.mockResolvedValue(report({ written: false, path: null }));
    const opened = vi.fn();
    vi.stubGlobal("open", opened);
    const { onSummary } = await openDialog();

    await setDescription("something broke");
    await click(en["debugReport.openIssue"]);

    // No save dialog, and no path: opening an issue must not drop a file
    // on the user's disk as a side effect.
    expect(pickerMock.pickSavePath).not.toHaveBeenCalled();
    expect(apiMock.createDebugReport.mock.calls[0][0].path).toBeNull();

    expect(opened).toHaveBeenCalledTimes(1);
    const url = new URL(opened.mock.calls[0][0] as string);
    // A prefilled page, not an API call: GitHub's own new-issue form.
    expect(url.origin + url.pathname).toBe("https://github.com/KNXBench-Labs/KNXBench/issues/new");
    expect(url.searchParams.get("body")).toContain("something broke");
    expect(onSummary).toHaveBeenCalledWith(en["debugReport.issueOpened"]);
    vi.unstubAllGlobals();
  });

  it("reports a failure instead of claiming a report was saved", async () => {
    pickerMock.pickSavePath.mockResolvedValue("/tmp/out.zip");
    apiMock.createDebugReport.mockRejectedValue(new Error("disk full"));
    const { onSummary, onError } = await openDialog();

    await click(en["debugReport.save"]);

    expect(onError).toHaveBeenCalledTimes(1);
    expect(onSummary).not.toHaveBeenCalled();
    // The dialog stays up, so the user can pick a different target.
    expect(document.querySelector(".debug-report-panel")).not.toBeNull();
  });
});
