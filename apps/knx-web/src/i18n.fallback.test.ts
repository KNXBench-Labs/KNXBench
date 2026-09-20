/** Proves translateFor's runtime English fallback for a German key the compiler cannot omit. */
// A dedicated file for one test, because `vi.mock` below replaces
// `./messages/de` for every test in this module — mixing it with the rest
// of `i18n.test.tsx`'s tests would mean unmocking partway through, which
// is more fragile than just paying for a second file.
import { describe, expect, it, vi } from "vitest";
import type { Messages, MessageKey } from "./messages/en";

// D3 promises English on a *runtime* miss in the German catalogue, but
// `de.ts`'s `Record<MessageKey, string>` annotation makes that situation a
// compile error in real code — there is no legitimate way to hand
// `translateFor` a German catalogue that is missing a key the type system
// knows about. To exercise the runtime behaviour at all, this test mocks
// the module and casts the mock's shape past `Record<MessageKey, string>`,
// standing in for the "corrupted build" scenario the fallback actually
// guards against. This is a deliberate, coordinator-mandated escape hatch,
// not a shortcut around a real typing gap — see the task 1 report.
vi.mock("./messages/de", () => {
  const partial: Partial<Record<MessageKey, string>> = {
    "toolbar.openProject": "Projekt öffnen…",
    "toolbar.openNativeProject": "Öffnen (.knxdb)…",
    // "toolbar.save" deliberately omitted.
    "toolbar.saveAs": "Speichern unter…",
    "toolbar.undo": "Rückgängig",
    "toolbar.redo": "Wiederholen",
    "toolbar.search": "Suchen… (Strg+K)",
    "toolbar.log": "Protokoll",
    "toolbar.settings": "Einstellungen",
  };
  return { messages: partial as unknown as Messages };
});

describe("translateFor (runtime English fallback)", () => {
  it("falls back to English when the active language's catalogue is missing the key", async () => {
    const { translateFor } = await import("./i18n");
    expect(translateFor("de", "toolbar.save")).toBe("Save");
  });

  it("still prefers German for a key German actually has", async () => {
    const { translateFor } = await import("./i18n");
    expect(translateFor("de", "toolbar.undo")).toBe("Rückgängig");
  });
});
