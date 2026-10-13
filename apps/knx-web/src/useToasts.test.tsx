/** Verifies the toast queue: lifetimes, pauses, visible slots, retained history, animated exits. */
// @vitest-environment happy-dom
import { act, StrictMode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import { subscribeAchievementEvents, type AchievementEvent } from "./achievementEvents";
import { ACHIEVEMENT_TOAST_MS, MAX_QUEUED_TOASTS, MAX_VISIBLE_TOASTS, TOAST_EXIT_FALLBACK_MS, useToasts, type AchievementPopup } from "./toast";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let host: HTMLDivElement | undefined;
let root: Root | undefined;
let toasts: ReturnType<typeof useToasts> | undefined;

function Probe() {
  toasts = useToasts();
  return null;
}

afterEach(async () => {
  vi.useRealTimers();
  if (root) await act(async () => root!.unmount());
  host?.remove();
  host = undefined;
  root = undefined;
  toasts = undefined;
});

it("reports each error toast once, even under StrictMode, and nothing for other toasts", async () => {
  host = document.createElement("div");
  document.body.appendChild(host);
  root = createRoot(host);
  await act(async () => root!.render(<StrictMode><Probe /></StrictMode>));
  const seen: AchievementEvent[] = [];
  const stop = subscribeAchievementEvents((event) => seen.push(event));
  try {
    await act(async () => toasts!.pushError("The gateway did not answer."));
    await act(async () => toasts!.pushFun("Saved."));
    await act(async () => toasts!.pushError("Still no answer."));
  } finally {
    stop();
  }
  expect(seen).toEqual([{ type: "errorToastShown" }, { type: "errorToastShown" }]);
});

it("retains raw error text and exact identities after replacement, clear and expiry", async () => {
  vi.useFakeTimers(); await mount();
  await act(() => { toasts!.pushError("first\nraw"); toasts!.pushError("second"); toasts!.pushFun("saved"); });
  expect(toasts!.notifications).toHaveLength(3);
  expect(toasts!.notifications.map(n => n.id)).toEqual([0, 1, 2]);
  expect(toasts!.notifications[0].entry).toMatchObject({ source: "ui:toast:error", message: "first\nraw", severity: "error" });
  expect(toasts!.notifications[0].entry.detail).toContain("first\nraw");
  await act(() => toasts!.clearErrors());
  await act(() => vi.advanceTimersByTime(10000));
  expect(toasts!.toasts).toEqual([]);
  expect(toasts!.notifications).toHaveLength(3);
});
it("retains achievements and reports notifications evicted by the bounded session history", async () => {
  vi.useFakeTimers(); await mount();
  await act(() => toasts!.pushAchievements([popup], n => `+${n} more`));
  expect(toasts!.notifications[0].entry.detail).toBe(popup.description);
  await act(() => { for (let i = 0; i < 1001; i++) toasts!.pushFun(`note ${i}`, 1); });
  expect(toasts!.notifications).toHaveLength(1000);
  expect(toasts!.notificationDroppedCount).toBe(2);
  expect(toasts!.notifications[999].entry.message).toBe("note 1000");
});

// Ported from the Codex ux-feedback branch (AP1, 219e4c3e), adapted to main's
// notification history, which replaces that branch's separate error archive.
it("pauses a focused toast and resumes only its remaining visible lifetime", async () => {
  vi.useFakeTimers();
  await mount();
  await act(async () => toasts!.pushFun("Read this before closing."));
  const id = toasts!.toasts[0].id;
  await act(async () => vi.advanceTimersByTime(3000));
  await act(async () => toasts!.pause(id, "focus"));
  await act(async () => vi.advanceTimersByTime(20000));
  expect(toasts!.toasts[0].leaving).toBeFalsy();
  await act(async () => toasts!.resume(id, "focus"));
  await act(async () => vi.advanceTimersByTime(5999));
  expect(toasts!.toasts[0].leaving).toBeFalsy();
  await act(async () => vi.advanceTimersByTime(1));
  expect(toasts!.toasts[0].leaving).toBe(true);
});

it("does not resume a hovered toast while its close button still has focus", async () => {
  vi.useFakeTimers();
  await mount();
  await act(async () => toasts!.pushFun("Long reading."));
  const id = toasts!.toasts[0].id;
  await act(async () => { toasts!.pause(id, "hover"); toasts!.pause(id, "focus"); });
  await act(async () => toasts!.resume(id, "hover"));
  await act(async () => vi.advanceTimersByTime(20000));
  expect(toasts!.toasts[0].leaving).toBeFalsy();
});

it("holds every lifetime while the document is hidden", async () => {
  vi.useFakeTimers();
  await mount();
  let state: DocumentVisibilityState = "visible";
  const descriptor = Object.getOwnPropertyDescriptor(document, "visibilityState");
  Object.defineProperty(document, "visibilityState", { configurable: true, get: () => state });
  try {
    await act(async () => toasts!.pushError("Refused while you were away."));
    await act(async () => vi.advanceTimersByTime(4000));
    state = "hidden";
    await act(async () => { document.dispatchEvent(new Event("visibilitychange")); });
    await act(async () => vi.advanceTimersByTime(60000));
    expect(toasts!.toasts[0].leaving).toBeFalsy();
    state = "visible";
    await act(async () => { document.dispatchEvent(new Event("visibilitychange")); });
    await act(async () => vi.advanceTimersByTime(4999));
    expect(toasts!.toasts[0].leaving).toBeFalsy();
    await act(async () => vi.advanceTimersByTime(1));
    expect(toasts!.toasts[0].leaving).toBe(true);
  } finally {
    if (descriptor) Object.defineProperty(document, "visibilityState", descriptor);
    else delete (document as { visibilityState?: unknown }).visibilityState;
  }
});

it("retires all owned timers on unmount", async () => {
  vi.useFakeTimers();
  await mount();
  await act(async () => { toasts!.pushFun("Saved."); toasts!.pushError("Refused."); });
  expect(vi.getTimerCount()).toBeGreaterThan(0);
  await act(async () => root!.unmount());
  root = undefined;
  expect(vi.getTimerCount()).toBe(0);
});

it("keeps multiple errors instead of replacing the previous unread message", async () => {
  vi.useFakeTimers(); await mount();
  await act(async () => { toasts!.pushError("First import refused."); toasts!.pushError("Second import refused."); });
  expect(toasts!.toasts).toHaveLength(2);
  expect(toasts!.toasts[0].message).toContain("First import refused.");
  expect(toasts!.toasts[1].message).toContain("Second import refused.");
});

it("starts a queued toast's full lifetime only when a visible slot becomes free", async () => {
  vi.useFakeTimers(); await mount();
  await act(async () => { for (let i = 0; i < MAX_VISIBLE_TOASTS + 1; i++) toasts!.pushFun(`Message ${i}`); });
  expect(toasts!.toasts).toHaveLength(MAX_VISIBLE_TOASTS);
  expect(toasts!.pendingCount).toBe(1);
  await act(async () => vi.advanceTimersByTime(ACHIEVEMENT_TOAST_MS + TOAST_EXIT_FALLBACK_MS));
  expect(toasts!.toasts).toMatchObject([{ message: "Message 3" }]);
  expect(toasts!.pendingCount).toBe(0);
  await act(async () => vi.advanceTimersByTime(ACHIEVEMENT_TOAST_MS - 1));
  expect(toasts!.toasts[0].leaving).toBeFalsy();
  await act(async () => vi.advanceTimersByTime(1));
  expect(toasts!.toasts[0].leaving).toBe(true);
});

it("retires the oldest waiting card on overflow, counts it, and keeps it in the history", async () => {
  vi.useFakeTimers(); await mount();
  await act(async () => { for (let i = 0; i < MAX_QUEUED_TOASTS + 2; i++) toasts!.pushFun(`Card ${i}`); });
  expect(toasts!.queueDropped).toBe(2);
  // The cards on screen are never the ones retired.
  expect(toasts!.toasts.map((entry) => entry.message)).toEqual(["Card 0", "Card 1", "Card 2"]);
  expect(toasts!.pendingCount).toBe(MAX_QUEUED_TOASTS - MAX_VISIBLE_TOASTS);
  const messages = toasts!.notifications.map((notification) => notification.entry.message);
  expect(messages).toHaveLength(MAX_QUEUED_TOASTS + 2);
  expect(messages).toContain("Card 3");
  expect(messages).toContain("Card 4");
});

const popup: AchievementPopup = { title: "Bus Master", description: "Unlocked every other achievement.", tier: "legendary", glyph: "trophy" };

async function mount() {
  host = document.createElement("div");
  document.body.appendChild(host);
  root = createRoot(host);
  await act(async () => root!.render(<StrictMode><Probe /></StrictMode>));
}

it("keeps an achievement popup for nine seconds, then lets it leave before removing it", async () => {
  vi.useFakeTimers();
  await mount();
  await act(async () => toasts!.pushAchievements([popup], (n) => `+${n} more`));
  expect(ACHIEVEMENT_TOAST_MS).toBe(9000);
  await act(async () => vi.advanceTimersByTime(ACHIEVEMENT_TOAST_MS - 1));
  expect(toasts!.toasts).toMatchObject([{ kind: "achievement", message: "Bus Master" }]);
  expect(toasts!.toasts[0].leaving).toBeFalsy();
  await act(async () => vi.advanceTimersByTime(1));
  // Leaving, not gone: the stack plays the exit animation first.
  expect(toasts!.toasts).toHaveLength(1);
  expect(toasts!.toasts[0].leaving).toBe(true);
  await act(async () => vi.advanceTimersByTime(TOAST_EXIT_FALLBACK_MS));
  expect(toasts!.toasts).toEqual([]);
});

it("removes a leaving popup as soon as its exit animation has ended", async () => {
  vi.useFakeTimers();
  await mount();
  await act(async () => toasts!.pushAchievements([popup], (n) => `+${n} more`));
  const id = toasts!.toasts[0].id;
  await act(async () => toasts!.dismiss(id));
  expect(toasts!.toasts[0].leaving).toBe(true);
  await act(async () => toasts!.finishExit(id));
  expect(toasts!.toasts).toEqual([]);
});

it("keeps an error for nine seconds, preserves its disclosure, then leaves animated", async () => {
  vi.useFakeTimers();
  await mount();
  await act(async () => toasts!.pushError("Save refused."));
  await act(async () => vi.advanceTimersByTime(ACHIEVEMENT_TOAST_MS - 1));
  expect(toasts!.toasts).toMatchObject([{ kind: "error", serverText: true }]);
  expect(toasts!.toasts[0].message).toContain("Save refused.");
  expect(toasts!.toasts[0].leaving).toBeFalsy();
  await act(async () => vi.advanceTimersByTime(1));
  expect(toasts!.toasts[0].leaving).toBe(true);
  await act(async () => vi.advanceTimersByTime(TOAST_EXIT_FALLBACK_MS));
  expect(toasts!.toasts).toEqual([]);
});

it.each(["fun", "error", "achievement"] as const)("lets a manually dismissed %s toast finish its exit", async (kind) => {
  vi.useFakeTimers();
  await mount();
  await act(async () => {
    if (kind === "fun") toasts!.pushFun("Saved.");
    else if (kind === "error") toasts!.pushError("Refused.");
    else toasts!.pushAchievements([popup], (n) => `+${n} more`);
  });
  const id = toasts!.toasts[0].id;
  await act(async () => toasts!.dismiss(id));
  expect(toasts!.toasts).toMatchObject([{ id, kind, leaving: true }]);
  await act(async () => toasts!.finishExit(id));
  expect(toasts!.toasts).toEqual([]);
  await act(async () => vi.advanceTimersByTime(ACHIEVEMENT_TOAST_MS + TOAST_EXIT_FALLBACK_MS));
  expect(toasts!.toasts).toEqual([]);
});

it("does not let a replaced error's timeout shorten the replacement's lifetime", async () => {
  vi.useFakeTimers();
  await mount();
  await act(async () => toasts!.pushError("First refusal."));
  await act(async () => vi.advanceTimersByTime(5000));
  await act(async () => toasts!.pushError("Second refusal.", { serverText: false }));
  await act(async () => vi.advanceTimersByTime(ACHIEVEMENT_TOAST_MS - 1));
  expect(toasts!.toasts).toHaveLength(1);
  expect(toasts!.toasts[0].message).toContain("Second refusal.");
  expect(toasts!.toasts[0]).toMatchObject({ serverText: false });
  expect(toasts!.toasts[0].leaving).toBeFalsy();
  await act(async () => vi.advanceTimersByTime(1));
  expect(toasts!.toasts[0].leaving).toBe(true);
});

it("clears errors without removing other toast kinds or resurrecting an expired error", async () => {
  vi.useFakeTimers();
  await mount();
  await act(async () => {
    toasts!.pushError("Refused.");
    toasts!.pushFun("Saved.");
    toasts!.pushAchievements([popup], (n) => `+${n} more`);
    toasts!.clearErrors();
  });
  expect(toasts!.toasts.map((entry) => entry.kind)).toEqual(["fun", "achievement"]);
  await act(async () => vi.advanceTimersByTime(ACHIEVEMENT_TOAST_MS + TOAST_EXIT_FALLBACK_MS));
  expect(toasts!.toasts).toEqual([]);
});

it("keeps a standard status toast for nine seconds and gives it the achievement exit lifecycle", async () => {
  vi.useFakeTimers();
  await mount();
  await act(async () => toasts!.pushFun("Saved."));
  await act(async () => vi.advanceTimersByTime(ACHIEVEMENT_TOAST_MS - 1));
  expect(toasts!.toasts).toMatchObject([{ kind: "fun", message: "Saved." }]);
  expect(toasts!.toasts[0].leaving).toBeFalsy();
  await act(async () => vi.advanceTimersByTime(1));
  expect(toasts!.toasts[0].leaving).toBe(true);
  await act(async () => vi.advanceTimersByTime(TOAST_EXIT_FALLBACK_MS));
  expect(toasts!.toasts).toEqual([]);
});

it("keeps an update actionable for nine seconds and retains its release link in notification history", async () => {
  vi.useFakeTimers(); await mount();
  await act(() => toasts!.pushUpdate("KNXBench 0.1.0-alpha.10 is available", "https://github.com/KNXBench-Labs/KNXBench/releases/tag/v0.1.0-alpha.10"));
  expect(toasts!.toasts).toMatchObject([{ kind: "update", releaseUrl: "https://github.com/KNXBench-Labs/KNXBench/releases/tag/v0.1.0-alpha.10", serverText: false }]);
  expect(toasts!.notifications[0].entry).toMatchObject({ source: "ui:toast:update", severity: "info", detail: "https://github.com/KNXBench-Labs/KNXBench/releases/tag/v0.1.0-alpha.10" });
  await act(() => vi.advanceTimersByTime(8999)); expect(toasts!.toasts[0].leaving).toBeFalsy();
  await act(() => vi.advanceTimersByTime(1)); expect(toasts!.toasts[0].leaving).toBe(true);
  await act(() => vi.advanceTimersByTime(TOAST_EXIT_FALLBACK_MS)); expect(toasts!.toasts).toEqual([]);
  expect(toasts!.notifications).toHaveLength(1);
});
