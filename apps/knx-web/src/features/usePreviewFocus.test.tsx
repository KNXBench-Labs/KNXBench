/** Tests pending preview focus ownership and disposal through the real hook's public contract. */
// @vitest-environment happy-dom
import { act, StrictMode, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import { usePreviewFocus } from "./usePreviewFocus";

const roots = new Set<Root>();
afterEach(async () => {
  await act(async () => { for (const root of roots) root.unmount(); });
  roots.clear();
  document.body.replaceChildren();
  vi.restoreAllMocks();
});

async function mount(strict = false) {
  let release!: () => void;
  const held = new Promise<void>((resolve) => { release = resolve; });
  const host = document.createElement("section");
  document.body.append(host);
  const root = createRoot(host);
  roots.add(root);
  let owner = "original-owner";
  let client: object = {};
  let begin!: () => void;
  let settlement: Promise<void> | undefined;
  function Harness({ owner, client }: { owner: string; client: object }) {
    const focus = usePreviewFocus(owner, client);
    const [busy, setBusy] = useState(false);
    begin = () => {
      const ticket = focus.begin(host.querySelector("button")!);
      setBusy(true);
      settlement = held.then(() => {
        focus.settle(ticket);
        setBusy(false);
      });
    };
    return <button type="button" disabled={busy}>Preview</button>;
  }
  async function render() {
    await act(async () => {
      const content = <Harness owner={owner} client={client} />;
      root.render(strict ? <StrictMode>{content}</StrictMode> : content);
    });
  }
  await render();
  const trigger = host.querySelector("button")!;
  return {
    host, trigger,
    async start() {
      trigger.focus();
      expect(document.activeElement).toBe(trigger);
      await act(async () => { begin(); });
      expect(trigger.disabled).toBe(true);
      // happy-dom's blur() refuses disabled controls, unlike the native transition modeled here.
      // Briefly enable only to deliver that DOM blur; React's pending busy state remains true.
      trigger.disabled = false;
      try { trigger.blur(); } finally { trigger.disabled = true; }
      expect(trigger.disabled).toBe(true);
      expect(document.activeElement).toBe(document.body);
    },
    async settle() {
      expect(settlement).toBeDefined();
      await act(async () => { release(); await settlement; });
      expect(trigger.disabled).toBe(false);
    },
    async replace(kind: "owner" | "client") {
      if (kind === "owner") owner = "successor-owner";
      else client = {};
      await render();
      expect(host.querySelector("button")).toBe(trigger);
      expect(trigger.isConnected).toBe(true);
      expect(trigger.disabled).toBe(true);
      expect(document.activeElement).toBe(document.body);
    },
    async unmount() {
      await act(async () => root.unmount());
      roots.delete(root);
    },
  };
}

describe("preview focus hook (DOM contract, not native acceptance)", () => {
  it("restores the exact existing trigger only after held settlement re-enables it", async () => {
    const fixture = await mount();
    const focus = vi.spyOn(fixture.trigger, "focus");
    await fixture.start();
    focus.mockClear();
    expect(focus).not.toHaveBeenCalled();
    expect(fixture.host.querySelectorAll("button")).toHaveLength(1);
    expect(fixture.host.textContent).not.toContain("Apply");
    await fixture.settle();
    expect(focus).toHaveBeenCalledOnce();
    expect(fixture.host.querySelector("button")).toBe(fixture.trigger);
    expect(document.activeElement).toBe(fixture.trigger);
  });

  it("withdraws after outside nonfocusable pointerdown even when body stays active", async () => {
    const fixture = await mount();
    await fixture.start();
    const outside = document.createElement("div");
    document.body.append(outside);
    outside.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
    expect(document.activeElement).toBe(document.body);
    const focus = vi.spyOn(fixture.trigger, "focus");
    await fixture.settle();
    expect(focus, "outside pointerdown must permanently withdraw focus").not.toHaveBeenCalled();
    expect(document.activeElement).toBe(document.body);
  });

  it("withdraws after external focus then blur back to body before settlement", async () => {
    const fixture = await mount();
    await fixture.start();
    const outside = document.createElement("button");
    document.body.append(outside);
    outside.focus();
    expect(document.activeElement).toBe(outside);
    outside.blur();
    expect(document.activeElement).toBe(document.body);
    const focus = vi.spyOn(fixture.trigger, "focus");
    await fixture.settle();
    expect(focus, "external focus must remain withdrawn after returning to body").not.toHaveBeenCalled();
    expect(document.activeElement).toBe(document.body);
  });

  it.each(["owner", "client"] as const)("retires an old ticket after same-node %s replacement without outside focus", async (kind) => {
    const fixture = await mount();
    await fixture.start();
    await fixture.replace(kind);
    const focus = vi.spyOn(fixture.trigger, "focus");
    await fixture.settle();
    expect(focus, "old ticket must not focus the retained successor node").not.toHaveBeenCalled();
    expect(fixture.trigger.isConnected).toBe(true);
    expect(document.activeElement).toBe(document.body);
  });

  it.each([false, true])("disposes each pending listener on real unmount (StrictMode=%s)", async (strict) => {
    const fixture = await mount(strict);
    // Observe only listeners registered by begin, after React's global setup.
    const documentAdd = vi.spyOn(document, "addEventListener");
    const documentRemove = vi.spyOn(document, "removeEventListener");
    const windowAdd = vi.spyOn(window, "addEventListener");
    const windowRemove = vi.spyOn(window, "removeEventListener");
    await fixture.start();
    const types = ["focusin", "pointerdown", "keydown"];
    const pendingDocument = documentAdd.mock.calls.filter(([type]) => types.includes(type));
    const pendingWindow = windowAdd.mock.calls.filter(([type]) => type === "blur");
    expect(pendingDocument.map(([type]) => type).sort()).toEqual([...types].sort());
    expect(pendingWindow).toHaveLength(1);
    for (const [type, callback] of pendingDocument) {
      expect(documentRemove.mock.calls.filter(([t, cb]) => t === type && cb === callback)).toHaveLength(0);
    }
    for (const [type, callback] of pendingWindow) {
      expect(windowRemove.mock.calls.filter(([t, cb]) => t === type && cb === callback)).toHaveLength(0);
    }
    // Do not settle first: this must witness pending unmount, not settled disposal.
    await fixture.unmount();
    for (const [type, callback, capture] of pendingDocument) {
      const disposed = documentRemove.mock.calls.filter(([t, cb]) => t === type && cb === callback);
      expect(disposed, `pending ${type} callback must be disposed on unmount`).toEqual([[type, callback, capture]]);
    }
    for (const [type, callback] of pendingWindow) {
      expect(windowRemove.mock.calls.filter(([t, cb]) => t === type && cb === callback),
        "pending window blur callback must be disposed on unmount").toEqual([[type, callback]]);
    }
  });

  it.each(["inert", "aria-hidden"])("never calls focus under a %s-only ancestor", async (attribute) => {
    const fixture = await mount();
    await fixture.start();
    fixture.host.setAttribute(attribute, attribute === "aria-hidden" ? "true" : "");
    expect(fixture.host.hasAttribute("hidden")).toBe(false);
    expect(fixture.trigger.closest("[hidden]")).toBeNull();
    expect(document.activeElement).toBe(document.body);
    const focus = vi.spyOn(fixture.trigger, "focus");
    await fixture.settle();
    expect(focus, `${attribute}-only ancestry must refuse the focus call`).not.toHaveBeenCalled();
    expect(document.activeElement).toBe(document.body);
  });
});
