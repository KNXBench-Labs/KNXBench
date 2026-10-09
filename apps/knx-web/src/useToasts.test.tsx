/** Verifies the toast queue: errors reported once, achievement popups linger and leave animated. */
// @vitest-environment happy-dom
import { act, StrictMode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import { subscribeAchievementEvents, type AchievementEvent } from "./achievementEvents";
import { ACHIEVEMENT_TOAST_MS, TOAST_EXIT_FALLBACK_MS, useToasts, type AchievementPopup } from "./toast";

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
