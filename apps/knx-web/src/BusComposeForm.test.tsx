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
  defaultDptInputFormat: (dpt: string) =>
    dpt === "DPST-1-1" ? "canonical" : dpt === "DPST-5-1" ? "decimal" : "canonical",
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
}));

import BusComposeForm, { type ComposeResolution } from "./BusComposeForm";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

// A disabled control is out of the tab order, so the prose that explains
// why it is disabled is reachable only if the control points at it. This
// resolves `aria-describedby` the way an assistive technology would —
// through the ids, into the live document — rather than asserting that
// the attribute merely exists.
function describedTextOf(selector: string): string {
  const control = host!.querySelector(selector)!;
  const ids = (control.getAttribute("aria-describedby") ?? "").split(" ").filter(Boolean);
  expect(ids.length).toBeGreaterThan(0);
  return ids
    .map((id) => {
      const target = host!.querySelector(`#${id}`);
      expect(target).not.toBeNull();
      return target!.textContent ?? "";
    })
    .join(" ");
}

let host: HTMLDivElement | undefined;

async function renderForm(
  destination: string,
  resolution: ComposeResolution,
  projectOpen = true,
  sessionClosed = false,
  contextStale = false,
) {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(
      <BusComposeForm
        destination={destination}
        resolution={resolution}
        projectOpen={projectOpen}
        sessionClosed={sessionClosed}
        contextStale={contextStale}
      />,
    );
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
  apiMock.writeBusValue.mockResolvedValue({
    encodedPayload: "0x01",
    service: "GroupValueWrite",
    decodedEcho: { kind: "value", dpt: "DPST-1-1", text: "on" },
  } satisfies BusWriteResponse);
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

    expect(apiMock.writeBusValue).toHaveBeenCalledWith(
      "1/2/3",
      "DPST-1-1",
      "on",
      "canonical",
    );
  });

  it("an explicit DPT typed in the field wins over the prefilled resolution", async () => {
    await renderForm("1/2/3", { kind: "single", dpt: "DPST-1-1" });
    await act(async () => {
      setInputValue(".bus-compose-dpt", "DPST-5-1");
      setInputValue(".bus-compose-value", "50");
    });

    await clickSend();

    expect(apiMock.writeBusValue).toHaveBeenCalledWith("1/2/3", "DPST-5-1", "50", "decimal");
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

    expect(apiMock.writeBusValue).toHaveBeenCalledWith(
      "1/2/3",
      "DPST-1-1",
      "on",
      "canonical",
    );
  });

  it("an unknown resolution (hand-typed or edited destination) defers to the server instead of guessing", async () => {
    await renderForm("", { kind: "unknown" });
    await act(async () => {
      setInputValue(".bus-compose-destination", "9/9/9");
      setInputValue(".bus-compose-value", "on");
    });

    await clickSend();

    expect(apiMock.writeBusValue).toHaveBeenCalledWith("9/9/9", null, "on", null);
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
    expect(apiMock.writeBusValue).toHaveBeenCalledWith("9/9/9", null, "on", null);
  });

  it("shows the server's echoed encodedPayload on success", async () => {
    apiMock.writeBusValue.mockResolvedValue({
      encodedPayload: "0x01 (6-bit)",
      service: "GroupValueWrite",
      decodedEcho: { kind: "value", dpt: "DPST-1-1", text: "on" },
    });
    await renderForm("1/2/3", { kind: "single", dpt: "DPST-1-1" });
    await act(async () => {
      setInputValue(".bus-compose-value", "on");
    });

    await clickSend();

    expect(host!.querySelector(".bus-compose-sent")!.textContent).toBe(
      "Sent GroupValueWrite: 0x01 (6-bit) — Decoded: on",
    );
  });

  it("shows the decoded echo of the bytes actually sent, not a verbatim copy of the typed value", async () => {
    // A DPT-9 (float) write: what went on the wire decodes back to a
    // different-looking string than what was typed ("21" in, "21°C" out,
    // per this DPT's own unit-rendering rule) — the surest sign this is
    // reading `decodedEcho`, not silently echoing the form's own input.
    apiMock.writeBusValue.mockResolvedValue({
      encodedPayload: "0c:1a",
      service: "GroupValueWrite",
      decodedEcho: { kind: "value", dpt: "DPST-9-1", text: "21 °C" },
    });
    await renderForm("1/2/3", { kind: "single", dpt: "DPST-9-1" });
    await act(async () => {
      setInputValue(".bus-compose-value", "21");
    });

    await clickSend();

    expect(host!.querySelector(".bus-compose-sent")!.textContent).toBe(
      "Sent GroupValueWrite: 0c:1a — Decoded: 21 °C",
    );
  });

  it("shows a decode-failure echo as text, alongside the encoded payload, without the send itself failing", async () => {
    apiMock.writeBusValue.mockResolvedValue({
      encodedPayload: "0x01",
      service: "GroupValueWrite",
      decodedEcho: { kind: "error", text: "an encode/decode mismatch", error: "an encode/decode mismatch" },
    });
    await renderForm("1/2/3", { kind: "single", dpt: "DPST-1-1" });
    await act(async () => {
      setInputValue(".bus-compose-value", "on");
    });

    await clickSend();

    expect(host!.querySelector(".bus-compose-error")).toBeNull();
    expect(host!.querySelector(".bus-compose-sent")!.textContent).toBe(
      "Sent GroupValueWrite: 0x01 — Decoded: an encode/decode mismatch",
    );
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

  it("disables every field and explains why for a closed session, and Send issues no request", async () => {
    await renderForm("1/2/3", { kind: "single", dpt: "DPST-1-1" }, true, true);
    await act(async () => {
      setInputValue(".bus-compose-value", "on");
    });

    expect(host!.querySelector<HTMLInputElement>(".bus-compose-destination")!.disabled).toBe(true);
    expect(host!.querySelector<HTMLInputElement>(".bus-compose-dpt")!.disabled).toBe(true);
    expect(host!.querySelector<HTMLInputElement>(".bus-compose-value")!.disabled).toBe(true);
    const sendButton = Array.from(host!.querySelectorAll("button")).find((b) => b.textContent === "Send")!;
    expect(sendButton.disabled).toBe(true);
    expect(host!.querySelector(".bus-compose-closed-hint")!.textContent).toBe(
      "This session is closed — sending is disabled.",
    );
    for (const selector of [".bus-compose-destination", ".bus-compose-dpt", ".bus-compose-value"]) {
      expect(describedTextOf(selector)).toContain("This session is closed");
    }

    await clickSend();

    expect(apiMock.writeBusValue).not.toHaveBeenCalled();
  });

  // Task 4's stale lock, from this component's side. The severity that
  // justifies a second test right next to the closed-session one: a closed
  // session's send bounces off a `409` and nothing happens on the bus,
  // whereas a stale context's send *succeeds* — with the previous project's
  // DPT, on real hardware, and Project Undo cannot reach it.
  it("disables every field and explains why when the project context is stale, and Send issues no request", async () => {
    await renderForm("1/2/3", { kind: "single", dpt: "DPST-1-1" }, true, false, true);
    await act(async () => {
      setInputValue(".bus-compose-value", "on");
    });

    expect(host!.querySelector<HTMLInputElement>(".bus-compose-destination")!.disabled).toBe(true);
    expect(host!.querySelector<HTMLInputElement>(".bus-compose-dpt")!.disabled).toBe(true);
    expect(host!.querySelector<HTMLInputElement>(".bus-compose-value")!.disabled).toBe(true);
    const sendButton = Array.from(host!.querySelectorAll("button")).find((b) => b.textContent === "Send")!;
    expect(sendButton.disabled).toBe(true);
    const hint = host!.querySelector(".bus-compose-stale-hint")!;
    expect(hint.getAttribute("role")).toBe("alert");
    expect(hint.textContent).toContain("sending is locked");
    for (const selector of [".bus-compose-destination", ".bus-compose-dpt", ".bus-compose-value"]) {
      expect(describedTextOf(selector)).toContain("sending is locked");
    }

    await clickSend();

    expect(apiMock.writeBusValue).not.toHaveBeenCalled();
  });
});
