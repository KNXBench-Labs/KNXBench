/** Verifies that every error toast is reported to the achievement tracker exactly once. */
// @vitest-environment happy-dom
import { act, StrictMode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, it } from "vitest";
import { subscribeAchievementEvents, type AchievementEvent } from "./achievementEvents";
import { useToasts } from "./toast";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let host: HTMLDivElement | undefined;
let root: Root | undefined;
let toasts: ReturnType<typeof useToasts> | undefined;

function Probe() {
  toasts = useToasts();
  return null;
}

afterEach(async () => {
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
