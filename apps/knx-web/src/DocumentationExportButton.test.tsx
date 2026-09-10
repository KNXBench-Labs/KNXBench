// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ProjectTree } from "./bindings/ProjectTree";

const apiMock = vi.hoisted(() => ({
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

import DocumentationExportButton from "./DocumentationExportButton";

let host: HTMLDivElement | undefined;

afterEach(() => {
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
});

// Same precedent as GroupAddressCsvButtons.test.tsx: the component only
// checks `tree` for truthiness (disables the button when it is `null`) and
// never reads its fields.
const fakeTree = { installations: [] } as unknown as ProjectTree;

async function renderButton(tree: ProjectTree | null = fakeTree) {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  const onSummary = vi.fn();
  const onError = vi.fn();
  const onClearErrors = vi.fn();
  await act(async () => {
    root.render(
      <DocumentationExportButton
        tree={tree}
        onSummary={onSummary}
        onError={onError}
        onClearErrors={onClearErrors}
      />,
    );
  });
  return { root, onSummary, onError, onClearErrors };
}

function exportButton() {
  return Array.from(host!.querySelectorAll("button")).find((b) =>
    b.textContent?.startsWith("Export documentation"),
  )!;
}

async function click(button: HTMLButtonElement) {
  await act(async () => {
    button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
  });
}

describe("DocumentationExportButton", () => {
  it("is disabled when no project is open", async () => {
    const { root } = await renderButton(null);
    expect(exportButton().disabled).toBe(true);
    root.unmount();
  });

  it("does not call the API or any callback when the save dialog is cancelled", async () => {
    filePickerMock.pickSavePath.mockResolvedValueOnce(null);
    const { root, onSummary, onError, onClearErrors } = await renderButton();
    await click(exportButton());
    expect(apiMock.exportDocumentation).not.toHaveBeenCalled();
    expect(onSummary).not.toHaveBeenCalled();
    expect(onError).not.toHaveBeenCalled();
    expect(onClearErrors).not.toHaveBeenCalled();
    root.unmount();
  });

  it("clears a prior error toast before running the export", async () => {
    filePickerMock.pickSavePath.mockResolvedValueOnce("/data/project.html");
    apiMock.exportDocumentation.mockResolvedValueOnce({ warnings: [] });
    const { root, onClearErrors, onSummary } = await renderButton();
    await click(exportButton());
    expect(onClearErrors).toHaveBeenCalledTimes(1);
    expect(onSummary).toHaveBeenCalledTimes(1);
    root.unmount();
  });

  it("passes the picked path with the 'HTML document' filter and reports a one-line summary carrying the warning count", async () => {
    filePickerMock.pickSavePath.mockResolvedValueOnce("/data/project.html");
    apiMock.exportDocumentation.mockResolvedValueOnce({
      warnings: [
        { location: "line 1.1", detail: "device has no line" },
        { location: "GA 1/2/3", detail: "orphaned group address" },
      ],
    });
    const { root, onSummary, onError } = await renderButton();
    await click(exportButton());
    expect(filePickerMock.pickSavePath).toHaveBeenCalledWith(
      [{ name: "HTML document", extensions: ["html"] }],
      expect.any(String),
    );
    expect(apiMock.exportDocumentation).toHaveBeenCalledWith("/data/project.html");
    expect(onSummary).toHaveBeenCalledTimes(1);
    expect(onSummary.mock.calls[0][0]).toContain("2");
    expect(onError).not.toHaveBeenCalled();
    root.unmount();
  });

  it("reports a clean export with no warnings in one line", async () => {
    filePickerMock.pickSavePath.mockResolvedValueOnce("/data/project.html");
    apiMock.exportDocumentation.mockResolvedValueOnce({ warnings: [] });
    const { root, onSummary, onError } = await renderButton();
    await click(exportButton());
    expect(onSummary).toHaveBeenCalledTimes(1);
    expect(onError).not.toHaveBeenCalled();
    root.unmount();
  });

  it("surfaces a thrown error through onError instead of a summary", async () => {
    filePickerMock.pickSavePath.mockResolvedValueOnce("/data/project.html");
    apiMock.exportDocumentation.mockRejectedValueOnce(new Error("no project open"));
    const { root, onSummary, onError } = await renderButton();
    await click(exportButton());
    expect(onError).toHaveBeenCalledTimes(1);
    expect(onSummary).not.toHaveBeenCalled();
    root.unmount();
  });
});
