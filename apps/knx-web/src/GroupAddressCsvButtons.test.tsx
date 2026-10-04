// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { ProjectTree } from "./bindings/ProjectTree";

const apiMock = vi.hoisted(() => ({
  exportGroupAddressesCsv: vi.fn(),
  importGroupAddressesCsv: vi.fn(),
}));

const filePickerMock = vi.hoisted(() => ({
  pickSavePath: vi.fn(),
  pickOpenPath: vi.fn(),
}));

vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}));

vi.mock("./filePicker", () => ({ ...filePickerMock }));

import GroupAddressCsvButtons from "./GroupAddressCsvButtons";

let host: HTMLDivElement | undefined;

afterEach(() => {
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
});

// The component only checks `tree` for truthiness (disables both buttons
// when it is `null`) and never reads its fields, so a cast stand-in is
// enough — same precedent as CatalogBrowser.test.tsx/ProjectExplorer.test.tsx
// use for trees whose contents the component under test doesn't inspect.
const fakeTree = { installations: [] } as unknown as ProjectTree;

async function renderButtons(tree: ProjectTree | null = fakeTree) {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  const onTreeUpdate = vi.fn();
  const onSummary = vi.fn();
  const onError = vi.fn();
  const onClearErrors = vi.fn();
  await act(async () => {
    root.render(
      <GroupAddressCsvButtons
        tree={tree}
        onTreeUpdate={onTreeUpdate}
        onSummary={onSummary}
        onError={onError}
        onClearErrors={onClearErrors}
      />,
    );
  });
  return { root, onTreeUpdate, onSummary, onError, onClearErrors };
}

function exportButton() {
  return Array.from(host!.querySelectorAll("button")).find((b) =>
    b.textContent?.startsWith("Export group addresses"),
  )!;
}

function importButton() {
  return Array.from(host!.querySelectorAll("button")).find((b) =>
    b.textContent?.startsWith("Import group addresses"),
  )!;
}

async function click(button: HTMLButtonElement) {
  await act(async () => {
    button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
  });
}

describe("GroupAddressCsvButtons", () => {
  it("disables both buttons when no project is open", async () => {
    const { root } = await renderButtons(null);
    expect(exportButton().disabled).toBe(true);
    expect(importButton().disabled).toBe(true);
    root.unmount();
  });

  it("does not call the export route when the save dialog is cancelled", async () => {
    filePickerMock.pickSavePath.mockResolvedValueOnce(null);
    const { root, onSummary, onError, onClearErrors } = await renderButtons();
    await click(exportButton());
    expect(apiMock.exportGroupAddressesCsv).not.toHaveBeenCalled();
    expect(onSummary).not.toHaveBeenCalled();
    expect(onError).not.toHaveBeenCalled();
    // Matches `App.tsx`'s own save handler: a cancelled dialog returns
    // before `clearErrors()` runs at all, same as it never reaches the
    // route call above.
    expect(onClearErrors).not.toHaveBeenCalled();
    root.unmount();
  });

  it("clears a prior error toast before running the export, the way every file-writing handler does", async () => {
    filePickerMock.pickSavePath.mockResolvedValueOnce("/data/group-addresses.csv");
    apiMock.exportGroupAddressesCsv.mockResolvedValueOnce({ warnings: [] });
    const { root, onClearErrors, onSummary } = await renderButtons();
    await click(exportButton());
    // The assertion that matters: this fails if the `onClearErrors()` call
    // is removed from `exportCsv`, so a stale error toast from an earlier,
    // unrelated failure would otherwise still be showing after this
    // success.
    expect(onClearErrors).toHaveBeenCalledTimes(1);
    expect(onSummary).toHaveBeenCalledTimes(1);
    root.unmount();
  });

  it("reports a clean export with no warnings in one line", async () => {
    filePickerMock.pickSavePath.mockResolvedValueOnce("/data/group-addresses.csv");
    apiMock.exportGroupAddressesCsv.mockResolvedValueOnce({ warnings: [] });
    const { root, onSummary, onError } = await renderButtons();
    await click(exportButton());
    expect(apiMock.exportGroupAddressesCsv).toHaveBeenCalledWith("/data/group-addresses.csv");
    expect(onSummary).toHaveBeenCalledTimes(1);
    expect(onSummary).toHaveBeenCalledWith("Group addresses exported to CSV, no warnings.");
    expect(onError).not.toHaveBeenCalled();
    root.unmount();
  });

  it("carries the warning count into the export summary rather than dropping it", async () => {
    filePickerMock.pickSavePath.mockResolvedValueOnce("/data/group-addresses.csv");
    apiMock.exportGroupAddressesCsv.mockResolvedValueOnce({
      warnings: [
        { row: 3, severity: "warning", detail: "narrowest range chosen ambiguously" },
        { row: 7, severity: "warning", detail: "name truncated" },
      ],
    });
    const { root, onSummary } = await renderButtons();
    await click(exportButton());
    expect(onSummary).toHaveBeenCalledWith(
      "Group addresses exported to CSV, 2 warnings — see Log.",
    );
    root.unmount();
  });

  it("surfaces an export failure through onError instead of a summary", async () => {
    filePickerMock.pickSavePath.mockResolvedValueOnce("/data/group-addresses.csv");
    apiMock.exportGroupAddressesCsv.mockRejectedValueOnce(new Error("no project open"));
    const { root, onSummary, onError } = await renderButtons();
    await click(exportButton());
    expect(onError).toHaveBeenCalledTimes(1);
    expect(onSummary).not.toHaveBeenCalled();
    root.unmount();
  });

  it("does not call the import route when the open dialog is cancelled", async () => {
    filePickerMock.pickOpenPath.mockResolvedValueOnce(null);
    const { root, onTreeUpdate, onSummary, onClearErrors } = await renderButtons();
    await click(importButton());
    expect(apiMock.importGroupAddressesCsv).not.toHaveBeenCalled();
    expect(onTreeUpdate).not.toHaveBeenCalled();
    expect(onSummary).not.toHaveBeenCalled();
    expect(onClearErrors).not.toHaveBeenCalled();
    root.unmount();
  });

  it("clears a prior error toast before running the import, the way every file-writing handler does", async () => {
    filePickerMock.pickOpenPath.mockResolvedValueOnce("/data/in.csv");
    apiMock.importGroupAddressesCsv.mockResolvedValueOnce({
      tree: { installations: [] } as unknown as ProjectTree,
      report: {
        separator: ",",
        rowsRead: 1,
        created: 1,
        updated: 0,
        unchanged: 0,
        ignoredColumns: [],
        problems: [],
      },
    });
    const { root, onClearErrors, onSummary } = await renderButtons();
    await click(importButton());
    // Same fix as the export button: fails if `onClearErrors()` is
    // removed from `importCsv`, leaving a stale error toast visible after
    // a successful import.
    expect(onClearErrors).toHaveBeenCalledTimes(1);
    expect(onSummary).toHaveBeenCalledTimes(1);
    root.unmount();
  });

  it("applies the returned tree and reports created/updated/unchanged on a successful import", async () => {
    filePickerMock.pickOpenPath.mockResolvedValueOnce("/data/in.csv");
    const nextTree = { installations: [{ id: 99 }] } as unknown as ProjectTree;
    apiMock.importGroupAddressesCsv.mockResolvedValueOnce({
      tree: nextTree,
      report: {
        separator: ",",
        rowsRead: 6,
        created: 2,
        updated: 1,
        unchanged: 3,
        ignoredColumns: [],
        problems: [],
      },
    });
    const { root, onTreeUpdate, onSummary, onError } = await renderButtons();
    await click(importButton());
    expect(apiMock.importGroupAddressesCsv).toHaveBeenCalledWith("/data/in.csv");
    expect(onTreeUpdate).toHaveBeenCalledTimes(1);
    expect(onTreeUpdate).toHaveBeenCalledWith(nextTree);
    expect(onSummary).toHaveBeenCalledWith(
      "Group addresses imported from CSV: 2 created, 1 updated, 0 readdressed, 0 deleted, 3 unchanged.",
    );
    expect(onError).not.toHaveBeenCalled();
    root.unmount();
  });

  it("carries a successful import's warnings and ignored columns into the summary rather than dropping them", async () => {
    filePickerMock.pickOpenPath.mockResolvedValueOnce("/data/in.csv");
    apiMock.importGroupAddressesCsv.mockResolvedValueOnce({
      tree: { installations: [] } as unknown as ProjectTree,
      report: {
        separator: ",",
        rowsRead: 4,
        created: 1,
        updated: 0,
        unchanged: 2,
        // Import applied fine (200), but the file carried a column this
        // reader ignores and a row that only warranted a warning — a 200
        // response is not the same as "nothing to tell the user".
        ignoredColumns: [{ name: "Comment", reason: "unknown" }],
        problems: [{ row: 3, severity: "warning", detail: "narrowest range chosen ambiguously" }],
      },
    });
    const { root, onSummary, onError } = await renderButtons();
    await click(importButton());
    // This is the assertion that would still pass if the warning/ignored
    // counts were silently dropped from the message, so it must check the
    // exact string — not just that *a* summary fired.
    expect(onSummary).toHaveBeenCalledWith(
      "Group addresses imported from CSV: 1 created, 0 updated, 0 readdressed, 0 deleted, 2 unchanged, 1 warning, " +
        "1 column ignored — see Log.",
    );
    expect(onError).not.toHaveBeenCalled();
    root.unmount();
  });

  it("shows a destructive preview and only applies after explicit confirmation", async () => {
    filePickerMock.pickOpenPath.mockResolvedValueOnce("/data/readdress.csv");
    const confirm = vi.fn(() => true);
    Object.defineProperty(window, "confirm", { configurable: true, value: confirm });
    const report = {
      separator: ",",
      rowsRead: 1,
      created: 0,
      updated: 0,
      readdressed: 1,
      deleted: 0,
      unchanged: 0,
      destructiveChanges: [
        {
          row: 2,
          action: "readdress",
          id: 7,
          sourceAddress: 100,
          targetAddress: 200,
          affectedLinks: [{ comObject: 9, direction: "send" }],
        },
      ],
      ignoredColumns: [],
      problems: [],
    };
    const unchangedTree = { installations: [{ id: 1 }] } as unknown as ProjectTree;
    const changedTree = { installations: [{ id: 2 }] } as unknown as ProjectTree;
    apiMock.importGroupAddressesCsv
      .mockResolvedValueOnce({
        tree: unchangedTree,
        report,
        applied: false,
        confirmationToken: "preview-token",
      })
      .mockResolvedValueOnce({
        tree: changedTree,
        report,
        applied: true,
        confirmationToken: null,
      });

    const { root, onTreeUpdate } = await renderButtons();
    await click(importButton());

    expect(confirm).toHaveBeenCalledWith(expect.stringContaining("1 communication-object links"));
    expect(apiMock.importGroupAddressesCsv).toHaveBeenNthCalledWith(1, "/data/readdress.csv");
    expect(apiMock.importGroupAddressesCsv).toHaveBeenNthCalledWith(
      2,
      "/data/readdress.csv",
      "preview-token",
    );
    expect(onTreeUpdate).toHaveBeenCalledTimes(1);
    expect(onTreeUpdate).toHaveBeenCalledWith(changedTree);
    root.unmount();
  });

  it("rejects a bad import (400) without ever touching the open project's tree", async () => {
    filePickerMock.pickOpenPath.mockResolvedValueOnce("/data/bad.csv");
    apiMock.importGroupAddressesCsv.mockRejectedValueOnce(
      new Error("1 error(s), nothing applied: row 4: unknown group address style"),
    );
    const { root, onTreeUpdate, onSummary, onError } = await renderButtons();
    await click(importButton());
    // The assertion that matters: a rejected import must never reach
    // `onTreeUpdate`, i.e. the project the rest of the app sees is left
    // exactly as it was. Merely checking that an error surfaced would pass
    // even if the component wrongly called `onTreeUpdate` first.
    expect(onTreeUpdate).not.toHaveBeenCalled();
    expect(onError).toHaveBeenCalledTimes(1);
    expect(onError.mock.calls[0][0]).toBeInstanceOf(Error);
    expect(onSummary).not.toHaveBeenCalled();
    root.unmount();
  });
});

// MODEL-01: with several installations the CSV buttons name the one they act
// on; preview and confirmation must name the same one (the server binds the
// installation into its confirmation token).
describe("GroupAddressCsvButtons — installation choice", () => {
  const twoInstallations = { installations: [{ id: 1, name: "Main" }, { id: 2, name: "Annex" }] } as unknown as ProjectTree;

  function installationSelect(): HTMLSelectElement | null {
    return host!.querySelector<HTMLSelectElement>('select[aria-label="Installation for CSV"]');
  }

  it("offers no choice with a single installation", async () => {
    const { root } = await renderButtons({ installations: [{ id: 1, name: "Main" }] } as unknown as ProjectTree);
    expect(installationSelect()).toBeNull();
    root.unmount();
  });

  it("exports the chosen installation", async () => {
    filePickerMock.pickSavePath.mockResolvedValueOnce("/data/annex.csv");
    apiMock.exportGroupAddressesCsv.mockResolvedValueOnce({ warnings: [] });
    const { root } = await renderButtons(twoInstallations);
    const select = installationSelect()!;
    expect(Array.from(select.options, (option) => option.textContent)).toEqual(["Main", "Annex"]);
    expect(select.value).toBe("1");
    await act(async () => {
      select.value = "2";
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });
    await click(exportButton());
    expect(apiMock.exportGroupAddressesCsv).toHaveBeenCalledExactlyOnceWith("/data/annex.csv", 2);
    root.unmount();
  });

  it("previews and confirms a destructive import into the same chosen installation", async () => {
    filePickerMock.pickOpenPath.mockResolvedValueOnce("/data/annex.csv");
    Object.defineProperty(window, "confirm", { configurable: true, value: vi.fn(() => true) });
    const report = { separator: ",", rowsRead: 1, created: 0, updated: 0, readdressed: 0, deleted: 1, unchanged: 0,
      destructiveChanges: [{ row: 2, action: "delete", id: 5, sourceAddress: 4352, targetAddress: null, affectedLinks: [] }],
      ignoredColumns: [], problems: [] };
    apiMock.importGroupAddressesCsv
      .mockResolvedValueOnce({ tree: twoInstallations, report, applied: false, confirmationToken: "preview-token" })
      .mockResolvedValueOnce({ tree: twoInstallations, report, applied: true, confirmationToken: null });
    const { root } = await renderButtons(twoInstallations);
    const select = installationSelect()!;
    await act(async () => {
      select.value = "2";
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });
    await click(importButton());
    expect(apiMock.importGroupAddressesCsv.mock.calls).toEqual([
      ["/data/annex.csv", undefined, 2],
      ["/data/annex.csv", "preview-token", 2],
    ]);
    root.unmount();
  });
});
