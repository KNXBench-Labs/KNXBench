/** Verifies line-scan cost, six-state reporting, cancellation and protected exclusions. */
// @vitest-environment happy-dom

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const apiMock = vi.hoisted(() => ({
  estimateLineScan: vi.fn(),
  startLineScan: vi.fn(),
  pollLineScan: vi.fn(),
  cancelLineScan: vi.fn(),
  compareLineScan: vi.fn(),
  reconcileLineScan: vi.fn(),
}));

vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
  errorStatus: (error: unknown) =>
    error && typeof error === "object" && "status" in error
      ? (error as { status: number }).status
      : undefined,
}));

import LineScanPanel from "./LineScanPanel";
import { savePreferredGateway } from "./gatewayPreference";
import { getSetting, initSettings, resetSettingsForTests, setSetting } from "./settingsStore";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let host: HTMLDivElement | undefined;
let root: Root | undefined;

function notFound(): Error {
  const error = new Error("no line scan") as Error & { status: number };
  error.status = 404;
  return error;
}

async function renderPanel(onTreeUpdate?: (tree: unknown) => void) {
  host = document.createElement("div");
  document.body.appendChild(host);
  root = createRoot(host);
  await act(async () => {
    root!.render(<LineScanPanel onTreeUpdate={onTreeUpdate} />);
    await Promise.resolve();
    await Promise.resolve();
  });
  return root;
}

function click(label: string) {
  const button = Array.from(host!.querySelectorAll("button")).find((item) => item.textContent === label);
  if (!button) throw new Error(`button not found: ${label}`);
  button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
}

function setInput(selector: string, value: string) {
  const input = host!.querySelector<HTMLInputElement>(selector)!;
  const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!;
  setter.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

beforeEach(() => {
  vi.useFakeTimers();
  window.localStorage.clear();
  resetSettingsForTests();
  apiMock.pollLineScan.mockRejectedValue(notFound());
  apiMock.estimateLineScan.mockResolvedValue({
    candidateCount: 3,
    omittedAddresses: [],
    responseTimeoutMs: 6000,
    vacantConfirmations: 1,
    interProbePauseMs: 100,
    worstCaseMs: 18200,
  });
  apiMock.startLineScan.mockResolvedValue({ sessionId: 4, estimate: awaitableEstimate() });
  apiMock.cancelLineScan.mockResolvedValue(scanResponse("cancelled", []));
  apiMock.compareLineScan.mockResolvedValue({
    unexpected: ["1.1.4"],
    missing: ["1.1.2"],
    excludedInProject: ["1.1.3"],
  });
  apiMock.reconcileLineScan.mockResolvedValue({ can_undo: true });
});

afterEach(async () => {
  if (root) {
    await act(async () => root!.unmount());
    root = undefined;
  }
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
  vi.useRealTimers();
});

function awaitableEstimate() {
  return {
    candidateCount: 3,
    omittedAddresses: [],
    responseTimeoutMs: 6000,
    vacantConfirmations: 1,
    interProbePauseMs: 100,
    worstCaseMs: 18200,
  };
}

function scanResponse(status: "running" | "completed" | "cancelled" | "failed", results: unknown[]) {
  return {
    sessionId: 4,
    status,
    error: null,
    nextSince: results.length,
    completedCount: results.length,
    totalCount: 6,
    omittedAddresses: [],
    excludedAddresses: [],
    results,
  };
}

describe("LineScanPanel", () => {
  it("seeds a fresh gateway field from the cached preference without estimating", async () => {
    savePreferredGateway("192.0.2.10:3671");
    await renderPanel();
    expect(
      host!.querySelector<HTMLInputElement>(
        '.line-scan-config input[placeholder="192.0.2.10:3671"]',
      )!.value,
    ).toBe("192.0.2.10:3671");
    expect(getSetting("preferredGateway")).toBe("192.0.2.10:3671");
    expect(apiMock.estimateLineScan).not.toHaveBeenCalled();
  });

  it("adopts the authoritative gateway after an initial 404 poll", async () => {
    savePreferredGateway("192.0.2.10:3671");
    let resolveGet!: (response: Response) => void;
    vi.stubGlobal("fetch", vi.fn(() => new Promise<Response>((resolve) => { resolveGet = resolve; })));
    const hydration = initSettings();
    await renderPanel();
    resolveGet({
      ok: true,
      status: 200,
      json: () => Promise.resolve({
        schemaVersion: 1,
        status: "ok",
        settings: { preferredGateway: "192.0.2.20:3671" },
      }),
    } as Response);
    await act(async () => { await hydration; });
    expect(
      host!.querySelector<HTMLInputElement>(
        '.line-scan-config input[placeholder="192.0.2.10:3671"]',
      )!.value,
    ).toBe("192.0.2.20:3671");
  });

  it("keeps manual gateway input typed before authoritative hydration", async () => {
    let resolveGet!: (response: Response) => void;
    vi.stubGlobal("fetch", vi.fn(() => new Promise<Response>((resolve) => { resolveGet = resolve; })));
    const hydration = initSettings();
    await renderPanel();
    await act(async () => {
      setInput('.line-scan-config input[placeholder="192.0.2.10:3671"]', "192.0.2.30:3671");
    });
    resolveGet({
      ok: true,
      status: 200,
      json: () => Promise.resolve({
        schemaVersion: 1,
        status: "ok",
        settings: { preferredGateway: "192.0.2.20:3671" },
      }),
    } as Response);
    await act(async () => { await hydration; });
    expect(
      host!.querySelector<HTMLInputElement>(
        '.line-scan-config input[placeholder="192.0.2.10:3671"]',
      )!.value,
    ).toBe("192.0.2.30:3671");
    expect(getSetting("preferredGateway")).toBe("192.0.2.20:3671");
  });

  it("keeps the cached gateway once an estimate starts before hydration", async () => {
    savePreferredGateway("192.0.2.10:3671");
    let resolveGet!: (response: Response) => void;
    vi.stubGlobal("fetch", vi.fn(() => new Promise<Response>((resolve) => { resolveGet = resolve; })));
    const hydration = initSettings();
    await renderPanel();
    await act(async () => {
      click("Estimate cost");
      await Promise.resolve();
    });
    expect(apiMock.estimateLineScan).toHaveBeenCalledWith(
      expect.objectContaining({ gateway: "192.0.2.10:3671" }),
    );
    resolveGet({
      ok: true,
      status: 200,
      json: () => Promise.resolve({
        schemaVersion: 1,
        status: "ok",
        settings: { preferredGateway: "192.0.2.20:3671" },
      }),
    } as Response);
    await act(async () => { await hydration; });
    expect(
      host!.querySelector<HTMLInputElement>(
        '.line-scan-config input[placeholder="192.0.2.10:3671"]',
      )!.value,
    ).toBe("192.0.2.10:3671");
  });

  it("shows an inspectable cost estimate before enabling start", async () => {
    await renderPanel();
    expect(host!.querySelector<HTMLButtonElement>(".line-scan-start")!.disabled).toBe(true);

    await act(async () => {
      setInput('.line-scan-config input[placeholder="192.0.2.10:3671"]', "192.0.2.10:3671");
      click("Estimate cost");
      await Promise.resolve();
    });

    expect(host!.textContent).toContain("3 candidate addresses");
    expect(host!.textContent).toContain("6000 ms timeout × 1 confirmation");
    expect(host!.textContent).toContain("100 ms pause");
    expect(host!.textContent).toContain("18.2 s response/pacing budget; transport overhead excluded");
    expect(host!.querySelector<HTMLButtonElement>(".line-scan-start")!.disabled).toBe(false);
    expect(apiMock.startLineScan).not.toHaveBeenCalled();
  });

  it("does not authorize changed inputs with a stale estimate response", async () => {
    let releaseEstimate: ((value: ReturnType<typeof awaitableEstimate>) => void) | undefined;
    const pendingEstimate = new Promise<ReturnType<typeof awaitableEstimate>>((resolve) => {
      releaseEstimate = resolve;
    });
    apiMock.estimateLineScan.mockReturnValueOnce(pendingEstimate);
    await renderPanel();

    await act(async () => {
      setInput('.line-scan-config input[placeholder="192.0.2.10:3671"]', "192.0.2.10:3671");
      click("Estimate cost");
      await Promise.resolve();
    });
    await act(async () => {
      const inputs = host!.querySelectorAll<HTMLInputElement>('.line-scan-config input[type="number"]');
      const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!;
      setter.call(inputs[2], "2");
      inputs[2].dispatchEvent(new Event("input", { bubbles: true }));
      releaseEstimate!(awaitableEstimate());
      await pendingEstimate;
    });

    expect(host!.querySelector<HTMLButtonElement>(".line-scan-start")!.disabled).toBe(true);
  });

  it("renders all six outcomes as distinct named states", async () => {
    apiMock.pollLineScan.mockResolvedValue(
      scanResponse("completed", [
        { address: "2.3.1", outcome: { kind: "occupied", maskVersion: 1793 } },
        { address: "2.3.2", outcome: { kind: "occupiedBusy" } },
        { address: "2.3.3", outcome: { kind: "occupiedSilent" } },
        { address: "2.3.4", outcome: { kind: "vacant" } },
        { address: "2.3.5", outcome: { kind: "indeterminate" } },
        { address: "2.3.6", outcome: { kind: "selfAddress" } },
      ]),
    );
    await renderPanel();

    const labels = Array.from(host!.querySelectorAll("[data-scan-outcome]")).map((node) => node.textContent);
    expect(labels).toEqual([
      "Occupied · mask 0x0701",
      "Occupied · busy",
      "Occupied · silent",
      "Vacant · no answer in window",
      "Indeterminate · frames lost",
      "Scanner address · not probed",
    ]);
  });

  it("renders scan outcomes in the active UI language", async () => {
    setSetting("uiLanguage", "de");
    apiMock.pollLineScan.mockResolvedValue(
      scanResponse("completed", [
        { address: "2.3.1", outcome: { kind: "occupiedBusy" } },
        { address: "2.3.2", outcome: { kind: "vacant" } },
      ]),
    );

    await renderPanel();

    const labels = Array.from(host!.querySelectorAll("[data-scan-outcome]")).map(
      (node) => node.textContent,
    );
    expect(labels).toEqual(["Belegt · beschäftigt", "Frei · keine Antwort im Zeitfenster"]);
  });

  it("never overlaps incremental result polls", async () => {
    let releasePoll: ((value: ReturnType<typeof scanResponse>) => void) | undefined;
    const pendingPoll = new Promise<ReturnType<typeof scanResponse>>((resolve) => {
      releasePoll = resolve;
    });
    apiMock.pollLineScan
      .mockRejectedValueOnce(notFound())
      .mockReturnValueOnce(pendingPoll)
      .mockResolvedValue(scanResponse("running", []));

    await renderPanel();
    await act(async () => {
      setInput('.line-scan-config input[placeholder="192.0.2.10:3671"]', "192.0.2.10:3671");
      click("Estimate cost");
      await Promise.resolve();
    });
    await act(async () => {
      click("Start read-only scan");
      await Promise.resolve();
    });

    expect(apiMock.pollLineScan).toHaveBeenCalledTimes(2);
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1_000);
    });
    expect(apiMock.pollLineScan).toHaveBeenCalledTimes(2);

    await act(async () => {
      releasePoll!(scanResponse("completed", []));
      await pendingPoll;
    });
  });

  it("cancels the running server scan rather than merely stopping polls", async () => {
    apiMock.pollLineScan.mockRejectedValueOnce(notFound()).mockResolvedValue(scanResponse("running", []));
    await renderPanel();
    await act(async () => setInput('.line-scan-config input[placeholder="192.0.2.10:3671"]', "192.0.2.10:3671"));
    await act(async () => {
      click("Estimate cost");
      await Promise.resolve();
    });
    await act(async () => {
      click("Start read-only scan");
      await Promise.resolve();
    });
    await act(async () => {
      click("Cancel scan");
      await Promise.resolve();
    });

    expect(apiMock.cancelLineScan).toHaveBeenCalledOnce();
    expect(apiMock.cancelLineScan).toHaveBeenCalledWith(4);
    expect(host!.textContent).toContain("Cancelled");
  });

  it("does not append a timer poll that finishes after cancellation", async () => {
    const finalResult = {
      address: "2.3.42",
      outcome: { kind: "occupiedBusy" },
    } as const;
    let releasePoll: ((value: ReturnType<typeof scanResponse>) => void) | undefined;
    let releaseCancel: ((value: ReturnType<typeof scanResponse>) => void) | undefined;
    const pendingPoll = new Promise<ReturnType<typeof scanResponse>>((resolve) => {
      releasePoll = resolve;
    });
    const pendingCancel = new Promise<ReturnType<typeof scanResponse>>((resolve) => {
      releaseCancel = resolve;
    });
    apiMock.pollLineScan
      .mockRejectedValueOnce(notFound())
      .mockResolvedValueOnce(scanResponse("running", []))
      .mockReturnValueOnce(pendingPoll);
    apiMock.cancelLineScan.mockReturnValueOnce(pendingCancel);
    await renderPanel();
    await act(async () => {
      setInput('.line-scan-config input[placeholder="192.0.2.10:3671"]', "192.0.2.10:3671");
      click("Estimate cost");
      await Promise.resolve();
    });
    await act(async () => {
      click("Start read-only scan");
      await Promise.resolve();
    });

    await act(async () => {
      click("Cancel scan");
      await vi.advanceTimersByTimeAsync(1_000);
    });
    await act(async () => {
      releaseCancel!(scanResponse("cancelled", [finalResult]));
      await pendingCancel;
      releasePoll!(scanResponse("cancelled", [finalResult]));
      await pendingPoll;
    });

    expect(host!.querySelectorAll("[data-scan-outcome]")).toHaveLength(1);
  });

  it("locks exclusion editing while the active scan plan is immutable", async () => {
    setSetting("lineScanExclusions", ["2.3.42"]);
    apiMock.pollLineScan.mockRejectedValueOnce(notFound()).mockResolvedValue(scanResponse("running", []));
    await renderPanel();
    await act(async () => {
      setInput('.line-scan-config input[placeholder="192.0.2.10:3671"]', "192.0.2.10:3671");
      click("Estimate cost");
      await Promise.resolve();
    });
    await act(async () => {
      click("Start read-only scan");
      await Promise.resolve();
    });

    expect(host!.querySelector<HTMLInputElement>(".line-scan-add-exclusion input")!.disabled).toBe(true);
    expect(Array.from(host!.querySelectorAll<HTMLButtonElement>(".line-scan-exclusions button")).every((button) => button.disabled)).toBe(true);
  });

  it("requires a deliberate confirmation before removing a configured exclusion", async () => {
    setSetting("lineScanExclusions", ["2.3.42"]);
    await renderPanel();
    expect(host!.textContent).toContain("2.3.42");
    expect(host!.textContent).toContain("Protected");

    await act(async () => click("Remove exclusion"));
    expect(getSetting("lineScanExclusions")).toEqual(["2.3.42"]);
    expect(host!.textContent).toContain("Confirm removal");

    await act(async () => click("Confirm removal"));
    expect(getSetting("lineScanExclusions")).toEqual([]);
    expect(host!.textContent).not.toContain("2.3.42");
  });

  it("shows completed evidence unselected and only reconciles deliberate choices", async () => {
    apiMock.pollLineScan.mockResolvedValue(scanResponse("completed", []));
    const onTreeUpdate = vi.fn();
    await renderPanel(onTreeUpdate);
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(apiMock.compareLineScan).toHaveBeenCalledWith(4);
    expect(host!.textContent).toContain("Bus answered, not in project");
    expect(host!.textContent).toContain("In project, no answer");
    expect(host!.textContent).toContain("In project, not examined");
    const choices = host!.querySelectorAll<HTMLInputElement>(
      '.line-scan-reconciliation input[type="checkbox"]',
    );
    expect(choices).toHaveLength(2);
    expect(Array.from(choices).every((choice) => !choice.checked)).toBe(true);
    const apply = Array.from(host!.querySelectorAll<HTMLButtonElement>("button")).find(
      (button) => button.textContent === "Apply selected changes",
    )!;
    expect(apply.disabled).toBe(true);

    await act(async () => choices[0].click());
    expect(apply.disabled).toBe(false);
    await act(async () => {
      apply.click();
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(apiMock.reconcileLineScan).toHaveBeenCalledWith(4, ["1.1.4"], []);
    expect(onTreeUpdate).toHaveBeenCalledWith({ can_undo: true });
  });

  it("refreshes completed evidence and clears selections when the project changes", async () => {
    apiMock.pollLineScan.mockResolvedValue(scanResponse("completed", []));
    await renderPanel();
    await act(async () => {
      await Promise.resolve();
      await Promise.resolve();
    });

    const firstChoice = host!.querySelector<HTMLInputElement>(
      '.line-scan-reconciliation input[type="checkbox"]',
    )!;
    await act(async () => firstChoice.click());
    expect(firstChoice.checked).toBe(true);

    await act(async () => {
      const changedProjectProps = { projectRevision: { revision: 2 } } as unknown as
        React.ComponentProps<typeof LineScanPanel>;
      root!.render(<LineScanPanel {...changedProjectProps} />);
      await Promise.resolve();
      await Promise.resolve();
    });

    expect(apiMock.compareLineScan).toHaveBeenCalledTimes(2);
    expect(
      host!.querySelector<HTMLInputElement>(
        '.line-scan-reconciliation input[type="checkbox"]',
      )!.checked,
    ).toBe(false);
    expect(
      Array.from(host!.querySelectorAll<HTMLButtonElement>("button")).find(
        (button) => button.textContent === "Apply selected changes",
      )!.disabled,
    ).toBe(true);
  });
});
