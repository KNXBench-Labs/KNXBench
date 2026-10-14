/** Timing, gate and row choice for the explorer's structure-line pulse (owner feedback P2). */
// The pulse is a one-shot CSS animation on rows that carry `data-tree-pulse-row`.
// Only this hook sets that attribute, and only on the rows visible when a pulse
// starts: a project with thousands of rows animates a screenful, not thousands.
import { useEffect, useState, type RefObject } from "react";

/** About every twelve seconds a pulse spreads from the root to the leaves. */
export const TREE_PULSE_INTERVAL_MS = 12_000;
/** Long enough for the deepest level's delayed pulse to finish (see styles.css). */
export const TREE_PULSE_WINDOW_MS = 2_400;
/** The row attribute the pulse animation keys on. */
export const TREE_PULSE_ROW = "data-tree-pulse-row";

/**
 * Pulses only where they are seen and wanted: a visible tab whose effective
 * structure-line motion is standard. Subtle grows the lines but never pulses;
 * off and the operating system's reduced motion stop everything in CSS.
 */
export function treePulseAllowed(doc: Pick<Document, "hidden" | "documentElement">): boolean {
  return !doc.hidden && doc.documentElement.getAttribute("data-motion-lines") === "standard";
}

type Span = { top: number; bottom: number };

/** Indexes of the rows that overlap the visible span (both in viewport pixels). */
export function visibleRowIndexes(rows: readonly Span[], view: Span): number[] {
  const result: number[] = [];
  rows.forEach((row, index) => {
    if (row.bottom > view.top && row.top < view.bottom) result.push(index);
  });
  return result;
}

/** Marks the visible nested rows below `container` and returns how to unmark them. */
function markVisibleRows(container: HTMLElement): () => void {
  const rows = [...container.querySelectorAll<HTMLElement>(".tree-root li li")];
  const box = container.getBoundingClientRect();
  const view = { top: Math.max(box.top, 0), bottom: Math.min(box.bottom, window.innerHeight) };
  const spans = rows.map((row) => (row.querySelector(".tree-row") ?? row).getBoundingClientRect());
  const marked = visibleRowIndexes(spans, view).map((index) => rows[index]);
  for (const row of marked) row.setAttribute(TREE_PULSE_ROW, "");
  return () => { for (const row of marked) row.removeAttribute(TREE_PULSE_ROW); };
}

/**
 * Whether a pulse is running. While it is, the nested rows that were visible
 * below `container` when it started carry {@link TREE_PULSE_ROW}.
 */
export function useTreePulse(container?: RefObject<HTMLElement | null>): boolean {
  const [pulsing, setPulsing] = useState(false);
  useEffect(() => {
    let end: number | undefined;
    let unmark = () => {};
    const tick = window.setInterval(() => {
      if (!treePulseAllowed(document)) return;
      window.clearTimeout(end);
      unmark();
      unmark = container?.current ? markVisibleRows(container.current) : () => {};
      setPulsing(true);
      end = window.setTimeout(() => {
        unmark();
        unmark = () => {};
        setPulsing(false);
      }, TREE_PULSE_WINDOW_MS);
    }, TREE_PULSE_INTERVAL_MS);
    return () => {
      window.clearInterval(tick);
      window.clearTimeout(end);
      unmark();
    };
  }, [container]);
  return pulsing;
}
