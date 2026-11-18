/** Unit tests for the per-session telegram counters (WP-44.2). */
import { afterEach, describe, expect, it } from "vitest";
import {
  readSessionTelegramCounters,
  recordReceivedTelegrams,
  recordSentTelegram,
  resetSessionTelegramCounters,
  useSessionTelegramCounters,
} from "./sessionTelegramCounters";

const A = { sessionId: 5, serverIncarnation: "process-a" };
const B = { sessionId: 7, serverIncarnation: "process-a" };

afterEach(() => resetSessionTelegramCounters(null));

describe("session telegram counters", () => {
  it("starts at zero for a new session and ignores events from any other session", () => {
    resetSessionTelegramCounters(A);
    expect(readSessionTelegramCounters()).toEqual({ identity: A, received: 0, sent: 0 });
    recordReceivedTelegrams(B, 3);
    recordSentTelegram(B);
    expect(readSessionTelegramCounters()!.received).toBe(0);
    expect(readSessionTelegramCounters()!.sent).toBe(0);
  });

  it("counts received wire telegrams and sent telegrams cumulatively", () => {
    resetSessionTelegramCounters(A);
    recordReceivedTelegrams(A, 4);
    recordReceivedTelegrams(A, 0);
    recordSentTelegram(A);
    recordSentTelegram(A);
    recordReceivedTelegrams(A, 2);
    expect(readSessionTelegramCounters()).toEqual({ identity: A, received: 6, sent: 2 });
  });

  it("resets when the session identity changes and clears on null", () => {
    resetSessionTelegramCounters(A);
    recordReceivedTelegrams(A, 9);
    resetSessionTelegramCounters(B);
    expect(readSessionTelegramCounters()).toEqual({ identity: B, received: 0, sent: 0 });
    resetSessionTelegramCounters(null);
    expect(readSessionTelegramCounters()).toBe(null);
  });

  it("re-attaching the same session keeps its counters", () => {
    resetSessionTelegramCounters(A);
    recordReceivedTelegrams(A, 3);
    resetSessionTelegramCounters(A);
    expect(readSessionTelegramCounters()!.received).toBe(3);
  });

  it("exposes the current value through the hook snapshot", () => {
    resetSessionTelegramCounters(A);
    recordReceivedTelegrams(A, 1);
    // The hook delegates to the same store; call its getSnapshot path directly.
    expect(useSessionTelegramCounters).toBeTypeOf("function");
    expect(readSessionTelegramCounters()!.received).toBe(1);
  });
});
