// @vitest-environment happy-dom
//
// Standalone tests for the compose/send form's own resolve-then-send state
// machine (design spec §6) — deliberately separate from
// `BusMonitorPanel.test.tsx`, which already covers session lifecycle,
// polling and the row click that seeds this component's props; none of
// that needs re-mocking here, this file only ever renders
// `<BusComposeForm>` directly with a fixed `destination`/`resolution`.
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { BusWriteResponse } from "./api";

const apiMock = vi.hoisted(() => ({
  writeBusValue: vi.fn(),
}));

vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}));

import BusComposeForm, { type ComposeResolution } from "./BusComposeForm";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let host: HTMLDivElement | undefined;

async function renderForm(destination: string, resolution: ComposeResolution, projectOpen = true) {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(<BusComposeForm destination={destination} resolution={resolution} projectOpen={projectOpen} />);
  });
  return root;
}

function setInputValue(selector: string, value: string) {
  const input = host!.querySelector<HTMLInputElement>(selector)!;
  const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!;
  setter.call(input, value);
  input.dispatchEvent(new Event("input", { bubbles: true }));
}

async function clickSend() {
  const button = Array.from(host!.querySelectorAll("button")).find((b) => b.textContent === "Send")!;
  await act(async () => {
    button.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    await Promise.resolve();
    await Promise.resolve();
  });
}

beforeEach(() => {
  apiMock.writeBusValue.mockResolvedValue({ encodedPayload: "0x01", service: "GroupValueWrite" } satisfies BusWriteResponse);
});

afterEach(() => {
  host?.remove();
  host = undefined;
  vi.clearAllMocks();
});

describe("BusComposeForm", () => {
  it("sends using the prefilled single-DPT resolution when the DPT field is left as-is", async () => {
    await renderForm("1/2/3", { kind: "single", dpt: "DPST-1-1" });
    await act(async () => {
      setInputValue(".bus-compose-value", "on");
    });

    await clickSend();

    expect(apiMock.writeBusValue).toHaveBeenCalledWith("1/2/3", "DPST-1-1", "on");
  });

  it("an explicit DPT typed in the field wins over the prefilled resolution", async () => {
    await renderForm("1/2/3", { kind: "single", dpt: "DPST-1-1" });
    await act(async () => {
      setInputValue(".bus-compose-dpt", "DPST-5-1");
      setInputValue(".bus-compose-value", "50");
    });

    await clickSend();

    expect(apiMock.writeBusValue).toHaveBeenCalledWith("1/2/3", "DPST-5-1", "50");
  });

  it("blocks the send and shows the verbatim no-DPT message for a None resolution — writeBusValue is never called", async () => {
    await renderForm("1/2/3", { kind: "none" });
    await act(async () => {
      setInputValue(".bus-compose-value", "on");
    });

    await clickSend();

    expect(apiMock.writeBusValue).not.toHaveBeenCalled();
    expect(host!.querySelector(".bus-compose-error")!.textContent).toBe(
      "No DPT resolved for this group address — enter one explicitly.",
    );
  });

  it("blocks the send and shows the verbatim conflict message with names — writeBusValue is never called", async () => {
    await renderForm("1/2/3", { kind: "conflict", names: "DPST-1-1, DPST-5-1" });
    await act(async () => {
      setInputValue(".bus-compose-value", "on");
    });

    await clickSend();

    expect(apiMock.writeBusValue).not.toHaveBeenCalled();
    expect(host!.querySelector(".bus-compose-error")!.textContent).toBe(
      "Conflicting DPTs for this group address: DPST-1-1, DPST-5-1 — enter one explicitly.",
    );
  });

  it("an explicit DPT rescues a Conflict resolution — the request is sent after all", async () => {
    await renderForm("1/2/3", { kind: "conflict", names: "DPST-1-1, DPST-5-1" });
    await act(async () => {
      setInputValue(".bus-compose-dpt", "DPST-1-1");
      setInputValue(".bus-compose-value", "on");
    });

    await clickSend();

    expect(apiMock.writeBusValue).toHaveBeenCalledWith("1/2/3", "DPST-1-1", "on");
  });

  it("an unknown resolution (hand-typed or edited destination) defers to the server instead of guessing", async () => {
    await renderForm("", { kind: "unknown" });
    await act(async () => {
      setInputValue(".bus-compose-destination", "9/9/9");
      setInputValue(".bus-compose-value", "on");
    });

    await clickSend();

    expect(apiMock.writeBusValue).toHaveBeenCalledWith("9/9/9", null, "on");
  });

  it("editing the destination after a None prefill drops the stale resolution, so the next send is not pre-blocked", async () => {
    await renderForm("1/2/3", { kind: "none" });
    await act(async () => {
      setInputValue(".bus-compose-destination", "9/9/9");
      setInputValue(".bus-compose-value", "on");
    });

    await clickSend();

    // The `"none"` resolution described `1/2/3`, not `9/9/9` — once the
    // user retypes the destination, this component no longer has any
    // basis to reject client-side and defers to the server instead.
    expect(apiMock.writeBusValue).toHaveBeenCalledWith("9/9/9", null, "on");
  });

  it("shows the server's echoed encodedPayload on success", async () => {
    apiMock.writeBusValue.mockResolvedValue({ encodedPayload: "0x01 (6-bit)", service: "GroupValueWrite" });
    await renderForm("1/2/3", { kind: "single", dpt: "DPST-1-1" });
    await act(async () => {
      setInputValue(".bus-compose-value", "on");
    });

    await clickSend();

    expect(host!.querySelector(".bus-compose-sent")!.textContent).toBe("Sent GroupValueWrite: 0x01 (6-bit)");
  });

  it("shows a server error inline on the form, not only as a toast", async () => {
    const error = new Error("gateway rejected the write") as Error & { status: number };
    error.status = 502;
    apiMock.writeBusValue.mockRejectedValue(error);
    await renderForm("1/2/3", { kind: "single", dpt: "DPST-1-1" });
    await act(async () => {
      setInputValue(".bus-compose-value", "on");
    });

    await clickSend();

    expect(host!.querySelector(".bus-compose-error")!.textContent).toBe("gateway rejected the write");
    expect(host!.querySelector(".bus-compose-sent")).toBeNull();
  });

  it("shows a hint that no DPT resolves automatically when no project is open", async () => {
    await renderForm("", { kind: "unknown" }, false);
    expect(host!.querySelector(".bus-compose-hint")).not.toBeNull();
  });

  it("shows no hint when a project is open", async () => {
    await renderForm("", { kind: "unknown" }, true);
    expect(host!.querySelector(".bus-compose-hint")).toBeNull();
  });
});
