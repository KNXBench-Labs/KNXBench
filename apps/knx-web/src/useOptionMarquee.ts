/** WP-47.1: measures a picker row's text overflow so it can marquee on hover. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { useLayoutEffect, useRef, useState } from "react";

export function useOptionMarquee<T extends HTMLElement>(text: string, active = true) {
  const rowRef = useRef<T>(null);
  const meterRef = useRef<HTMLSpanElement>(null);
  const [distance, setDistance] = useState(0);
  useLayoutEffect(() => {
    const row = rowRef.current;
    const meter = meterRef.current;
    if (!active || !row || !meter) { if (active) setDistance(0); return; }
    meter.textContent = text;
    const styles = getComputedStyle(row);
    meter.style.fontFamily = styles.fontFamily;
    meter.style.fontSize = styles.fontSize;
    meter.style.fontWeight = styles.fontWeight;
    meter.style.letterSpacing = styles.letterSpacing;
    const overflow = meter.offsetWidth - row.clientWidth;
    setDistance(overflow > 0 ? overflow : 0);
  }, [text, active]);
  return { rowRef, meterRef, distance, marquee: distance > 0 };
}
