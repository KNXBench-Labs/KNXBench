/** The status-bar journal note stays silent, follows a running write and offers a retry. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const apiMock = vi.hoisted(() => ({
  journalStatus: vi.fn(),
  retryJournal: vi.fn(),
}));
vi.mock("./api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("./api")>()),
  ...apiMock,
}));

import JournalStatusIndicator, { JOURNAL_POLL_MS } from "./JournalStatusIndicator";

let container: HTMLDivElement;
let root: Root;

beforeEach(() => {
  (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  apiMock.journalStatus.mockReset();
  apiMock.retryJournal.mockReset();
  container = document.createElement("div");
  document.body.append(container);
  root = createRoot(container);
});

afterEach(() => {
  act(() => root.unmount());
  container.remove();
  vi.useRealTimers();
});

async function show(revision: number, saved = true) {
  await act(async () => { root.render(<JournalStatusIndicator revision={revision} saved={saved} />); });
}

async function settle() {
  await act(async () => { await Promise.resolve(); await Promise.resolve(); });
}

describe("JournalStatusIndicator", () => {
  it("asks nothing before the first change and shows nothing when saved", async () => {
    await show(1);
    expect(apiMock.journalStatus).not.toHaveBeenCalled();
    expect(container.textContent).toBe("");
  });

  it("follows a running write until it is done", async () => {
    vi.useFakeTimers();
    apiMock.journalStatus
      .mockResolvedValueOnce({ state: "saving" })
      .mockResolvedValueOnce({ state: "saved" });
    await show(1);
    await show(2);
    await settle();
    expect(container.querySelector("[role=status]")?.textContent).toMatch(/Saving/);
    await act(async () => { await vi.advanceTimersByTimeAsync(JOURNAL_POLL_MS); });
    await settle();
    expect(apiMock.journalStatus).toHaveBeenCalledTimes(2);
    expect(container.textContent).toBe("");
  });

  it("keeps a failure visible and retries on request", async () => {
    apiMock.journalStatus.mockResolvedValue({ state: "failed", message: "disk full" });
    apiMock.retryJournal.mockResolvedValue({ state: "saving" });
    await show(1);
    await show(2);
    await settle();
    expect(container.querySelector("[role=alert]")?.textContent).toMatch(/disk full/);
    await act(async () => { container.querySelector("button")!.click(); });
    await settle();
    expect(apiMock.retryJournal).toHaveBeenCalledTimes(1);
  });

  it("asks nothing for a project without a file", async () => {
    await show(1, false);
    await show(2, false);
    expect(apiMock.journalStatus).not.toHaveBeenCalled();
  });

  it("shows nothing when the server has no journal route", async () => {
    apiMock.journalStatus.mockRejectedValue(new Error("404"));
    await show(1);
    await show(2);
    await settle();
    expect(container.textContent).toBe("");
  });
});
