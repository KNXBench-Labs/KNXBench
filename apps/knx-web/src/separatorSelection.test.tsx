/** Separator drags must never start a document text selection or a native drag (WP-11 defect). */
// @vitest-environment happy-dom
//
// The 28 October 2026 native WP-11 run found that a pane separator drag in
// WebKitGTK could select document text; later separator drags then lost their
// moves and never committed. The table column handle, which never lost a drag,
// prevents the pointerdown default, blocks dragstart and sets user-select: none.
// These tests pin the same guards on both pane separators, plus clearing an
// existing selection, which the cancelled default would otherwise keep.
import { readFileSync } from "node:fs";
import { act, useRef, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, it } from "vitest";
import PaneSplitter from "./PaneSplitter";
import ResizablePane from "./ResizablePane";
import { parseRules } from "./themeTokens";

let host: HTMLDivElement | undefined;
let root: Root | undefined;

afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  root = undefined;
  host?.remove();
  host = undefined;
});

function Splitter() {
  const [value, setValue] = useState<number | null>(200);
  const target = useRef<HTMLDivElement | null>(null);
  return <>
    <div ref={target} style={{ height: value ?? undefined }} />
    <PaneSplitter label="Block height" target={target} resizes="above" value={value} onChange={setValue} min={72} max={480} />
  </>;
}

async function mount(kind: "pane" | "splitter"): Promise<HTMLElement> {
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  await act(async () => root!.render(kind === "pane"
    ? <ResizablePane label="Navigation width" side="left" initialWidth={300} min={200} max={600}><p>Project status</p></ResizablePane>
    : <Splitter />));
  return host.querySelector<HTMLElement>(kind === "pane" ? ".pane-resizer" : ".pane-splitter")!;
}

async function dispatch(target: HTMLElement, event: Event): Promise<Event> {
  await act(async () => { target.dispatchEvent(event); });
  return event;
}

const press = (button: number) =>
  new PointerEvent("pointerdown", { bubbles: true, cancelable: true, button, clientX: 300, clientY: 300 });

for (const kind of ["pane", "splitter"] as const) {
  it(`${kind}: a primary press prevents the default selection start and focuses the separator`, async () => {
    const separator = await mount(kind);
    const event = await dispatch(separator, press(0));
    expect(event.defaultPrevented).toBe(true);
    expect(document.activeElement).toBe(separator);
  });

  it(`${kind}: a primary press clears a selection left from earlier text selection`, async () => {
    const separator = await mount(kind);
    const text = document.createElement("p");
    text.textContent = "Project status";
    document.body.append(text);
    try {
      window.getSelection()!.selectAllChildren(text);
      expect(window.getSelection()!.toString()).toBe("Project status");
      await dispatch(separator, press(0));
      expect(window.getSelection()!.toString()).toBe("");
    } finally {
      text.remove();
    }
  });

  it(`${kind}: arrow keys still resize after a pointer press focused the separator`, async () => {
    const separator = await mount(kind);
    await dispatch(separator, press(0));
    await dispatch(separator, new PointerEvent("pointerup", { bubbles: true, button: 0, clientX: 300, clientY: 300 }));
    const before = Number(separator.getAttribute("aria-valuenow"));
    const key = kind === "pane" ? "ArrowRight" : "ArrowDown";
    await dispatch(separator, new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }));
    expect(Number(separator.getAttribute("aria-valuenow"))).not.toBe(before);
  });

  it(`${kind}: a secondary press keeps its default (context menus stay native)`, async () => {
    const separator = await mount(kind);
    expect((await dispatch(separator, press(2))).defaultPrevented).toBe(false);
  });

  it(`${kind}: a native dragstart from the separator is cancelled`, async () => {
    const separator = await mount(kind);
    const event = await dispatch(separator, new Event("dragstart", { bubbles: true, cancelable: true }));
    expect(event.defaultPrevented).toBe(true);
  });
}

it("both separators declare user-select: none in the stylesheet", () => {
  const rules = parseRules(readFileSync("src/styles.css", "utf8"));
  for (const selector of [".pane-resizer", ".pane-splitter"]) {
    const declarations = rules
      .filter((rule) => rule.depth === 0 && rule.selector.split(",").map((part) => part.trim()).includes(selector))
      .flatMap((rule) => rule.declarations);
    expect(declarations.some((d) => d.property === "user-select" && d.value === "none"), selector).toBe(true);
  }
});
