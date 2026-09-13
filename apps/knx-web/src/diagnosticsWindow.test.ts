/** Tests companion-window addressing and the four outcomes of trying to open one. */
// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

const tauriMock = vi.hoisted(() => ({
  getByLabel: vi.fn(),
  constructed: [] as string[],
  once: vi.fn(),
  setFocus: vi.fn(),
}));

vi.mock("@tauri-apps/api/webviewWindow", () => ({
  WebviewWindow: class {
    static getByLabel = tauriMock.getByLabel;
    constructor(label: string) {
      tauriMock.constructed.push(label);
    }
    once = tauriMock.once;
    setFocus = tauriMock.setFocus;
  },
}));

import {
  canFocusMainWindow,
  companionUrl,
  focusMainWindow,
  isCompanionView,
  openCompanionWindow,
  resetCompanionWindowRef,
} from "./diagnosticsWindow";

function setTauri(present: boolean) {
  if (present) (window as unknown as Record<string, unknown>).__TAURI__ = {};
  else delete (window as unknown as Record<string, unknown>).__TAURI__;
}

beforeEach(() => {
  resetCompanionWindowRef();
  setTauri(false);
  tauriMock.constructed.length = 0;
  // `once('tauri://created', cb)` resolves the creation promise; the error
  // branch is exercised by its own test below.
  tauriMock.once.mockImplementation((event: string, handler: () => void) => {
    if (event === "tauri://created") handler();
    return Promise.resolve(() => {});
  });
});

afterEach(() => {
  vi.restoreAllMocks();
  vi.clearAllMocks();
  setTauri(false);
});

describe("isCompanionView", () => {
  it("recognises the companion query", () => {
    expect(isCompanionView("?view=diagnostics")).toBe(true);
    expect(isCompanionView("?foo=1&view=diagnostics")).toBe(true);
  });

  it("treats everything else as the editing workspace", () => {
    expect(isCompanionView("")).toBe(false);
    expect(isCompanionView("?view=editor")).toBe(false);
    expect(isCompanionView("?view=diagnostics2")).toBe(false);
    // A hash is not a query. Getting this wrong would mean a bookmark with
    // `#view=diagnostics` silently opening a read-only shell.
    expect(isCompanionView("#view=diagnostics")).toBe(false);
  });
});

describe("companionUrl", () => {
  it("keeps the origin and path, and carries nothing else across", () => {
    expect(companionUrl("http://127.0.0.1:4711/?foo=1#somewhere")).toBe(
      "http://127.0.0.1:4711/?view=diagnostics",
    );
  });

  it("works from the companion's own URL, so reopening is idempotent", () => {
    expect(companionUrl("http://localhost:1420/?view=diagnostics")).toBe(
      "http://localhost:1420/?view=diagnostics",
    );
  });
});

describe("openCompanionWindow in a browser", () => {
  it("opens a named window at the companion URL", async () => {
    const fake = { focus: vi.fn(), closed: false } as unknown as Window;
    const open = vi.spyOn(window, "open").mockReturnValue(fake);

    expect(await openCompanionWindow("http://localhost:1420/?x=1")).toBe("opened");
    expect(open).toHaveBeenCalledWith("http://localhost:1420/?view=diagnostics", "knxbench-diagnostics");
  });

  // codex-goal.md names reopening and blocked popups as cases to handle,
  // not as cases to hope do not happen.
  it("reports a focus rather than a second window when one is already open", async () => {
    const fake = { focus: vi.fn(), closed: false } as unknown as Window;
    vi.spyOn(window, "open").mockReturnValue(fake);

    expect(await openCompanionWindow("http://localhost:1420/")).toBe("opened");
    expect(await openCompanionWindow("http://localhost:1420/")).toBe("focused");
  });

  it("opens again after the companion was closed", async () => {
    const closedOne = { focus: vi.fn(), closed: true } as unknown as Window;
    const freshOne = { focus: vi.fn(), closed: false } as unknown as Window;
    const open = vi.spyOn(window, "open").mockReturnValueOnce(closedOne).mockReturnValueOnce(freshOne);

    expect(await openCompanionWindow("http://localhost:1420/")).toBe("opened");
    expect(await openCompanionWindow("http://localhost:1420/")).toBe("opened");
    expect(open).toHaveBeenCalledTimes(2);
  });

  it("reports a blocked popup as blocked, not as a failure", async () => {
    vi.spyOn(window, "open").mockReturnValue(null);
    expect(await openCompanionWindow("http://localhost:1420/")).toBe("blocked");
  });

  it("reports a thrown refusal as a failure instead of propagating it", async () => {
    vi.spyOn(window, "open").mockImplementation(() => {
      throw new Error("sandboxed");
    });
    expect(await openCompanionWindow("http://localhost:1420/")).toBe("failed");
  });
});

describe("openCompanionWindow under Tauri", () => {
  it("creates a labelled webview window when none exists", async () => {
    setTauri(true);
    tauriMock.getByLabel.mockResolvedValue(null);

    expect(await openCompanionWindow("http://127.0.0.1:4711/")).toBe("opened");
    expect(tauriMock.constructed).toEqual(["diagnostics"]);
  });

  it("focuses the existing webview window instead of creating a second one", async () => {
    setTauri(true);
    tauriMock.getByLabel.mockResolvedValue({ setFocus: tauriMock.setFocus });

    expect(await openCompanionWindow("http://127.0.0.1:4711/")).toBe("focused");
    expect(tauriMock.constructed).toEqual([]);
    expect(tauriMock.setFocus).toHaveBeenCalled();
  });

  // A missing capability grant surfaces as `tauri://error`, not as a
  // rejected promise. Resolving on whichever event arrives is what turns
  // that into a reported failure rather than a window that never appears
  // and a button that seemed to work.
  it("reports a refused creation as a failure", async () => {
    setTauri(true);
    tauriMock.getByLabel.mockResolvedValue(null);
    tauriMock.once.mockImplementation((event: string, handler: () => void) => {
      if (event === "tauri://error") handler();
      return Promise.resolve(() => {});
    });

    expect(await openCompanionWindow("http://127.0.0.1:4711/")).toBe("failed");
  });
});

describe("returning to the main window", () => {
  function setOpener(value: unknown) {
    Object.defineProperty(window, "opener", { value, configurable: true, writable: true });
  }

  it("focuses the opener in a browser", async () => {
    const opener = { focus: vi.fn(), closed: false };
    setOpener(opener);
    expect(await canFocusMainWindow()).toBe(true);
    expect(await focusMainWindow()).toBe(true);
    expect(opener.focus).toHaveBeenCalled();
  });

  it("reports that there is nothing to return to when the companion stands alone", async () => {
    setOpener(null);
    expect(await canFocusMainWindow()).toBe(false);
    expect(await focusMainWindow()).toBe(false);
  });

  it("focuses the main webview window under Tauri", async () => {
    setTauri(true);
    tauriMock.getByLabel.mockResolvedValue({ setFocus: tauriMock.setFocus });
    expect(await canFocusMainWindow()).toBe(true);
    expect(await focusMainWindow()).toBe(true);
    expect(tauriMock.getByLabel).toHaveBeenCalledWith("main");
  });
});
