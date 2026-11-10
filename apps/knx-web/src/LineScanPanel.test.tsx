/** Verifies line-scan cost, six-state reporting, cancellation and protected exclusions. */
// @vitest-environment happy-dom

import { tableColumnSmoke } from "./tableColumnSmoke.fixture";
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
  apiMock.startLineScan.mockResolvedValue({ sessionId: 4, serverIncarnation: "scan-test", transport: "udp", skippedFrames: null, estimate: awaitableEstimate() });
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
  vi.resetAllMocks();
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
    serverIncarnation: "scan-test",
    transport: "udp" as const,
    skippedFrames: null,
    disconnectConfirmed: status === "running" ? null : true,
    cleanupError: null,
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
  it.each([false, true].flatMap(detach => ["success", "error"].map(outcome => [detach, outcome] as const)))(
    "cancels the accepted pair before its first poll settles (detach=%s, late poll=%s)", async (detach, outcome) => {
      await renderPanel(); await prepareStart();
      let finishPoll!: (value: ReturnType<typeof scanResponse>) => void;
      let rejectPoll!: (error: Error) => void;
      let finishCancel!: (value: ReturnType<typeof scanResponse>) => void;
      apiMock.pollLineScan.mockReturnValueOnce(new Promise((resolve, reject) => { finishPoll = resolve; rejectPoll = reject; }));
      apiMock.cancelLineScan.mockReturnValueOnce(new Promise(resolve => { finishCancel = resolve; }));
      await act(async () => { click("Start read-only scan"); await Promise.resolve(); });
      await act(async () => click("Cancel scan"));
      expect(apiMock.cancelLineScan).toHaveBeenCalledExactlyOnceWith({ sessionId: 4, serverIncarnation: "scan-test", transport: "udp" });
      if (detach) {
        await act(async () => { root!.unmount(); root = undefined; }); host!.remove();
        apiMock.pollLineScan.mockResolvedValueOnce({ ...scanResponse("running", [{ address: "1.1.7", outcome: { kind: "occupiedBusy" } }]), serverIncarnation: "successor" });
        await renderPanel();
      }
      await act(async () => {
        if (outcome === "success") finishPoll(scanResponse("running", [{ address: "1.1.5", outcome: { kind: "occupiedBusy" } }]));
        else rejectPoll(new Error("retired first poll"));
        await Promise.resolve();
      });
      expect(host!.textContent).not.toContain("1.1.5");
      expect(host!.textContent).not.toContain("retired first poll");
      if (!detach) expect([...host!.querySelectorAll<HTMLButtonElement>("button")].find(button => button.textContent === "Cancel scan")!.disabled).toBe(true);
      await act(async () => { finishCancel(scanResponse("cancelled", [])); await Promise.resolve(); });
      if (detach) expect(host!.textContent).toContain("1.1.7");
      else expect(host!.textContent).toContain("Cancelled");
      expect(apiMock.cancelLineScan).toHaveBeenCalledTimes(1);
    },
  );
  it.each([400, 404, 405])("keeps definite Start refusal %s editable and retries only on an explicit new choice", async status => {
    await renderPanel(); await prepareStart();
    await act(async () => { chooseTransport("tcp"); });
    await act(async () => { click("Estimate cost"); await Promise.resolve(); });
    apiMock.startLineScan.mockRejectedValueOnce(Object.assign(new Error("definite refusal"), { status }));
    await act(async () => { click("Start read-only scan"); await Promise.resolve(); });
    expect(host!.querySelector<HTMLSelectElement>(".line-scan-config select")!.disabled).toBe(false);
    expect(host!.querySelector<HTMLInputElement>('.line-scan-config input[placeholder="192.0.2.10:3671"]')!.disabled).toBe(false);
    expect(apiMock.startLineScan).toHaveBeenCalledTimes(1);
    expect(apiMock.cancelLineScan).not.toHaveBeenCalled();
    await act(async () => { chooseTransport("udp"); });
    expect(host!.querySelector<HTMLButtonElement>(".line-scan-start")!.disabled).toBe(true);
    await act(async () => { click("Estimate cost"); await Promise.resolve(); });
    apiMock.pollLineScan.mockResolvedValueOnce(scanResponse("running", []));
    await act(async () => { click("Start read-only scan"); await Promise.resolve(); });
    expect(apiMock.startLineScan).toHaveBeenCalledTimes(2);
    expect(apiMock.startLineScan.mock.calls.map(call => call[1])).toEqual(["tcp", "udp"]);
  });
  it.each([
    new Error("lost Start reply"), new Error("Malformed line scan identity."),
    Object.assign(new Error("server failure"), { status: 500 }), Object.assign(new Error("connect uncertain"), { status: 502 }),
  ])("does not turn ambiguous Start failure %s into reconnect authority", async reason => {
    await renderPanel(); await prepareStart();
    apiMock.startLineScan.mockRejectedValueOnce(reason);
    await act(async () => { click("Start read-only scan"); await Promise.resolve(); });
    expect(host!.querySelector<HTMLButtonElement>(".line-scan-start")!.disabled).toBe(true);
    expect(host!.querySelector<HTMLSelectElement>(".line-scan-config select")!.disabled).toBe(true);
    expect(apiMock.startLineScan).toHaveBeenCalledTimes(1);
    expect(apiMock.cancelLineScan).not.toHaveBeenCalled();
  });
  it.each(["success", "error"])("fences a delayed comparison %s after a same-id successor", async outcome => {
    let finish!: (value: { unexpected: string[]; missing: string[]; excludedInProject: string[] }) => void;
    let reject!: (error: Error) => void;
    apiMock.pollLineScan.mockResolvedValueOnce(scanResponse("completed", []));
    apiMock.compareLineScan.mockReturnValueOnce(new Promise((resolve, fail) => { finish = resolve; reject = fail; }));
    await renderPanel(); await prepareStart();
    apiMock.startLineScan.mockResolvedValueOnce({ sessionId: 4, serverIncarnation: "successor", transport: "udp", skippedFrames: null, estimate: awaitableEstimate() });
    apiMock.pollLineScan.mockResolvedValueOnce({ ...scanResponse("completed", []), serverIncarnation: "successor" });
    apiMock.compareLineScan.mockResolvedValueOnce({ unexpected: ["1.1.7"], missing: [], excludedInProject: [] });
    await act(async () => { click("Start read-only scan"); await Promise.resolve(); });
    await act(async () => {
      if (outcome === "success") finish({ unexpected: ["1.1.9"], missing: [], excludedInProject: [] });
      else reject(new Error("retired comparison"));
      await Promise.resolve();
    });
    expect(host!.textContent).toContain("1.1.7");
    expect(host!.textContent).not.toContain("1.1.9");
    expect(host!.textContent).not.toContain("retired comparison");
    expect(apiMock.compareLineScan).toHaveBeenCalledTimes(2);
  });
  it.each(["success", "error"])("refuses a successor Start until deliberate reconciliation %s settles", async outcome => {
    let finish!: (value: { can_undo: boolean }) => void;
    let reject!: (error: Error) => void;
    apiMock.pollLineScan.mockResolvedValueOnce(scanResponse("completed", []));
    const onTreeUpdate = vi.fn();
    await renderPanel(onTreeUpdate); await prepareStart();
    await act(async () => host!.querySelector<HTMLInputElement>('.line-scan-reconciliation input[type="checkbox"]')!.click());
    apiMock.reconcileLineScan.mockReturnValueOnce(new Promise((resolve, fail) => { finish = resolve; reject = fail; }));
    await act(async () => click("Apply selected changes"));
    const start = host!.querySelector<HTMLButtonElement>(".line-scan-start")!;
    expect(start.disabled).toBe(true);
    await act(async () => start.click());
    expect(apiMock.startLineScan).not.toHaveBeenCalled();
    await act(async () => {
      if (outcome === "success") finish({ can_undo: true }); else reject(new Error("reconciliation refused"));
      await Promise.resolve();
    });
    expect(start.disabled).toBe(false);
    if (outcome === "success") expect(onTreeUpdate).toHaveBeenCalledWith({ can_undo: true });
    else expect(onTreeUpdate).not.toHaveBeenCalled();
    apiMock.startLineScan.mockResolvedValueOnce({ sessionId: 5, serverIncarnation: "scan-test", transport: "udp", skippedFrames: null, estimate: awaitableEstimate() });
    apiMock.pollLineScan.mockResolvedValueOnce({ ...scanResponse("completed", []), sessionId: 5 });
    await act(async () => { click("Start read-only scan"); await Promise.resolve(); });
    await act(async () => host!.querySelector<HTMLInputElement>('.line-scan-reconciliation input[type="checkbox"]')!.click());
    expect([...host!.querySelectorAll<HTMLButtonElement>("button")].find(button => button.textContent === "Apply selected changes")!.disabled).toBe(false);
  });

  async function prepareStart() {
    await act(async () => setInput('.line-scan-config input[placeholder="192.0.2.10:3671"]', "192.0.2.10:3671"));
    await act(async () => { click("Estimate cost"); await Promise.resolve(); });
  }
  function chooseTransport(value: "udp" | "tcp") {
    const select = host!.querySelector<HTMLSelectElement>(".line-scan-config select")!;
    Object.getOwnPropertyDescriptor(window.HTMLSelectElement.prototype,"value")!.set!.call(select,value);
    select.dispatchEvent(new Event("change",{bubbles:true}));
  }
  it("keeps a scan-local UDP default despite TCP monitoring preference and invalidates estimates on selection", async () => {
    setSetting("preferredIpTunnelTransport","tcp");
    await renderPanel(); await prepareStart();
    expect(host!.querySelector<HTMLSelectElement>("select")!.value).toBe("udp");
    await act(async () => chooseTransport("tcp"));
    expect(host!.querySelector<HTMLButtonElement>(".line-scan-start")!.disabled).toBe(true);
    expect(apiMock.startLineScan).not.toHaveBeenCalled();
    expect(getSetting("preferredIpTunnelTransport")).toBe("tcp");
    apiMock.startLineScan.mockResolvedValueOnce({ sessionId:4,serverIncarnation:"scan-test",transport:"tcp",skippedFrames:0,estimate:awaitableEstimate() });
    apiMock.pollLineScan.mockResolvedValueOnce({ ...scanResponse("running",[]),transport:"tcp",skippedFrames:2 });
    await act(async () => { click("Estimate cost"); await Promise.resolve(); });
    await act(async () => { click("Start read-only scan"); await Promise.resolve(); });
    expect(apiMock.startLineScan).toHaveBeenCalledWith(expect.objectContaining({ gateway:"192.0.2.10:3671" }),"tcp");
    expect(host!.textContent).toContain("Observed scan transport: TCP");
    expect(host!.textContent).toContain("2 stream frames skipped");
    expect(host!.querySelector<HTMLSelectElement>("select")!.disabled).toBe(true);
  });
  it.each([false,true])("retains pending explicit Cancel across detach=%s and uses exactly the returned pair", async detach => {
    let resolveStart!: (value: unknown) => void;
    apiMock.startLineScan.mockReturnValueOnce(new Promise(resolve => { resolveStart=resolve; }));
    apiMock.cancelLineScan.mockResolvedValueOnce({ ...scanResponse("cancelled", []), sessionId: 41, serverIncarnation: "old-process", transport: "tcp", skippedFrames: 0 });
    await renderPanel(); await prepareStart();
    await act(async () => chooseTransport("tcp"));
    await act(async () => { click("Estimate cost"); await Promise.resolve(); });
    await act(async () => { click("Start read-only scan"); });
    await act(async () => { click("Cancel scan"); });
    expect(apiMock.cancelLineScan).not.toHaveBeenCalled();
    expect(host!.textContent).not.toContain("Cancelled");
    if (detach) await act(async () => { root!.unmount(); root=undefined; });
    await act(async () => { resolveStart({ sessionId:41,serverIncarnation:"old-process",transport:"tcp",skippedFrames:0,estimate:awaitableEstimate() }); await Promise.resolve(); });
    expect(apiMock.cancelLineScan).toHaveBeenCalledExactlyOnceWith({ sessionId:41,serverIncarnation:"old-process",transport:"tcp" });
    expect(apiMock.pollLineScan).toHaveBeenCalledTimes(1);
  });
  it("does not silently cancel an accepted server-owned scan on navigation", async () => {
    apiMock.pollLineScan.mockResolvedValueOnce(scanResponse("running",[]));
    await renderPanel();
    await act(async () => { root!.unmount(); root=undefined; });
    expect(apiMock.cancelLineScan).not.toHaveBeenCalled();
  });
  it.each(["success","error"])("fences an old first-poll %s after a start accepts a new same-id process", async outcome => {
    let finish!: (value: ReturnType<typeof scanResponse>) => void;
    let reject!: (reason: Error) => void;
    apiMock.pollLineScan.mockReturnValueOnce(new Promise((resolve,reject_) => { finish=resolve; reject=reject_; }));
    await renderPanel(); await prepareStart();
    apiMock.startLineScan.mockResolvedValueOnce({ sessionId:4,serverIncarnation:"new-process",transport:"udp",skippedFrames:null,estimate:awaitableEstimate() });
    apiMock.pollLineScan.mockResolvedValueOnce({ ...scanResponse("running",[{address:"1.1.7",outcome:{kind:"occupiedBusy"}}]),serverIncarnation:"new-process" });
    await act(async () => { click("Start read-only scan"); await Promise.resolve(); });
    await act(async () => { if (outcome==="success") finish(scanResponse("completed",[{address:"1.1.5",outcome:{kind:"occupiedBusy"}}])); else reject(new Error("obsolete failure")); await Promise.resolve(); });
    expect(host!.textContent).toContain("1.1.7"); expect(host!.textContent).not.toContain("1.1.5");
    expect(host!.textContent).not.toContain("obsolete failure");
    expect(apiMock.compareLineScan).not.toHaveBeenCalled();
  });
  it("locks actions after malformed reattachment instead of guessing legacy UDP", async () => {
    apiMock.pollLineScan.mockRejectedValueOnce(new Error("Malformed line scan transport or skipped-frame count."));
    await renderPanel(); await prepareStart();
    expect(host!.querySelector<HTMLButtonElement>(".line-scan-start")!.disabled).toBe(true);
    expect(host!.querySelector<HTMLSelectElement>("select")!.disabled).toBe(true);
    expect(apiMock.startLineScan).not.toHaveBeenCalled(); expect(apiMock.cancelLineScan).not.toHaveBeenCalled();
  });
  it("does not update a remounted successor with a detached pending cancel error", async () => {
    let rejectCancel!: (error:Error)=>void;
    apiMock.pollLineScan.mockResolvedValueOnce(scanResponse("running",[]));
    apiMock.cancelLineScan.mockReturnValueOnce(new Promise((_,reject) => { rejectCancel=reject; }));
    await renderPanel(); await act(async () => click("Cancel scan"));
    await act(async () => { root!.unmount(); root=undefined; }); host!.remove();
    apiMock.pollLineScan.mockResolvedValueOnce({ ...scanResponse("running",[{address:"1.1.7",outcome:{kind:"occupiedBusy"}}]),serverIncarnation:"successor" });
    await renderPanel();
    await act(async () => { rejectCancel(new Error("obsolete cancel")); await Promise.resolve(); });
    expect(host!.textContent).toContain("1.1.7"); expect(host!.textContent).not.toContain("obsolete cancel");
    expect(apiMock.cancelLineScan).toHaveBeenCalledTimes(1);
  });
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
    expect(apiMock.cancelLineScan).toHaveBeenCalledWith({ sessionId: 4, serverIncarnation: "scan-test", transport: "udp" });
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

    expect(apiMock.compareLineScan).toHaveBeenCalledWith({ sessionId: 4, serverIncarnation: "scan-test", transport: "udp" });
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
    // Creating devices only is the routine reconciliation; selecting a missing device turns the
    // same button into a deletion, so it must look destructive for exactly as long as it is one.
    expect(apply.classList.contains("primary-action")).toBe(true);
    expect(apply.classList.contains("danger-action")).toBe(false);
    await act(async () => choices[1].click());
    expect(apply.classList.contains("danger-action")).toBe(true);
    expect(apply.classList.contains("primary-action")).toBe(false);
    await act(async () => choices[1].click());
    expect(apply.classList.contains("primary-action")).toBe(true);
    expect(apply.classList.contains("danger-action")).toBe(false);
    await act(async () => {
      apply.click();
      await Promise.resolve();
      await Promise.resolve();
    });
    expect(apiMock.reconcileLineScan).toHaveBeenCalledWith({ sessionId: 4, serverIncarnation: "scan-test", transport: "udp" }, ["1.1.4"], []);
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

tableColumnSmoke([["line-scan", 2]], async () => {
  apiMock.pollLineScan.mockResolvedValue(scanResponse("completed", [{ address: "1.1.1", outcome: { kind: "vacant" } }]));
  await renderPanel(); return { host: host! };
});
