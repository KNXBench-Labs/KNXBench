/** Tests for the achievements overview: unlocked, locked, hidden, progress and status notes. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import AchievementsDialog from "./AchievementsDialog";
import type { AchievementDefinition } from "./achievementCatalog";
import type { TrackerSnapshot } from "./achievementTracker";
import { messages as en } from "./messages/en";

const CATALOG: AchievementDefinition[] = [
  { id: "foundation", tier: "bronze", hidden: false, glyph: "buildings", titleKey: "achievement.foundation.title", descriptionKey: "achievement.foundation.description", rule: { kind: "event", event: "projectCreated" } },
  { id: "time-traveller", tier: "bronze", hidden: false, glyph: "undo", titleKey: "achievement.time-traveller.title", descriptionKey: "achievement.time-traveller.description", rule: { kind: "count", event: "undo", goal: 100 } },
  { id: "night-shift", tier: "bronze", hidden: true, glyph: "moon", titleKey: "achievement.night-shift.title", descriptionKey: "achievement.night-shift.description", rule: { kind: "localHours", event: "projectSaved", fromHour: 2, toHour: 4 } },
  { id: "konami", tier: "legendary", hidden: true, glyph: "gamepad", titleKey: "achievement.konami.title", descriptionKey: "achievement.konami.description", rule: { kind: "event", event: "konamiCode" } },
];

let host: HTMLDivElement | undefined;
let root: Root | undefined;

afterEach(() => {
  act(() => root?.unmount());
  host?.remove();
  host = root = undefined;
});

async function render(snapshot: TrackerSnapshot, enabled = true, onClose = vi.fn()) {
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  await act(async () => {
    root!.render(<AchievementsDialog snapshot={snapshot} enabled={enabled} onClose={onClose} catalog={CATALOG} />);
  });
  return document.querySelector(".achievements-dialog") as HTMLElement;
}

function item(dialog: HTMLElement, id: string): HTMLElement {
  return dialog.querySelector(`[data-achievement="${id}"]`) as HTMLElement;
}

const READY: TrackerSnapshot = {
  status: "ready",
  record: {
    unlocked: { konami: "2026-10-07T19:30:00.000Z" },
    progress: { "time-traveller": 40 },
  },
};

describe("AchievementsDialog", () => {
  it("summarises how many are unlocked", async () => {
    const dialog = await render(READY);
    expect(dialog.textContent).toContain("1 of 4 unlocked");
  });

  it("lists unlocked achievements first, with their date and real text even when hidden", async () => {
    const dialog = await render(READY);
    const ids = [...dialog.querySelectorAll("[data-achievement]")].map((el) => el.getAttribute("data-achievement"));
    expect(ids[0]).toBe("konami");
    const konami = item(dialog, "konami");
    expect(konami.textContent).toContain(en["achievement.konami.title"]);
    expect(konami.textContent).toContain(en["achievement.konami.description"]);
    expect(konami.textContent).toContain("2026");
    expect(konami.textContent).toContain(en["achievements.tier.legendary"]);
  });

  it("keeps a locked hidden achievement secret", async () => {
    const dialog = await render(READY);
    const night = item(dialog, "night-shift");
    expect(night.textContent).toContain(en["achievements.hiddenTitle"]);
    expect(night.textContent).toContain(en["achievements.hiddenDescription"]);
    expect(dialog.textContent).not.toContain(en["achievement.night-shift.title"]);
    expect(dialog.textContent).not.toContain(en["achievement.night-shift.description"]);
  });

  it("shows progress towards a counting goal", async () => {
    const dialog = await render(READY);
    const traveller = item(dialog, "time-traveller");
    expect(traveller.textContent).toContain("40 / 100");
    const bar = traveller.querySelector("progress")!;
    expect(bar.getAttribute("value")).toBe("40");
    expect(bar.getAttribute("max")).toBe("100");
  });

  it("marks a locked one-shot achievement as locked", async () => {
    const dialog = await render(READY);
    expect(item(dialog, "foundation").textContent).toContain(en["achievements.locked"]);
  });

  it.each([
    ["loading", "achievements.loading"],
    ["unavailable", "achievements.unavailable"],
    ["readOnly", "achievements.readOnly"],
  ] as const)("explains the %s state", async (status, key) => {
    const dialog = await render({ ...READY, status });
    expect(dialog.textContent).toContain(en[key]);
  });

  it("explains that achievements are switched off", async () => {
    const dialog = await render(READY, false);
    expect(dialog.textContent).toContain(en["achievements.disabled"]);
  });

  it("closes from its close button", async () => {
    const onClose = vi.fn();
    const dialog = await render(READY, true, onClose);
    const close = [...dialog.querySelectorAll("button")].find((b) => b.textContent === en["achievements.close"])!;
    await act(async () => close.click());
    expect(onClose).toHaveBeenCalled();
  });
});
