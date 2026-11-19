/** WP-45: bounded select (WP-41 scalar slot) with a hover marquee on overflow. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import React, { useRef, useState, type ReactNode, type SelectHTMLAttributes } from "react";

/**
 * A `<select>` capped at the shared WP-41 scalar value slot (10 rem, about
 * 20–25 monospace characters). The full selected value stays reachable
 * through the `title` tooltip and the native option list; when the value
 * does not fit, hovering starts a slow marquee scroll (CSS, gated on the
 * motion level and `prefers-reduced-motion` in styles.css). Keyboard and
 * screen-reader access are untouched: this renders the same native select,
 * only wrapped in a clipping span.
 */
export default function CompactSelect(
  props: SelectHTMLAttributes<HTMLSelectElement> & { children: ReactNode },
) {
  const { children, onChange, onMouseEnter, onMouseLeave, onFocus, onBlur, ...rest } = props;
  const selectRef = useRef<HTMLSelectElement>(null);
  const measureRef = useRef<HTMLSpanElement>(null);
  const [distance, setDistance] = useState(0);
  // Measure after mount too: the first render has no layout yet, and a value
  // that overflows on load should already carry its title before any hover.

  // The select's own text lives in the UA shadow DOM and is not measurable,
  // so the selected option's label is measured in a hidden span with the
  // same font. `distance` is the overflow in px, or 0 when the value fits.
  // Read on every render so a changed value refreshes the tooltip; the ref is
  // only set after mount, hence the state-driven title below.
  const [title, setTitle] = useState<string | undefined>(undefined);
  function measure() {
    const select = selectRef.current;
    const meter = measureRef.current;
    if (!select || !meter) return;
    const text = select.selectedOptions[0]?.text ?? "";
    setTitle(text);
    meter.textContent = text;
    const styles = getComputedStyle(select);
    // The measure span inherits the wrapper's font; copy the select's own
    // font onto it so the measurement matches what the select would render.
    meter.style.fontFamily = styles.fontFamily;
    meter.style.fontSize = styles.fontSize;
    meter.style.fontWeight = styles.fontWeight;
    meter.style.letterSpacing = styles.letterSpacing;
    // A native select clips its rendered value and reserves ~24 px for its
    // dropdown arrow, so the authoritative measurement is the hidden span's
    // text width (in the select's own font) against the client width minus
    // that arrow reserve: only genuine text overflow engages the marquee.
    const overflow = meter.offsetWidth - (select.clientWidth - 24);
    setDistance(overflow > 0 ? overflow : 0);
  }

  // The marquee class marks the value as overflowing; the animation itself is
  // hover/focus-scoped in CSS, so the scroll runs only while hovered/focused.
  const marquee = distance > 0;
  return (
    <span className={marquee ? "select-compact is-marquee" : "select-compact"}>
      <select
        ref={selectRef}
        title={title}
        style={marquee ? { "--select-marquee-distance": `${distance}px` } as React.CSSProperties : undefined}
        onChange={(e) => { measure(); onChange?.(e); }}
        onMouseEnter={(e) => { measure(); onMouseEnter?.(e); }}
        onFocus={(e) => { measure(); onFocus?.(e); }}
        onBlur={(e) => { onBlur?.(e); }}
        {...rest}
      >
        {children}
      </select>
      <span ref={measureRef} className="select-compact-measure" aria-hidden="true" />
    </span>
  );
}
