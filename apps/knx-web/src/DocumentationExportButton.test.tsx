/** Tests that DocumentationExportButton gates on an open project and opens the documentation dialog. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ProjectTree } from "./bindings/ProjectTree";

const apiMock = vi.hoisted(() => ({
  previewDocumentation: vi.fn(),
  exportDocumentation: vi.fn(),
}));

vi.mock("./api", () => ({ ...apiMock, errorMessage: String }));

vi.mock("./filePicker", () => ({ pickSavePath: vi.fn() }));

import DocumentationExportButton from "./DocumentationExportButton";

let host: HTMLDivElement | undefined;

afterEach(() => {
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
});

// The component only checks `tree` for truthiness and never reads its fields.
const fakeTree = { installations: [] } as unknown as ProjectTree;

async function renderButton(tree: ProjectTree | null) {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(
      <DocumentationExportButton
        tree={tree}
        onSummary={vi.fn()}
        onError={vi.fn()}
        onClearErrors={vi.fn()}
      />,
    );
  });
  return root;
}

describe("DocumentationExportButton", () => {
  it("is disabled when no project is open", async () => {
    const root = await renderButton(null);
    expect(host!.querySelector("button")!.disabled).toBe(true);
    await act(async () => root.unmount());
  });

  it("opens the documentation dialog, which requests a preview", async () => {
    apiMock.previewDocumentation.mockResolvedValue({ html: "<p>doc</p>", warnings: [] });
    const root = await renderButton(fakeTree);
    const button = host!.querySelector("button")!;
    expect(button.textContent).toBe("Export documentation…");
    await act(async () => {
      button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(document.body.querySelector(".documentation-dialog")).not.toBeNull();
    expect(apiMock.previewDocumentation).toHaveBeenCalledTimes(1);
    await act(async () => root.unmount());
    expect(document.body.querySelector(".documentation-dialog")).toBeNull();
  });
});
