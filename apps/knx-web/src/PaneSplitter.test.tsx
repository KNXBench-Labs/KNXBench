/** Tests for the stacked-pane splitter: its ARIA, its keyboard steps, its drag, and its limits. */
// @vitest-environment happy-dom
//
// T28/F3. The three blocks in the workbench's left column had fixed
// heights; these pin the separator that gives one block's height to its
// neighbour. The component is controlled, so every test drives it through
// the small harness below rather than poking it directly — that is also
// the only way to catch a splitter that moves its `aria-valuenow` without
// ever telling its parent.
import { act, useRef, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import PaneSplitter from "./PaneSplitter";

const MIN = 72;
const MAX = 480;

let host: HTMLDivElement | undefined;
let root: Root | undefined;

afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  root = undefined;
  host?.remove();
  host = undefined;
  vi.restoreAllMocks();
});

function Harness(props: { resizes: "above" | "below"; initial: number | null }) {
  const [value, setValue] = useState<number | null>(props.initial);
  const target = useRef<HTMLDivElement | null>(null);
  return (
    <>
      <div ref={target} data-testid="block" style={{ height: value ?? undefined }} />
      <PaneSplitter
        label="Height of the navigation block"
        target={target}
        resizes={props.resizes}
        value={value}
        onChange={setValue}
        min={MIN}
        max={MAX}
      />
    </>
  );
}

async function mount(resizes: "above" | "below", initial: number | null) {
  host = document.createElement("div");
  document.body.append(host);
  root = createRoot(host);
  await act(async () => root!.render(<Harness resizes={resizes} initial={initial} />));
  return host.querySelector<HTMLElement>(".pane-splitter")!;
}

function now(splitter: HTMLElement): number {
  return Number(splitter.getAttribute("aria-valuenow"));
}

// What the *parent* was handed, as opposed to what the splitter shows.
// The two can disagree: the component clamps the value it renders, so a
// splitter that emitted an out-of-range height would still look correct
// while its parent set a block height below the usable minimum.
function stored(): string {
  return host!.querySelector<HTMLElement>('[data-testid="block"]')!.style.height;
}

async function press(splitter: HTMLElement, key: string) {
  await act(async () => {
    splitter.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
  });
}

async function drag(splitter: HTMLElement, from: number, to: number) {
  await act(async () => {
    splitter.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, button: 0, clientY: from }));
  });
  await act(async () => {
    splitter.dispatchEvent(new PointerEvent("pointermove", { bubbles: true, clientY: to }));
  });
  await act(async () => {
    splitter.dispatchEvent(new PointerEvent("pointerup", { bubbles: true, clientY: to }));
  });
}

it("announces itself as a separator with a label and a full value range", async () => {
  const splitter = await mount("above", 200);

  expect(splitter.getAttribute("role")).toBe("separator");
  expect(splitter.getAttribute("aria-orientation")).toBe("horizontal");
  expect(splitter.getAttribute("aria-label")).toBe("Height of the navigation block");
  expect(splitter.getAttribute("aria-valuemin")).toBe(String(MIN));
  expect(splitter.getAttribute("aria-valuemax")).toBe(String(MAX));
  expect(now(splitter)).toBe(200);
  // Reachable by Tab, which is the whole point of the keyboard handler
  // below having anything to receive.
  expect(splitter.tabIndex).toBe(0);
});

// Before anyone touches it the block keeps the height its content gives
// it, and `aria-valuenow` still has to be a number — the measured one, not
// a constant. happy-dom measures every box as zero, so the measurement is
// stubbed; what is under test is that the component asks at all.
it("reports the block's measured height before it has been moved", async () => {
  vi.spyOn(Element.prototype, "getBoundingClientRect").mockReturnValue(
    { height: 183, width: 0, top: 0, left: 0, right: 0, bottom: 0, x: 0, y: 0, toJSON: () => ({}) },
  );
  const splitter = await mount("above", null);
  expect(now(splitter)).toBe(183);
});

it("grows the block above it downwards and shrinks it upwards, by keyboard", async () => {
  const splitter = await mount("above", 200);

  await press(splitter, "ArrowDown");
  expect(now(splitter)).toBe(216);
  await press(splitter, "ArrowUp");
  await press(splitter, "ArrowUp");
  expect(now(splitter)).toBe(184);
});

// The same two keys, the other block: dragging the lower separator down
// makes the diagnostics block *smaller*, because it is below the line.
it("reverses the arrows for the block below it", async () => {
  const splitter = await mount("below", 200);

  await press(splitter, "ArrowDown");
  expect(now(splitter)).toBe(184);
  await press(splitter, "ArrowUp");
  await press(splitter, "ArrowUp");
  expect(now(splitter)).toBe(216);
});

it("stops at both limits, and Home and End go straight to them", async () => {
  const splitter = await mount("above", MIN + 8);

  await press(splitter, "ArrowUp");
  expect(now(splitter)).toBe(MIN);
  expect(stored()).toBe(`${MIN}px`);
  await press(splitter, "ArrowUp");
  expect(now(splitter)).toBe(MIN);
  expect(stored()).toBe(`${MIN}px`);

  await press(splitter, "End");
  expect(now(splitter)).toBe(MAX);
  await press(splitter, "ArrowDown");
  expect(now(splitter)).toBe(MAX);
  expect(stored()).toBe(`${MAX}px`);

  await press(splitter, "Home");
  expect(now(splitter)).toBe(MIN);
  expect(stored()).toBe(`${MIN}px`);
});

it("ignores keys it does not own, so the rest of the page still gets them", async () => {
  const splitter = await mount("above", 200);

  await press(splitter, "ArrowLeft");
  await press(splitter, "PageDown");
  await press(splitter, "Enter");
  expect(now(splitter)).toBe(200);
});

it("follows a pointer drag, in both directions and within its limits", async () => {
  const splitter = await mount("above", 200);

  await drag(splitter, 300, 340);
  expect(now(splitter)).toBe(240);
  await drag(splitter, 300, 260);
  expect(now(splitter)).toBe(200);
  // Dragged past the floor: the block stops, the pointer does not.
  await drag(splitter, 300, -1000);
  expect(now(splitter)).toBe(MIN);
  expect(stored()).toBe(`${MIN}px`);
});

it("stops following once the pointer is released", async () => {
  const splitter = await mount("above", 200);

  await drag(splitter, 300, 340);
  await act(async () => {
    splitter.dispatchEvent(new PointerEvent("pointermove", { bubbles: true, clientY: 500 }));
  });
  expect(now(splitter)).toBe(240);
});

// A right-click on the separator opens a context menu; it must not also
// start a drag that then follows the cursor around.
it("ignores a pointer-down from a button other than the primary one", async () => {
  const splitter = await mount("above", 200);

  await act(async () => {
    splitter.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, button: 2, clientY: 300 }));
  });
  await act(async () => {
    splitter.dispatchEvent(new PointerEvent("pointermove", { bubbles: true, clientY: 400 }));
  });
  expect(now(splitter)).toBe(200);
});
