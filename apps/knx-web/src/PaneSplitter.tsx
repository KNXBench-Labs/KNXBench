/** Horizontal splitter that gives height back and forth between two stacked blocks of a pane. */
// `ResizablePane.tsx`'s counterpart for the other axis. The two were kept
// as separate components on purpose: `ResizablePane` *is* a pane — it
// renders the `<aside>`, owns its own width, and puts its resizer on one
// edge of itself — while this one is only the separator, placed between
// two siblings that already exist, owning no state and no layout of its
// own. Folding them together would have meant a component that renders a
// wrapper on one axis and nothing on the other.
//
// What is shared is the interaction vocabulary, and that is copied
// deliberately, key for key: `role="separator"` with `aria-orientation`,
// `aria-valuemin`/`max`/`now`, `tabIndex={0}`, arrow keys in 16px steps
// with `Home`/`End` for the limits, and a pointer drag held by pointer
// capture so it survives the cursor leaving the 6px hit area.
import { useLayoutEffect, useRef, useState, type RefObject } from "react";
import { appUiScale } from "./appUiScale";

// Same step `ResizablePane` moves by, so the two separators in the
// workbench feel like one control in two orientations rather than two
// controls with opinions.
const KEYBOARD_STEP = 16;

export default function PaneSplitter(props: {
  label: string;
  /**
   * The block whose height this splitter sets. `resizes` says which side
   * of the separator it is on, which is also the sign of the drag: pulling
   * the separator down grows the block above it and shrinks the block
   * below it.
   */
  target: RefObject<HTMLElement | null>;
  resizes: "above" | "below";
  /** `null` until the user first moves it — see `measured` below. */
  value: number | null;
  onChange: (height: number) => void;
  min: number;
  max: number;
}) {
  const { label, target, resizes, value, onChange, min, max } = props;
  const drag = useRef<{ y: number; height: number; scale: number } | null>(null);
  const clamp = (n: number) => Math.max(min, Math.min(max, n));
  const direction = resizes === "above" ? 1 : -1;

  // Until someone moves it, the block keeps the content height it has
  // always had, and this reads that height off the DOM instead of starting
  // from a constant. A constant would be a guess about how tall five
  // buttons are in the active language and font, and the first paint would
  // clip or gap wherever the guess was wrong. `aria-valuenow` has to be a
  // number from the very first render, so "no value yet" cannot simply be
  // left blank.
  const [measured, setMeasured] = useState<number | null>(null);
  useLayoutEffect(() => {
    if (value !== null) return;
    const element = target.current;
    if (!element) return;
    const measure = () => {
      // DOMRect is visual pixels under application zoom; heights are layout.
      const height = element.getBoundingClientRect().height / appUiScale();
      if (height > 0) setMeasured(height);
    };
    measure();
    // A folded navigation section has no measurable box until it opens.
    const observer = new ResizeObserver(measure);
    observer.observe(element);
    return () => observer.disconnect();
  }, [value, target]);

  const current = clamp(value ?? measured ?? min);
  // Read at interaction time too: a first input can precede ResizeObserver.
  const interactionHeight = () => {
    const height = (target.current?.getBoundingClientRect().height ?? 0) / appUiScale();
    return value === null && height > 0 ? clamp(height) : current;
  };

  return (
    <div
      role="separator"
      aria-label={label}
      aria-orientation="horizontal"
      aria-valuemin={min}
      aria-valuemax={max}
      aria-valuenow={current}
      tabIndex={0}
      className="pane-splitter"
      onKeyDown={(e) => {
        if (!["ArrowUp", "ArrowDown", "Home", "End"].includes(e.key)) return;
        e.preventDefault();
        onChange(
          e.key === "Home"
            ? min
            : e.key === "End"
              ? max
              : clamp(interactionHeight() + (e.key === "ArrowDown" ? KEYBOARD_STEP : -KEYBOARD_STEP) * direction),
        );
      }}
      onDragStart={(e) => e.preventDefault()}
      onPointerDown={(e) => {
        if (e.button !== 0) return;
        // As in ResizablePane: no text selection may start from or survive a drag.
        e.preventDefault();
        window.getSelection()?.removeAllRanges();
        e.currentTarget.focus();
        drag.current = { y: e.clientY, height: interactionHeight(), scale: appUiScale() };
        // Guarded rather than assumed: a pointer event can arrive without
        // a capture API behind it (a synthesised one in a test, an older
        // engine), and a drag that cannot be captured is still a drag
        // worth following.
        e.currentTarget.setPointerCapture?.(e.pointerId);
      }}
      onPointerMove={(e) => {
        if (!drag.current) return;
        onChange(clamp(drag.current.height + (e.clientY - drag.current.y) / drag.current.scale * direction));
      }}
      onPointerUp={() => {
        drag.current = null;
      }}
      onPointerCancel={() => {
        drag.current = null;
      }}
      onLostPointerCapture={() => {
        drag.current = null;
      }}
    />
  );
}
