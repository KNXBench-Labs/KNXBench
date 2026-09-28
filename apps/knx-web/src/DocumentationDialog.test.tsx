/** Tests for DocumentationDialog: section selection, sandboxed preview, warnings, print, export and errors. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { DocumentationPreview } from "./api";
import { resetUiLanguageForTests, saveUiLanguage } from "./uiLanguage";
import { resetSettingsForTests, settingsStorage } from "./settingsStore";

const apiMock = vi.hoisted(() => ({
  previewDocumentation: vi.fn(),
  exportDocumentation: vi.fn(),
}));

const filePickerMock = vi.hoisted(() => ({
  pickSavePath: vi.fn(),
}));

vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}));

vi.mock("./filePicker", () => ({ ...filePickerMock }));

import DocumentationDialog, { PREVIEW_SANDBOX } from "./DocumentationDialog";

const cleanPreview: DocumentationPreview = {
  html: "<!doctype html><html><body><h1>Project documentation</h1></body></html>",
  warnings: [],
};

let root: Root | undefined;
let host: HTMLDivElement | undefined;

afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  root = undefined;
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
  resetSettingsForTests();
  resetUiLanguageForTests();
});

async function renderDialog() {
  host = document.createElement("div");
  document.body.appendChild(host);
  root = createRoot(host);
  const callbacks = {
    onSummary: vi.fn(),
    onError: vi.fn(),
    onClearErrors: vi.fn(),
    onClose: vi.fn(),
  };
  await act(async () => {
    root!.render(<DocumentationDialog {...callbacks} />);
  });
  return callbacks;
}

function dialog(): HTMLElement {
  return document.body.querySelector<HTMLElement>(".documentation-dialog")!;
}

function button(label: string): HTMLButtonElement {
  return Array.from(dialog().querySelectorAll("button")).find((b) => b.textContent === label)!;
}

function checkbox(section: string): HTMLInputElement {
  return dialog().querySelector<HTMLInputElement>(`input[value="${section}"]`)!;
}

async function click(element: HTMLElement) {
  await act(async () => {
    element.dispatchEvent(new MouseEvent("click", { bubbles: true }));
  });
}

describe("DocumentationDialog", () => {
  it("requests every section in the UI language when it opens", async () => {
    apiMock.previewDocumentation.mockResolvedValue(cleanPreview);
    await renderDialog();
    expect(apiMock.previewDocumentation).toHaveBeenCalledWith({
      sections: ["summary", "topology", "buildings", "groupAddresses", "devices"],
      language: "en",
    });
    expect(dialog().getAttribute("role")).toBe("dialog");
  });

  it("sends the same section selection to preview and export", async () => {
    apiMock.previewDocumentation.mockResolvedValue(cleanPreview);
    apiMock.exportDocumentation.mockResolvedValue({ warnings: [] });
    filePickerMock.pickSavePath.mockResolvedValue("/data/project.html");
    await renderDialog();

    // Unchecked out of document order; the request stays in document order.
    await click(checkbox("devices"));
    await click(checkbox("topology"));
    await click(button("Export…"));

    const expected = { sections: ["summary", "buildings", "groupAddresses"], language: "en" };
    expect(apiMock.previewDocumentation).toHaveBeenLastCalledWith(expected);
    expect(apiMock.exportDocumentation).toHaveBeenCalledWith("/data/project.html", expected);
    const previewOptions = apiMock.previewDocumentation.mock.lastCall![0];
    const exportOptions = apiMock.exportDocumentation.mock.lastCall![1];
    expect(exportOptions).toStrictEqual(previewOptions);
  });

  it("allows an empty selection, which still yields header, contents and limits", async () => {
    apiMock.previewDocumentation.mockResolvedValue(cleanPreview);
    await renderDialog();
    for (const section of ["summary", "topology", "buildings", "groupAddresses", "devices"]) {
      await click(checkbox(section));
    }
    expect(apiMock.previewDocumentation).toHaveBeenLastCalledWith({ sections: [], language: "en" });
    expect(dialog().textContent).toContain("Header, contents and limits are always included.");
  });

  it("renders the preview in a sandboxed srcdoc frame that may not run scripts", async () => {
    apiMock.previewDocumentation.mockResolvedValue(cleanPreview);
    await renderDialog();
    const frame = dialog().querySelector("iframe")!;
    expect(frame.getAttribute("sandbox")).toBe(PREVIEW_SANDBOX);
    const tokens = frame.getAttribute("sandbox")!.split(" ");
    expect(tokens).not.toContain("allow-scripts");
    expect(tokens).not.toContain("allow-top-navigation");
    expect(frame.getAttribute("srcdoc")).toBe(cleanPreview.html);
    expect(frame.hasAttribute("src")).toBe(false);
    expect(frame.getAttribute("title")).toBe("Documentation preview");
  });

  it("lists the preview warnings next to the preview", async () => {
    apiMock.previewDocumentation.mockResolvedValue({
      html: cleanPreview.html,
      warnings: [
        { location: "line 1.1", detail: "device has no line" },
        { location: "GA 1/2/3", detail: "orphaned group address" },
      ],
    });
    await renderDialog();
    const aside = dialog().querySelector("aside")!;
    expect(aside.querySelector("h3")!.textContent).toBe("Warnings (2)");
    const items = Array.from(aside.querySelectorAll("li")).map((li) => li.textContent);
    expect(items).toEqual(["line 1.1 device has no line", "GA 1/2/3 orphaned group address"]);
  });

  it("says so when the preview has no warnings", async () => {
    apiMock.previewDocumentation.mockResolvedValue(cleanPreview);
    await renderDialog();
    expect(dialog().querySelector("aside")!.textContent).toContain("No warnings.");
  });

  it("opens the print dialog of the preview document, not of the application", async () => {
    apiMock.previewDocumentation.mockResolvedValue(cleanPreview);
    const { onError } = await renderDialog();
    const frame = dialog().querySelector("iframe")!;
    const framePrint = vi.fn();
    Object.defineProperty(frame, "contentWindow", { value: { print: framePrint } });
    // happy-dom has no `window.print`; a stub proves the page itself is
    // never the one being printed.
    const appPrint = vi.fn();
    Object.defineProperty(window, "print", { value: appPrint, configurable: true });

    await click(button("Print…"));

    expect(framePrint).toHaveBeenCalledTimes(1);
    expect(appPrint).not.toHaveBeenCalled();
    expect(onError).not.toHaveBeenCalled();
    Reflect.deleteProperty(window, "print");
  });

  it("reports a print failure instead of swallowing it", async () => {
    apiMock.previewDocumentation.mockResolvedValue(cleanPreview);
    const { onError } = await renderDialog();
    const frame = dialog().querySelector("iframe")!;
    const failure = new Error("printing blocked");
    Object.defineProperty(frame, "contentWindow", {
      value: {
        print: () => {
          throw failure;
        },
      },
    });
    await click(button("Print…"));
    expect(onError).toHaveBeenCalledWith(failure);
  });

  it("shows a preview error as an alert and disables print", async () => {
    apiMock.previewDocumentation.mockRejectedValue(new Error("no project open"));
    await renderDialog();
    const alert = dialog().querySelector('[role="alert"]')!;
    expect(alert.textContent).toBe("Preview failed: no project open");
    expect(dialog().querySelector("iframe")).toBeNull();
    expect(button("Print…").disabled).toBe(true);
  });

  it("disables print and marks the preview busy while it loads", async () => {
    apiMock.previewDocumentation.mockReturnValue(new Promise(() => {}));
    await renderDialog();
    expect(button("Print…").disabled).toBe(true);
    expect(dialog().querySelector(".documentation-preview")!.getAttribute("aria-busy")).toBe("true");
    expect(dialog().textContent).toContain("Loading preview…");
  });

  it("ignores a preview response that arrives after the selection changed", async () => {
    let resolveStale!: (preview: DocumentationPreview) => void;
    apiMock.previewDocumentation
      .mockReturnValueOnce(new Promise((resolve) => (resolveStale = resolve)))
      .mockResolvedValueOnce({ html: "<p>current</p>", warnings: [] });
    await renderDialog();
    await click(checkbox("devices"));
    await act(async () => {
      resolveStale({ html: "<p>stale</p>", warnings: [{ location: "x", detail: "stale" }] });
    });
    expect(dialog().querySelector("iframe")!.getAttribute("srcdoc")).toBe("<p>current</p>");
    expect(dialog().querySelector("aside h3")!.textContent).toBe("Warnings (0)");
  });

  it("does nothing when the save dialog is cancelled", async () => {
    apiMock.previewDocumentation.mockResolvedValue(cleanPreview);
    filePickerMock.pickSavePath.mockResolvedValue(null);
    const { onSummary, onError, onClearErrors, onClose } = await renderDialog();
    await click(button("Export…"));
    expect(apiMock.exportDocumentation).not.toHaveBeenCalled();
    expect(onSummary).not.toHaveBeenCalled();
    expect(onError).not.toHaveBeenCalled();
    expect(onClearErrors).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
  });

  it("clears old errors, reports the warning count and closes after an export", async () => {
    apiMock.previewDocumentation.mockResolvedValue(cleanPreview);
    filePickerMock.pickSavePath.mockResolvedValue("/data/project.html");
    apiMock.exportDocumentation.mockResolvedValue({
      warnings: [
        { location: "line 1.1", detail: "device has no line" },
        { location: "GA 1/2/3", detail: "orphaned group address" },
      ],
    });
    const { onSummary, onError, onClearErrors, onClose } = await renderDialog();
    await click(button("Export…"));
    expect(filePickerMock.pickSavePath).toHaveBeenCalledWith(
      [{ name: "HTML document", extensions: ["html"] }],
      "project-documentation.html",
    );
    expect(onClearErrors).toHaveBeenCalledTimes(1);
    expect(onSummary).toHaveBeenCalledWith("Project documentation exported, 2 warnings — see Log.");
    expect(onError).not.toHaveBeenCalled();
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("surfaces an export failure through onError and stays open", async () => {
    apiMock.previewDocumentation.mockResolvedValue(cleanPreview);
    filePickerMock.pickSavePath.mockResolvedValue("/data/project.html");
    apiMock.exportDocumentation.mockRejectedValue(new Error("disk full"));
    const { onSummary, onError, onClose } = await renderDialog();
    await click(button("Export…"));
    expect(onError).toHaveBeenCalledTimes(1);
    expect(onSummary).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
  });

  it("closes on Escape and on the Close button", async () => {
    apiMock.previewDocumentation.mockResolvedValue(cleanPreview);
    const { onClose } = await renderDialog();
    await act(async () => {
      dialog().dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    });
    await click(button("Close"));
    expect(onClose).toHaveBeenCalledTimes(2);
  });

  it("speaks German and requests the German document when the UI language is German", async () => {
    saveUiLanguage(settingsStorage, "de");
    resetUiLanguageForTests();
    apiMock.previewDocumentation.mockRejectedValue(new Error("kein Projekt"));
    filePickerMock.pickSavePath.mockResolvedValue(null);
    await renderDialog();
    expect(apiMock.previewDocumentation).toHaveBeenCalledWith(
      expect.objectContaining({ language: "de" }),
    );
    expect(dialog().querySelector("h2")!.textContent).toBe("Projektdokumentation");
    expect(dialog().querySelector("legend")!.textContent).toBe("Abschnitte");
    expect(dialog().querySelector('[role="alert"]')!.textContent).toBe(
      "Vorschau fehlgeschlagen: kein Projekt",
    );
    await click(button("Exportieren…"));
    expect(filePickerMock.pickSavePath).toHaveBeenCalledWith(
      [{ name: "HTML-Dokument", extensions: ["html"] }],
      "project-documentation.html",
    );
  });
});
