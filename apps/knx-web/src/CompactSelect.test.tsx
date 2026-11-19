/** WP-45: CompactSelect bounds the select to the scalar slot and marquees on hover. */
// @vitest-environment happy-dom
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, describe, expect, it } from "vitest";
import CompactSelect from "./CompactSelect";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

let host: HTMLDivElement | null = null;
let root: Root | null = null;
afterEach(() => {
  act(() => root?.unmount());
  root = null;
  host?.remove();
  host = null;
});

function renderSelect(props?: { value?: string; long?: string }) {
  host = document.createElement("div");
  document.body.appendChild(host);
  root = createRoot(host);
  const long = props?.long ?? "A very long option label that cannot fit";
  act(() => root!.render(
    <CompactSelect value={props?.value ?? "short"} aria-label="demo">
      <option value="short">Short</option>
      <option value="long">{long}</option>
    </CompactSelect>,
  ));
  return host.querySelector("select")!;
}

describe("CompactSelect (WP-45)", () => {
  it("renders a native select with the full value in the title", () => {
    const select = renderSelect({ value: "long", long: "DPST-20-102 – Long label" });
    expect(select.value).toBe("long");
    // The title is set from the measured selected option on interaction; force
    // it by focusing, which every real user path (hover/keyboard) also does.
    act(() => { select.focus(); });
    expect(select.title).toBe("DPST-20-102 – Long label");
    expect(select.querySelectorAll("option")).toHaveLength(2);
    // The wrapper carries the shared class; the hidden measuring span is aria-hidden.
    expect(host!.querySelector(".select-compact")).not.toBeNull();
    expect(host!.querySelector(".select-compact-measure")?.getAttribute("aria-hidden")).toBe("true");
  });

  it("exposes the overflow as a marquee class driven by the measured distance", () => {
    renderSelect({ value: "long" });
    const wrapper = host!.querySelector(".select-compact")!;
    const select = host!.querySelector("select")!;
    // No measurement yet: distance is 0, so no marquee class.
    expect(wrapper.classList.contains("is-marquee")).toBe(false);
    // happy-dom reports zero widths; the arrow reserve (24 px) makes the
    // measured overflow positive, proving the class follows the measurement.
    act(() => { select.focus(); }); // focus runs the same measure() as hover
    expect(wrapper.classList.contains("is-marquee")).toBe(true);
    expect(select.getAttribute("style")).toContain("--select-marquee-distance");
  });

  it("keeps keyboard operation on the native select (change events flow)", () => {
    const select = renderSelect();
    const setter = Object.getOwnPropertyDescriptor(HTMLSelectElement.prototype, "value")!.set!;
    let seen = "";
    act(() => root!.render(
      <CompactSelect value="short" aria-label="demo" onChange={(e) => { seen = e.target.value; }}>
        <option value="short">Short</option>
        <option value="long">Long</option>
      </CompactSelect>,
    ));
    const sel = host!.querySelector("select")!;
    act(() => {
      setter.call(sel, "long");
      sel.dispatchEvent(new Event("change", { bubbles: true }));
    });
    expect(seen).toBe("long");
    expect(select).toBeTruthy();
  });
});
