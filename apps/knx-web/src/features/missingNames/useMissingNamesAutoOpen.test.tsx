/** Tests the hook that opens the missing-name wizard once per project session (WP-51.3b/3c). */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { MissingNameScan } from "./client";
import { proposal } from "./missingNames.test-fixtures";
import { useMissingNamesAutoOpen, type AutoOpenGate, type MissingNamesAutoOpenOptions } from "./useMissingNamesAutoOpen";

const TREE = { server_incarnation: "srv", project_incarnation: 3 };
const KEY = "srv|3";
const withCandidate = { items: [proposal("device", 1, "Lamp")] } as unknown as MissingNameScan;
const empty = { items: [] } as unknown as MissingNameScan;

let root: Root;
let check: (tree: typeof TREE) => void;
let currentKey: string | null;
let gate: AutoOpenGate;
const open = vi.fn();
const scan = vi.fn<NonNullable<MissingNamesAutoOpenOptions["scan"]>>();

function Harness() {
  check = useMissingNamesAutoOpen({ currentKey: () => currentKey, gate: () => gate, open, scan });
  return null;
}

async function run(tree = TREE) {
  await act(async () => { check(tree); });
}

beforeEach(async () => {
  (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
  currentKey = KEY;
  gate = { overlayOpen: false, saving: false };
  scan.mockResolvedValue(withCandidate);
  root = createRoot(document.createElement("div"));
  await act(async () => root.render(<Harness />));
});

afterEach(async () => {
  await act(async () => root.unmount());
  vi.resetAllMocks();
  (globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = false;
});

describe("useMissingNamesAutoOpen", () => {
  it("asks for one item and opens when the project has a candidate", async () => {
    await run();
    expect(scan).toHaveBeenCalledWith({ offset: 0, limit: 1 });
    expect(open).toHaveBeenCalledTimes(1);
  });

  it("never opens a read-only project, even with a candidate (Q17)", async () => {
    scan.mockResolvedValue({ ...withCandidate, readOnly: true });
    await run();
    expect(scan).toHaveBeenCalledTimes(1);
    expect(open).not.toHaveBeenCalled();
  });

  it("does not open without a candidate", async () => {
    scan.mockResolvedValue(empty);
    await run();
    expect(open).not.toHaveBeenCalled();
  });

  it("opens only once for the same project session, and again for the next one", async () => {
    await run();
    await run();
    expect(open).toHaveBeenCalledTimes(1);
    currentKey = "srv|4";
    await run({ server_incarnation: "srv", project_incarnation: 4 });
    expect(open).toHaveBeenCalledTimes(2);
  });

  it("swallows a rejected and a throwing scan", async () => {
    scan.mockRejectedValueOnce(new Error("offline"));
    await run();
    scan.mockImplementationOnce(() => { throw new Error("sync"); });
    await run();
    expect(open).not.toHaveBeenCalled();
    // Neither failure consumed the session's one automatic open.
    await run();
    expect(open).toHaveBeenCalledTimes(1);
  });

  it("reads the gate when the scan answers, not when it started", async () => {
    let answer!: (scan: MissingNameScan) => void;
    scan.mockReturnValueOnce(new Promise((resolve) => { answer = resolve; }));
    await run();
    gate = { overlayOpen: true, saving: false };
    await act(async () => answer(withCandidate));
    expect(open).not.toHaveBeenCalled();
  });

  it("does not consume the session's automatic open when a modal blocks the answer", async () => {
    let answer!: (scan: MissingNameScan) => void;
    scan.mockReturnValueOnce(new Promise((resolve) => { answer = resolve; }));
    await run();
    gate = { overlayOpen: true, saving: false };
    await act(async () => answer(withCandidate));
    expect(open).not.toHaveBeenCalled();
    gate = { overlayOpen: false, saving: false };
    await run();
    expect(open).toHaveBeenCalledTimes(1);
  });

  it("does not open while saving", async () => {
    gate = { overlayOpen: false, saving: true };
    await run();
    expect(open).not.toHaveBeenCalled();
  });

  it("drops an answer for a project that is no longer shown", async () => {
    let answer!: (scan: MissingNameScan) => void;
    scan.mockReturnValueOnce(new Promise((resolve) => { answer = resolve; }));
    await run();
    currentKey = "srv|4";
    await act(async () => answer(withCandidate));
    expect(open).not.toHaveBeenCalled();
  });

  it("drops an answer that arrives after unmount", async () => {
    let answer!: (scan: MissingNameScan) => void;
    scan.mockReturnValueOnce(new Promise((resolve) => { answer = resolve; }));
    await run();
    await act(async () => root.unmount());
    await act(async () => answer(withCandidate));
    expect(open).not.toHaveBeenCalled();
    root = createRoot(document.createElement("div"));
  });
});
