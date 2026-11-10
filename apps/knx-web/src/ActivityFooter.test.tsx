/** Verifies accessible delayed activity presentation, concurrent rows and sticky error history. */
// @vitest-environment happy-dom
import { act, StrictMode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { ActivityProvider, useActivityController } from "./ActivityContext";
import ActivityFooter from "./ActivityFooter";
import { ACTIVITY_RECENT_LIMIT, type ActivityController, type ActivityHandle } from "./activityController";
import { resetSettingsForTests, setSetting } from "./settingsStore";
import { resetUiLanguageForTests } from "./uiLanguage";
import { messages as en } from "./messages/en";
import { messages as de } from "./messages/de";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
let activity: ActivityController;
let host: HTMLDivElement;
let root: Root;
const details = vi.fn();
function Capture() { activity = useActivityController(); return null; }
function Shell({ panel = true }: { panel?: boolean }) {
  return <StrictMode><ActivityProvider><Capture />{panel && <div data-producer-view="catalogue" />}
    <ActivityFooter canOpenDetails={entry => entry.destination === "catalogue"} onOpenDetails={details} /></ActivityProvider></StrictMode>;
}
beforeEach(() => {
  vi.useFakeTimers(); resetSettingsForTests(); localStorage.clear(); resetUiLanguageForTests(); details.mockReset();
  host = document.createElement("div"); document.body.append(host); root = createRoot(host);
});
afterEach(async () => { await act(async () => root.unmount()); host.remove(); vi.useRealTimers(); });
const toggle = () => host.querySelector<HTMLButtonElement>(".activity-footer-toggle")!;
const summary = () => host.querySelector(".activity-footer-summary")!;
async function render(panel = true) { await act(async () => root.render(<Shell panel={panel} />)); }
async function advance(ms: number) { await act(async () => vi.advanceTimersByTimeAsync(ms)); }
async function begin(id: string): Promise<ActivityHandle> {
  let task!: ActivityHandle;
  await act(async () => { task = activity.begin({ kind: "open", operationId: id, access: "write" }, { source: `${id}.knxdb` }); });
  return task;
}
it("acknowledges inventory now, delays global shimmer and never animates terminal rows", async () => {
  await render(); const quick = await begin("quick");
  expect(toggle().textContent).toContain("(1)"); expect(summary().textContent).toBe("");
  await advance(1_999); await act(async () => { quick.finish(); });
  await advance(4_000); expect(summary().textContent).toBe(""); expect(host.querySelector(".task-status-text")).toBeNull();
  const slow = await begin("slow");
  await act(async () => { slow.update({ phaseKey: "loadProgress.phase.loadStoredProject" }); });
  await advance(2_000);
  expect(summary().textContent).toContain(en["loadProgress.phase.loadStoredProject"]);
  expect(summary().textContent).toContain("slow.knxdb"); expect(summary().querySelector(".task-status-text")).not.toBeNull();
  await act(async () => { slow.finish(); });
  expect(summary().textContent).toContain(en["backgroundActivity.state.succeeded"]);
  expect(summary().textContent).not.toContain(en["loadProgress.phase.loadStoredProject"]);
  expect(host.querySelector(".task-status-text")).toBeNull();
  await advance(3_000); expect(summary().textContent).toBe("");
  await act(async () => toggle().click());
  expect(host.querySelectorAll("li[data-state=succeeded]")).toHaveLength(2);
});
it.each(["en", "de"] as const)("%s: concurrent errors stay accessible until acknowledged, not merely closed", async language => {
  setSetting("uiLanguage", language); resetUiLanguageForTests(); const m = language === "en" ? en : de;
  await render(); const a = await begin("first"); const b = await begin("second");
  await advance(2_000); await act(async () => { a.finish("failed", "fictional failure"); b.update({ state: "waiting" }); toggle().click(); });
  expect(host.querySelectorAll("li")).toHaveLength(2);
  expect(host.querySelector("li[data-state=failed] .task-status-text")).toBeNull();
  expect(host.querySelector("li[data-state=waiting] .task-status-text")).toBeNull();
  expect(host.textContent).toContain(m["backgroundActivity.state.waiting"]);
  expect(Array.from(host.querySelectorAll("button")).some(button => /cancel|abbrechen/i.test(button.textContent ?? ""))).toBe(false);
  const region = host.querySelector<HTMLElement>(".activity-footer-panel")!;
  expect(toggle().getAttribute("aria-controls")).toBe(region.id);
  await act(async () => region.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true })));
  expect(toggle().getAttribute("aria-expanded")).toBe("false"); expect(document.activeElement).toBe(toggle());
  await advance(60_000); expect(summary().textContent).toContain(m["backgroundActivity.state.failed"]);
  await act(async () => toggle().click());
  const ack = Array.from(host.querySelectorAll<HTMLButtonElement>("button")).find(button => button.textContent === m["backgroundActivity.acknowledge"])!;
  await act(async () => { ack.focus(); ack.click(); });
  expect(document.activeElement).toBe(toggle());
  expect(summary().textContent).not.toContain(m["backgroundActivity.state.failed"]);
  expect(host.textContent).toContain("fictional failure");
});
it.each(["decision", "failed"] as const)("%s drops the running phase, retains the true outcome and does not shimmer", async state => {
  await render(); const task = await begin("phase");
  await act(async () => { task.update({ phaseKey: "loadProgress.phase.openContainer", phase: "Old running phase" }); });
  await advance(2_000);
  expect(summary().textContent).toContain(en["loadProgress.phase.openContainer"]);
  await act(async () => { task.finish(state, state === "failed" ? "Exact failure" : undefined); toggle().click(); });
  expect(summary().textContent).toContain(en[`backgroundActivity.state.${state}`]);
  expect(host.textContent).not.toContain(en["loadProgress.phase.openContainer"]);
  expect(host.textContent).not.toContain("Old running phase");
  expect(host.querySelector(".task-status-text")).toBeNull();
  if (state === "failed") expect(host.textContent).toContain("Exact failure");
});
it.each(["en", "de"] as const)("%s: history retains source project and real device captions after replacement", async language => {
  setSetting("uiLanguage", language); resetUiLanguageForTests();
  await render(); const scope = {};
  await act(async () => {
    activity.begin({ kind: "deviceDetail", operationId: "old-device", access: "read", projectScope: scope, entityId: "42" }, {
      projectName: "Original installation / Annex", source: "original.knxdb", entities: [{ label: "Pump controller", address: "1.1.42" }],
    });
  });
  await advance(2_000);
  expect(summary().textContent).toContain("Original installation / Annex");
  expect(summary().textContent).toContain("Pump controller · 1.1.42");
  await act(async () => {
    activity.invalidateScope(scope);
    activity.begin({ kind: "deviceDetail", operationId: "new-device", access: "read", projectScope: {}, entityId: "42" }, {
      projectName: "Replacement installation", entities: [{ label: "Different controller", address: "2.1.42" }],
    }).finish();
    toggle().click();
  });
  const previous = host.querySelector('li[data-state="discarded"]')!;
  expect(previous.textContent).toContain("original.knxdb");
  expect(previous.textContent).toContain("Original installation / Annex");
  expect(previous.textContent).toContain("Pump controller · 1.1.42");
  expect(previous.textContent).not.toContain("Replacement installation");
  expect(previous.textContent).not.toContain("Different controller");
});
it.each([false, true])("moves focus to the surviving toggle before acknowledging (row evicted: %s)", async evict => {
  await render(); const failed = await begin("failed-focus");
  await act(async () => { failed.finish("failed", "focus failure"); });
  if (evict) await act(async () => {
    for (let index = 0; index < ACTIVITY_RECENT_LIMIT; index++) {
      activity.begin({ kind: "save", operationId: `newer-${index}`, access: "write" }).finish();
    }
  });
  await act(async () => toggle().click());
  const ack = Array.from(host.querySelectorAll<HTMLButtonElement>("button")).find(button => button.textContent === en["backgroundActivity.acknowledge"])!;
  await act(async () => ack.focus()); expect(document.activeElement).toBe(ack);
  await act(async () => ack.click());
  expect(document.activeElement).toBe(toggle());
  expect(host.querySelector(`[data-activity-id="${failed.id}"]`) === null).toBe(evict);
  expect(host.querySelectorAll("li")).toHaveLength(evict ? ACTIVITY_RECENT_LIMIT : 1);
  expect(toggle().getAttribute("aria-expanded")).toBe("true");
});
it.each(["en", "de"] as const)("%s: a 500-target comparison has a compact summary and lossless expanded context", async language => {
  setSetting("uiLanguage", language); resetUiLanguageForTests();
  await render();
  await act(async () => {
    activity.begin({ kind: "compare", operationId: "many", access: "read", projectScope: {} }, {
      projectName: "Large fictional installation",
      entities: Array.from({ length: 500 }, (_, index) => ({ label: `Controller ${index + 1}`, address: `1.${Math.floor(index / 250)}.${index % 250 + 1}` })),
    });
  });
  await advance(2_000);
  expect(summary().textContent).toContain("Controller 1 · 1.0.1");
  expect(summary().textContent).toContain(language === "en" ? "499 more devices" : "499 weitere Geräte");
  expect(summary().textContent).not.toContain("Controller 500");
  expect(summary().textContent!.length).toBeLessThan(250);
  await act(async () => toggle().click());
  expect(host.querySelector(".activity-footer-panel li strong")!.textContent).toContain("Controller 500 · 1.1.250");
  expect(activity.getSnapshot()[0].entities).toHaveLength(500);
});
it("retains observers through view removal and routes details without worker commands", async () => {
  await render();
  await act(async () => { activity.begin({ kind: "catalogue", operationId: "catalogue-1", access: "independent" }, { destination: "catalogue", source: "fictional.knxprod" }); });
  await render(false); await advance(2_000);
  expect(summary().textContent).toContain("fictional.knxprod");
  await act(async () => toggle().click());
  const open = Array.from(host.querySelectorAll<HTMLButtonElement>("button")).find(button => button.textContent === en["backgroundActivity.details"])!;
  await act(async () => open.click());
  expect(details).toHaveBeenCalledTimes(1);
  expect(details.mock.calls[0][0].identity.operationId).toBe("catalogue-1");
  expect(activity.getSnapshot()).toHaveLength(1);
});
