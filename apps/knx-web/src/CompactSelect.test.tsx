/** CompactSelect: measurement lifecycle, tooltip, hidden reveal overlay, forwarded handlers. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import CompactSelect from "./CompactSelect";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let host: HTMLDivElement | null = null;
let root: Root | null = null;
/** Layout is absent in happy-dom: the select is 160 px wide, the hidden twin as wide as this. */
let textWidth = 300;
let observed: (() => void) | null = null;

beforeEach(() => {
  textWidth = 300;
  observed = null;
  vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
    return { width: this.classList.contains("select-compact-measure") ? textWidth : 0 } as DOMRect;
  });
  vi.spyOn(HTMLElement.prototype, "clientWidth", "get").mockImplementation(function (this: HTMLElement) {
    return this.tagName === "SELECT" ? 160 : 0;
  });
  vi.stubGlobal("ResizeObserver", class { constructor(cb: () => void) { observed = cb; } observe() {} disconnect() {} });
});
afterEach(() => {
  act(() => root?.unmount());
  root = null;
  host?.remove();
  host = null;
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});

function renderSelect(value: string, extra: Record<string, unknown> = {}) {
  host = document.createElement("div");
  document.body.appendChild(host);
  root = createRoot(host);
  const ui = (v: string) => (
    <CompactSelect value={v} aria-label="demo" onChange={() => {}} {...extra}>
      <option value="short">Short</option>
      <option value="long">A very long option label that cannot fit</option>
    </CompactSelect>
  );
  act(() => root!.render(ui(value)));
  return { select: host.querySelector("select")!, rerender: (v: string) => act(() => root!.render(ui(v))) };
}
const wrapper = () => host!.querySelector(".select-compact")!;
const overlay = () => host!.querySelector(".select-compact-reveal");

describe("CompactSelect", () => {
  it("measures at mount: title and overflow are right before any hover or focus", () => {
    const { select } = renderSelect("long");
    expect(select.title).toBe("A very long option label that cannot fit");
    expect(wrapper().classList.contains("is-marquee")).toBe(true);
    // Overflow = twin width 300 - (select 160 - arrow reserve 24) with zero padding.
    expect((wrapper() as HTMLElement).style.getPropertyValue("--select-marquee-distance")).toBe("164px");
    expect(overlay()?.getAttribute("aria-hidden")).toBe("true");
    expect(overlay()?.textContent).toBe("A very long option label that cannot fit");
  });

  it("leaves a fitting value plain: no overlay, no marquee class", () => {
    textWidth = 100;
    const { select } = renderSelect("short");
    expect(select.title).toBe("Short");
    expect(wrapper().classList.contains("is-marquee")).toBe(false);
    expect(overlay()).toBeNull();
  });

  it("re-measures on a value change and on a resize without re-render", () => {
    textWidth = 100;
    const { select, rerender } = renderSelect("short");
    expect(overlay()).toBeNull();
    textWidth = 300;
    rerender("long");
    expect(select.title).toBe("A very long option label that cannot fit");
    expect(overlay()?.textContent).toBe("A very long option label that cannot fit");
    textWidth = 120; // e.g. the container grew; the observer, not a render, reports it
    act(() => observed?.());
    expect(overlay()).toBeNull();
  });

  it("never overlays a disabled select and keeps an explicit title", () => {
    const { select } = renderSelect("long", { disabled: true, title: "Explicit" });
    expect(overlay()).toBeNull();
    expect(select.title).toBe("Explicit");
  });

  it("forwards every handler, including mouse leave, and keeps the native select unhidden", () => {
    const calls: string[] = [];
    const { select } = renderSelect("long", {
      onMouseEnter: () => calls.push("enter"), onMouseLeave: () => calls.push("leave"),
      onFocus: () => calls.push("focus"), onBlur: () => calls.push("blur"), onKeyDown: () => calls.push("key"),
    });
    act(() => {
      select.dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
      select.dispatchEvent(new MouseEvent("mouseout", { bubbles: true }));
      select.focus();
      select.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, key: "ArrowDown" }));
      select.blur();
    });
    expect(calls).toEqual(["enter", "leave", "focus", "key", "blur"]);
    expect(select.getAttribute("aria-hidden")).toBeNull();
    expect(select.tabIndex).toBe(0);
  });

  it("keeps change events flowing from the native select", () => {
    let seen = "";
    const { select } = renderSelect("short", { onChange: (e: { target: HTMLSelectElement }) => { seen = e.target.value; } });
    const setter = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, "value")!.set!;
    act(() => {
      setter.call(select, "long");
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });
    expect(seen).toBe("long");
  });
});
