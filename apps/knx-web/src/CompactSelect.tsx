/** Bounded select (WP-41 scalar slot) that reveals an overflowing value in a stationary overlay. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { useLayoutEffect, useRef, useState, type CSSProperties, type ReactNode, type SelectHTMLAttributes } from "react";

/** Width a native select keeps free on the right for its dropdown arrow. */
const ARROW_RESERVE_PX = 24;

interface Reveal {
  text: string;
  /** How far the text must travel to show its tail; 0 when the value fits. */
  distance: number;
  /** Left inset and width of the select's text area, in px. */
  left: number;
  width: number;
  fontFamily: string;
  fontSize: string;
  fontWeight: string;
  letterSpacing: string;
}

const EMPTY: Reveal = { text: "", distance: 0, left: 0, width: 0, fontFamily: "", fontSize: "", fontWeight: "", letterSpacing: "" };

function sameReveal(a: Reveal, b: Reveal): boolean {
  return (Object.keys(a) as (keyof Reveal)[]).every((key) => a[key] === b[key]);
}

/**
 * A `<select>` capped at the shared WP-41 scalar value slot (10 rem). It stays
 * the one native control: hit box, border, arrow, keyboard and screen-reader
 * behaviour never change. Text a native select clips cannot be scrolled from
 * outside, so while the pointer rests on it (or it has keyboard focus) and
 * motion is allowed, an `aria-hidden` overlay laid over the select's text
 * area (the select's own text turns transparent meanwhile) scrolls the
 * complete label. Everywhere else (reduced motion, motion "off") the control
 * stays static and the full value is reachable through the `title` tooltip
 * and the native option list. The overlay is measured after every commit and
 * whenever the select is resized.
 */
export default function CompactSelect(
  props: SelectHTMLAttributes<HTMLSelectElement> & { children: ReactNode },
) {
  const { children, title, ...rest } = props;
  const selectRef = useRef<HTMLSelectElement>(null);
  const measureRef = useRef<HTMLSpanElement>(null);
  const [reveal, setReveal] = useState<Reveal>(EMPTY);

  function measure() {
    const select = selectRef.current;
    const meter = measureRef.current;
    if (!select || !meter) return;
    const text = select.options[select.selectedIndex]?.text ?? "";
    const styles = getComputedStyle(select);
    meter.textContent = text;
    // The select's own text lives in the UA shadow DOM and cannot be
    // measured, so a hidden twin in the same font stands in for it.
    meter.style.fontFamily = styles.fontFamily;
    meter.style.fontSize = styles.fontSize;
    meter.style.fontWeight = styles.fontWeight;
    meter.style.letterSpacing = styles.letterSpacing;
    const left = (parseFloat(styles.paddingLeft) || 0) + (parseFloat(styles.borderLeftWidth) || 0);
    const width = Math.max(0, select.clientWidth - (parseFloat(styles.paddingLeft) || 0) - ARROW_RESERVE_PX);
    // Sub-pixel width: `offsetWidth` rounds and would leave the last glyph clipped.
    const overflow = meter.getBoundingClientRect().width - width;
    const next: Reveal = {
      text, distance: overflow > 0 ? overflow : 0, left, width,
      fontFamily: styles.fontFamily, fontSize: styles.fontSize, fontWeight: styles.fontWeight, letterSpacing: styles.letterSpacing,
    };
    setReveal((current) => (sameReveal(current, next) ? current : next));
  }

  // Mount and every later commit (value or option labels may have changed),
  // so the tooltip and overflow are right before any hover.
  useLayoutEffect(measure);
  // Width changes without a re-render: container resize, UI zoom, font load.
  useLayoutEffect(() => {
    const select = selectRef.current;
    if (!select || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(() => measure());
    observer.observe(select);
    return () => observer.disconnect();
  }, []);

  const overflowing = reveal.distance > 0 && !rest.disabled;
  const wrapperStyle = overflowing ? {
    "--select-marquee-distance": `${reveal.distance}px`,
    "--select-reveal-left": `${reveal.left}px`,
    "--select-reveal-width": `${reveal.width}px`,
  } as CSSProperties : undefined;
  return (
    <span className={overflowing ? "select-compact is-marquee" : "select-compact"} style={wrapperStyle}>
      <select ref={selectRef} title={title ?? (reveal.text || undefined)} {...rest}>
        {children}
      </select>
      <span ref={measureRef} className="select-compact-measure" aria-hidden="true" />
      {overflowing ? (
        <span className="select-compact-reveal" aria-hidden="true">
          <span style={{ fontFamily: reveal.fontFamily, fontSize: reveal.fontSize, fontWeight: reveal.fontWeight, letterSpacing: reveal.letterSpacing }}>{reveal.text}</span>
        </span>
      ) : null}
    </span>
  );
}
