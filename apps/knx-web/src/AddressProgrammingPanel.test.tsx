/** Verifies the address-programming panel: consent first, the server's phrase, honest outcomes. */
// @vitest-environment happy-dom

import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";

const apiMock = vi.hoisted(() => ({
  addressProgrammingAvailability: vi.fn(),
  addressProgrammingPhrase: vi.fn(),
  startAddressProgramming: vi.fn(),
  pollAddressProgramming: vi.fn(),
  stopAddressProgramming: vi.fn(),
  serverVersion: vi.fn(),
}));

vi.mock("./api", () => ({
  ...apiMock,
  errorMessage: (error: unknown) => (error instanceof Error ? error.message : String(error)),
  errorStatus: (error: unknown) =>
    error && typeof error === "object" && "status" in error
      ? (error as { status: number }).status
      : undefined,
}));

import AddressProgrammingPanel from "./AddressProgrammingPanel";
import { messages as en } from "./messages/en";
import type { AddressProgrammingStatusResponse } from "./api";
import { subscribeAchievementEvents, type AchievementEvent } from "./achievementEvents";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let host: HTMLDivElement | undefined;
let root: Root | undefined;

const PHRASE = {
  address: "1.1.30",
  confirmationPhrase: "I confirm individual-address programming to 1.1.30",
  defaultWaitSeconds: 120,
  maxWaitSeconds: 600,
};

function withStatus(status: number, message: string): Error {
  const error = new Error(message) as Error & { status: number };
  error.status = status;
  return error;
}

function response(
  status: AddressProgrammingStatusResponse["status"],
  events: AddressProgrammingStatusResponse["events"],
  nextSince: number,
): AddressProgrammingStatusResponse {
  return { programmingId: 4, address: "1.1.30", waitSeconds: 120, status, nextSince, events };
}

async function flush() {
  await act(async () => {
    for (let i = 0; i < 6; i += 1) await Promise.resolve();
  });
}

async function render() {
  host = document.createElement("div");
  document.body.appendChild(host);
  root = createRoot(host);
  await act(async () => {
    root!.render(<AddressProgrammingPanel project={null} />);
  });
  await flush();
}

function button(label: string, scope: ParentNode = host!): HTMLButtonElement {
  const found = [...scope.querySelectorAll("button")].find((b) => b.textContent === label);
  if (!found) throw new Error(`button not found: ${label}`);
  return found;
}

async function type(name: string, value: string) {
  const input = host!.querySelector<HTMLInputElement>(`input[name="${name}"]`)!;
  const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!;
  await act(async () => {
    setter.call(input, value);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  });
}

async function fill() {
  await type("address", "1.1.30");
  await type("gateway", "192.0.2.10:3671");
}

function consentDialog(): HTMLElement | null {
  return document.querySelector<HTMLElement>(".programming-consent-dialog");
}

async function confirm() {
  await act(async () => button("Program 1.1.30").click());
  await flush();
  await act(async () => button(en["programmingConsent.confirm"], consentDialog()!).click());
  await flush();
}

beforeEach(() => {
  vi.useFakeTimers();
  window.localStorage.clear();
  // Historical workflow tests model a hypothetical recovery-ready server;
  // production currently returns false from its pre-write recovery gate.
  apiMock.addressProgrammingAvailability.mockReset().mockResolvedValue({ startAvailable: true, reason: null });
  apiMock.pollAddressProgramming.mockReset().mockRejectedValue(withStatus(404, "none yet"));
  apiMock.addressProgrammingPhrase.mockReset().mockResolvedValue(PHRASE);
  apiMock.startAddressProgramming.mockReset().mockResolvedValue({ programmingId: 4 });
  apiMock.stopAddressProgramming.mockReset().mockResolvedValue({ stopping: true });
  apiMock.serverVersion.mockReset().mockResolvedValue({ version: "0.1.0-alpha.1" });
});

afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  host?.remove();
  host = undefined;
  root = undefined;
  vi.useRealTimers();
});

it("says what it writes, and that it is not a download", async () => {
  await render();
  expect(host!.textContent).toContain(en["addressProgramming.title"]);
  expect(host!.textContent).toContain("this is not a download of parameters");
  expect(host!.textContent).toContain("Nothing is written while waiting");
});

it("shows the backend recovery refusal and never requests consent or a phrase", async () => {
  apiMock.addressProgrammingAvailability.mockResolvedValueOnce({
    startAvailable: false,
    reason: "no verified durable pre-write backup",
  });
  await render();
  await fill();
  const start = button("Program 1.1.30");
  expect(start.disabled).toBe(true);
  expect(host!.textContent).toContain("no verified durable pre-write backup");
  start.click();
  await flush();
  expect(consentDialog()).toBeNull();
  expect(apiMock.addressProgrammingPhrase).not.toHaveBeenCalled();
  expect(apiMock.startAddressProgramming).not.toHaveBeenCalled();
});

it("can still observe and stop an already-held waiting session while new starts are blocked", async () => {
  apiMock.addressProgrammingAvailability.mockResolvedValueOnce({
    startAvailable: false,
    reason: "no verified durable pre-write backup",
  });
  apiMock.pollAddressProgramming.mockReset()
    .mockResolvedValueOnce(response({ state: "waiting", rounds: 1, inProgrammingMode: [] }, [], 0))
    .mockResolvedValueOnce(response({ state: "stopped", rounds: 1 }, [], 0));
  await render();
  expect(host!.querySelector(".address-programming-progress")?.textContent)
    .toContain(en["addressProgramming.pressButton"]);
  await act(async () => button(en["addressProgramming.stop"]).click());
  await flush();
  expect(apiMock.stopAddressProgramming).toHaveBeenCalledWith(4);
  expect(apiMock.addressProgrammingPhrase).not.toHaveBeenCalled();
  expect(apiMock.startAddressProgramming).not.toHaveBeenCalled();
});

it("fails closed when the availability read fails, then rechecks on explicit retry", async () => {
  apiMock.addressProgrammingAvailability.mockRejectedValueOnce(new Error("offline"));
  await render();
  await fill();
  expect(button("Program 1.1.30").disabled).toBe(true);
  expect(host!.querySelector('[role="alert"]')?.textContent).toContain("offline");
  await act(async () => button(en["addressProgramming.retryAvailability"]).click());
  await flush();
  expect(apiMock.addressProgrammingAvailability).toHaveBeenCalledTimes(2);
  expect(button("Program 1.1.30").disabled).toBe(false);
  expect(consentDialog()).toBeNull();
});

it("does not infer permission from a malformed availability response", async () => {
  apiMock.addressProgrammingAvailability.mockResolvedValueOnce({ reason: "missing gate result" });
  await render();
  await fill();
  expect(button("Program 1.1.30").disabled).toBe(true);
  expect(host!.querySelector('[role="alert"]')?.textContent)
    .toContain(en["addressProgramming.availabilityInvalid"]);
  expect(apiMock.addressProgrammingPhrase).not.toHaveBeenCalled();
});

it("rejects a contradictory ready flag with a recovery refusal reason", async () => {
  apiMock.addressProgrammingAvailability.mockResolvedValueOnce({
    startAvailable: true,
    reason: "durable backup still missing",
  });
  await render();
  await fill();
  expect(button("Program 1.1.30").disabled).toBe(true);
  expect(host!.querySelector('[role="alert"]')?.textContent)
    .toContain(en["addressProgramming.availabilityInvalid"]);
  expect(consentDialog()).toBeNull();
});

it("closes the start affordance when the server revokes recovery after an earlier available response", async () => {
  apiMock.startAddressProgramming.mockRejectedValueOnce(withStatus(412, "no verified durable pre-write backup"));
  await render();
  await fill();
  await confirm();
  expect(apiMock.startAddressProgramming).toHaveBeenCalledTimes(1);
  expect(button("Program 1.1.30").disabled).toBe(true);
  expect(host!.textContent).toContain("no verified durable pre-write backup");
  expect(consentDialog()).toBeNull();
});

it("asks for consent first, and Cancel sends nothing", async () => {
  await render();
  await fill();
  await act(async () => button("Program 1.1.30").click());
  await flush();
  const dialog = consentDialog();
  expect(dialog).not.toBeNull();
  expect(dialog!.textContent).toContain("1.1.30");
  await act(async () => button(en["programmingConsent.cancel"], dialog!).click());
  await flush();
  expect(apiMock.startAddressProgramming).not.toHaveBeenCalled();
});

it("starts with the server's own phrase for the typed address and the chosen wait", async () => {
  await render();
  await fill();
  await type("wait", "45");
  await confirm();
  expect(apiMock.addressProgrammingPhrase).toHaveBeenCalledWith("1.1.30");
  expect(apiMock.startAddressProgramming).toHaveBeenCalledTimes(1);
  expect(apiMock.startAddressProgramming).toHaveBeenCalledWith(
    "1.1.30",
    "192.0.2.10:3671",
    "I confirm individual-address programming to 1.1.30",
    45,
  );
});

it("refuses a wait out of range before asking anything", async () => {
  await render();
  await fill();
  await type("wait", "601");
  await act(async () => button("Program 1.1.30").click());
  await flush();
  expect(consentDialog()).toBeNull();
  expect(host!.querySelector("[role=alert]")!.textContent).toBe("The wait must be between 1 and 600 seconds.");
  expect(apiMock.startAddressProgramming).not.toHaveBeenCalled();
});

it("shows the server's refusal of an invalid address, and asks nothing", async () => {
  apiMock.addressProgrammingPhrase.mockReset().mockRejectedValue(withStatus(400, "1.1.220 is excluded"));
  await render();
  await type("address", "1.1.220");
  await type("gateway", "192.0.2.10:3671");
  await act(async () => button("Program 1.1.220").click());
  await flush();
  expect(consentDialog()).toBeNull();
  expect(host!.querySelector("[role=alert]")!.textContent).toBe("1.1.220 is excluded");
  expect(apiMock.startAddressProgramming).not.toHaveBeenCalled();
});

it("tells the person at the device to press, then to release all but one, then that it runs", async () => {
  await render();
  await fill();
  apiMock.pollAddressProgramming
    .mockReset()
    .mockResolvedValueOnce(
      response({ state: "waiting", rounds: 1, inProgrammingMode: [] }, [{ kind: "round", number: 1, inProgrammingMode: [] }], 1),
    )
    .mockResolvedValueOnce(
      response(
        { state: "waiting", rounds: 3, inProgrammingMode: ["15.15.255", "1.1.40"] },
        [{ kind: "round", number: 3, inProgrammingMode: ["15.15.255", "1.1.40"] }],
        2,
      ),
    )
    .mockResolvedValueOnce(
      response({ state: "programming", previousAddress: "15.15.255" }, [{ kind: "found", currentAddress: "15.15.255" }], 3),
    );
  await confirm();
  const progress = () => host!.querySelector(".address-programming-progress")!;
  expect(progress().querySelector("[data-instruction]")!.getAttribute("data-instruction")).toBe("press");
  expect(progress().textContent).toContain(en["addressProgramming.pressButton"]);
  expect(button(en["addressProgramming.stop"])).toBeTruthy();

  await act(async () => vi.advanceTimersByTime(600));
  await flush();
  expect(progress().querySelector("[data-instruction]")!.getAttribute("data-instruction")).toBe("release");
  expect(progress().textContent).toContain("(15.15.255, 1.1.40): release all but one");

  await act(async () => vi.advanceTimersByTime(600));
  await flush();
  expect(progress().textContent).toContain("Found 15.15.255; programming 1.1.30 now");
  expect(progress().textContent).toContain("cannot be stopped");
  // No stop button once the procedure runs.
  expect([...host!.querySelectorAll("button")].some((b) => b.textContent === en["addressProgramming.stop"])).toBe(false);
});

it("stops the wait with the running programming's id", async () => {
  await render();
  await fill();
  apiMock.pollAddressProgramming
    .mockReset()
    .mockResolvedValueOnce(response({ state: "waiting", rounds: 2, inProgrammingMode: [] }, [], 0))
    .mockResolvedValueOnce(response({ state: "stopped", rounds: 2 }, [], 0));
  await confirm();
  await act(async () => button(en["addressProgramming.stop"]).click());
  await flush();
  expect(apiMock.stopAddressProgramming).toHaveBeenCalledWith(4);
  expect(host!.textContent).toContain("Stopped after 2 rounds. Nothing was written.");
});

it("says 'yes' with the old and new address when it worked", async () => {
  apiMock.pollAddressProgramming
    .mockReset()
    .mockResolvedValue(response({ state: "finished", written: "yes", previousAddress: "15.15.255", wasFree: true }, [], 0));
  await render();
  const written = host!.querySelector(".address-programming-written")!;
  expect(written.getAttribute("data-written")).toBe("yes");
  expect(written.textContent).toBe(
    "Address written: yes, 15.15.255 → 1.1.30. The device answered at 1.1.30 and was restarted.",
  );
});

it("never lets a written-but-silent device read as 'nothing happened'", async () => {
  apiMock.pollAddressProgramming
    .mockReset()
    .mockResolvedValue(
      response({ state: "failed", written: "unconfirmed", step: 4, error: "step 4: no answer at 1.1.30" }, [], 0),
    );
  await render();
  const warning = host!.querySelector("[data-written]")!;
  expect(warning.getAttribute("data-written")).toBe("unconfirmed");
  expect(warning.className).toBe("form-warning");
  expect(warning.textContent).toContain("yes, but NOT confirmed");
  expect(warning.textContent).toContain("in step 4");
  expect(host!.textContent).not.toContain(en["addressProgramming.written.no"]);
});

it("says 'no' when it gave up before writing", async () => {
  apiMock.pollAddressProgramming
    .mockReset()
    .mockResolvedValue(
      response({ state: "failed", written: "no", step: null, error: "gave up after 60 rounds; nothing was written" }, [], 0),
    );
  await render();
  expect(host!.querySelector("[data-written]")!.textContent).toBe(en["addressProgramming.written.no"]);
  expect(host!.querySelector(".form-error")!.textContent).toContain("gave up");
});

// ADR-0089: only an address the device answers at counts towards achievements.
async function achievementEventsFor(status: AddressProgrammingStatusResponse["status"]): Promise<AchievementEvent[]> {
  const seen: AchievementEvent[] = [];
  const unsubscribe = subscribeAchievementEvents((event) => seen.push(event));
  apiMock.pollAddressProgramming.mockReset().mockResolvedValue(response(status, [], 1));
  try {
    await render();
  } finally {
    unsubscribe();
  }
  if (root) await act(async () => root!.unmount());
  host?.remove();
  root = undefined;
  host = undefined;
  return seen;
}

it("reports a programmed address only when the device answers at it", async () => {
  expect(await achievementEventsFor({ state: "finished", written: "yes", previousAddress: "15.15.255", wasFree: true }))
    .toEqual([{ type: "individualAddressVerified" }]);
  for (const written of ["unconfirmed", "noNeed", "no"] as const) {
    expect(await achievementEventsFor({ state: "finished", written, previousAddress: "15.15.255", wasFree: true }), written)
      .toEqual([]);
  }
  expect(await achievementEventsFor({ state: "failed", written: "unconfirmed", step: 3, error: "silent" })).toEqual([]);
});
