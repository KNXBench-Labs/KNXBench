/** Pins visible, action-matching labels on every unsaved-changes dialog. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import UnsavedChangesDialog from "./UnsavedChangesDialog";
import { messages as en } from "./messages/en";
import { messages as de } from "./messages/de";

let host: HTMLDivElement | undefined;
let root: Root | undefined;
afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  root = undefined;
  host?.remove();
  host = undefined;
});

const VARIANTS = {
  quit: { keep: "quit.cancel", discard: "quit.discard", save: "quit.save" },
  replace: { keep: "replace.cancel", discard: "replace.discard", save: "replace.save" },
} as const;

async function mount(labels: { keep: string; discard: string; save: string }, saving = false) {
  const handlers = { onKeep: vi.fn(), onDiscard: vi.fn(), onSave: vi.fn() };
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  await act(async () => root!.render(
    <UnsavedChangesDialog titleId="t" title="Unsaved changes" message="m" details={["Next: other.knxproj"]}
      keepLabel={labels.keep} discardLabel={labels.discard} saveLabel={labels.save} savingLabel="Saving…"
      saving={saving} {...handlers} />,
  ));
  const buttons = [...host.querySelectorAll<HTMLButtonElement>(".unsaved-changes-actions button")];
  return { buttons, handlers };
}

describe("unsaved-changes dialog labels", () => {
  // The defect: the middle button rendered without a readable label. Every
  // answer must carry non-empty catalogue text in both shipped languages,
  // and the three answers must be three different sentences.
  for (const [variant, keys] of Object.entries(VARIANTS)) {
    for (const [language, catalogue] of [["en", en], ["de", de]] as const) {
      it(`${variant} (${language}) renders keep, discard and save with their own words`, async () => {
        const labels = { keep: catalogue[keys.keep], discard: catalogue[keys.discard], save: catalogue[keys.save] };
        for (const label of Object.values(labels)) expect(label.trim().length).toBeGreaterThan(3);
        expect(new Set(Object.values(labels)).size).toBe(3);
        const { buttons } = await mount(labels);
        expect(buttons.map((b) => b.textContent)).toEqual([labels.keep, labels.discard, labels.save]);
      });
    }
  }

  it("says what each answer does rather than a bare Cancel", () => {
    expect(en["quit.cancel"]).toBe("Keep editing");
    expect(en["replace.cancel"]).toBe("Keep editing");
    expect(de["quit.cancel"]).toBe("Weiter bearbeiten");
    expect(de["replace.cancel"]).toBe("Weiter bearbeiten");
    expect(en["replace.discard"]).toMatch(/^Discard changes/);
    expect(de["replace.discard"]).toMatch(/^Änderungen verwerfen/);
    expect(en["quit.discard"]).toMatch(/without saving/);
    expect(de["quit.discard"]).toMatch(/Ohne Speichern/);
    expect(en["replace.hint"]).toContain(en["replace.save"]);
    expect(de["replace.hint"]).toContain(de["replace.save"]);
  });

  it("gives each answer an explicit role instead of inheriting the accent fill", async () => {
    const { buttons } = await mount({ keep: "Keep", discard: "Discard", save: "Save" });
    expect(buttons.map((b) => b.className.match(/dialog-action--(\w+)/)?.[1])).toEqual(["secondary", "danger", "primary"]);
  });
});

describe("unsaved-changes dialog behaviour", () => {
  it("routes each answer, and Escape keeps editing", async () => {
    const { buttons, handlers } = await mount({ keep: "Keep", discard: "Discard", save: "Save" });
    await act(async () => buttons[0].click());
    await act(async () => buttons[1].click());
    await act(async () => buttons[2].click());
    expect([handlers.onKeep, handlers.onDiscard, handlers.onSave].map((f) => f.mock.calls.length)).toEqual([1, 1, 1]);
    await act(async () => {
      host!.querySelector('[role="dialog"]')!.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    });
    expect(handlers.onKeep).toHaveBeenCalledTimes(2);
    expect(document.activeElement).toBe(buttons[0]);
  });

  it("while saving keeps only 'keep editing' live and says it is saving", async () => {
    const { buttons } = await mount({ keep: "Keep", discard: "Discard", save: "Save" }, true);
    expect(buttons.map((b) => b.disabled)).toEqual([false, true, true]);
    expect(buttons[2].textContent).toBe("Saving…");
  });
});
