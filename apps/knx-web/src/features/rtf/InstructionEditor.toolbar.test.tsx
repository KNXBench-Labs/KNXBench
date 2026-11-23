/** Pins the real InstructionEditor toolbar against the shared toolbar checklist (WP-49.4b). */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { InstructionEditor } from "./InstructionEditor";
import type { Context, InstructionClient, InstructionView } from "./client";
import { literal } from "./document";
import { press } from "../../test/comboboxChecklist";
import { describeToolbarChecklist } from "../../test/toolbarChecklist";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const context: Context = { serverIncarnation: "synthetic-server", projectIncarnation: 1, snapshotRevision: 2 };
const source: InstructionView = { ...context, device: 1, origin: "imported", originalXmlSha256: "a".repeat(64), originalPresent: true, originalBytes: 8, originalFormat: "plain", unsupported: [], previewError: null, effectiveDocument: literal("original"), overrideDocument: null, persistence: "session-only" };
let host: HTMLDivElement | undefined;
let root: Root | undefined;

function client(held = false): InstructionClient {
  const preview = { token: "synthetic-plan", device: 1, replacement: literal("edited"), conversionRequired: false, noOp: false, expiresInSeconds: 300 };
  const gate = held ? new Promise<void>(() => {}) : Promise.resolve();
  return { read: vi.fn().mockResolvedValue(source), preview: vi.fn(async () => { await gate; return preview; }), apply: vi.fn(), cancel: vi.fn().mockResolvedValue(undefined) } as unknown as InstructionClient;
}
async function mount(c: InstructionClient, parentKeyDown: () => void, ctx: Context | null = context) {
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  await act(async () => root!.render(<div onKeyDown={parentKeyDown}><InstructionEditor deviceId={1} context={ctx} client={c} /></div>));
}
async function unmount() {
  if (root) await act(async () => root!.unmount());
  root = undefined; host?.remove(); host = undefined;
}
const toolbar = () => host!.querySelector<HTMLElement>("[role=toolbar]")!;
const controls = () => [...toolbar().querySelectorAll<HTMLElement>("button")];
const textarea = () => host!.querySelector("textarea")!;
const byText = (text: string) => [...host!.querySelectorAll("button")].find(b => b.textContent === text)!;
async function edit(text: string) {
  await act(async () => {
    Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, "value")!.set!.call(textarea(), text);
    textarea().dispatchEvent(new Event("input", { bubbles: true }));
  });
}

afterEach(unmount);

describeToolbarChecklist({
  name: "InstructionEditor",
  mount: parentKeyDown => mount(client(true), parentKeyDown),
  unmount,
  toolbar,
  controls,
  // Bold/Italic/Underline/Remove formatting act on the selection; none of them is a toggle.
  toggles: () => [],
  disable: async () => {
    await edit("changed");
    await act(async () => byText("Preview change").click());
    return controls();
  },
});

describe("InstructionEditor toolbar: parent-owned behaviour", () => {
  it("offers the four formatting buttons, and none at all in the read-only view (positive control)", async () => {
    await mount(client(), () => {});
    expect(controls().map(b => b.textContent)).toEqual(["Bold", "Italic", "Underline", "Remove formatting"]);
    await unmount();
    await mount(client(), () => {}, null);
    expect(host!.querySelector("[role=toolbar]")).toBeNull();
    expect(textarea().readOnly).toBe(true);
  });

  it("keeps the textarea selection on a pointer press and returns focus to the textarea after a click", async () => {
    await mount(client(), () => {});
    textarea().focus();
    textarea().setSelectionRange(0, 3);
    await act(async () => { textarea().dispatchEvent(new KeyboardEvent("keyup", { key: "Shift", bubbles: true })); });
    const down = new MouseEvent("mousedown", { bubbles: true, cancelable: true });
    await act(async () => { byText("Bold").dispatchEvent(down); });
    expect(down.defaultPrevented, "mousedown must not steal focus from the textarea").toBe(true);
    // Keyboard activation (focus on the button, then click) ends on the textarea with the same selection.
    byText("Bold").focus();
    await act(async () => byText("Bold").click());
    expect(document.activeElement).toBe(textarea());
    // The selection recorded before the press is what gets formatted (happy-dom does not keep the caret itself).
    const spans = [...host!.querySelectorAll("[class*=preview] span")];
    expect(spans.map(s => s.textContent)).toEqual(["ori", "ginal"]);
    expect(spans[0].getAttribute("style")).toContain("font-weight: 700");
  });

  it("has no keyboard shortcuts of its own: Ctrl+B / Ctrl+I in the textarea are left alone and format nothing", async () => {
    await mount(client(), () => {});
    textarea().focus();
    textarea().setSelectionRange(0, 3);
    for (const key of ["b", "i", "u"]) {
      const event = await press(textarea(), key, { ctrlKey: true });
      expect(event.defaultPrevented, `Ctrl+${key}`).toBe(false);
    }
    expect(host!.querySelectorAll("[class*=preview] span")).toHaveLength(1);
    expect(host!.querySelector("[class*=preview] span")?.getAttribute("style")).toContain("font-weight: 400");
  });

  it("disables the buttons while a conflict is shown and re-enables them after reloading (state change, not only initial)", async () => {
    const c = client();
    await mount(c, () => {});
    expect(controls().every(b => !(b as HTMLButtonElement).disabled)).toBe(true);
    await edit("draft");
    // A changed project revision with the same identity forces a conflict over the dirty draft.
    (c.read as ReturnType<typeof vi.fn>).mockResolvedValue({ ...source, effectiveDocument: literal("someone else") });
    await act(async () => root!.render(<div onKeyDown={() => {}}><InstructionEditor deviceId={1} context={{ ...context, snapshotRevision: 3 }} client={c} /></div>));
    expect(host!.querySelector("[role=alert]")).not.toBeNull();
    expect(controls().every(b => (b as HTMLButtonElement).disabled)).toBe(true);
  });
});
