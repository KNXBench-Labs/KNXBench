/** KL-60: which rows of a long variable-height list a scroll viewport has to render. */

export interface VirtualWindow {
  /** First rendered row (inclusive). */
  start: number;
  /** Row after the last rendered one (exclusive). */
  end: number;
  /** Height of the skipped rows above the window, in px. */
  before: number;
  /** Height of the skipped rows below the window, in px. */
  after: number;
}

/**
 * The rows that intersect `[scrollTop - overscan, scrollTop + viewport + overscan]`,
 * given each row's height (measured where known, estimated otherwise). A
 * scroll position past the end is clamped, so the last rows stay reachable.
 * `before + rendered heights + after` always equals the full list height.
 */
export function virtualWindow(
  heights: readonly number[],
  scrollTop: number,
  viewportHeight: number,
  overscan: number,
): VirtualWindow {
  const offsets = new Array<number>(heights.length + 1);
  offsets[0] = 0;
  for (let i = 0; i < heights.length; i++) offsets[i + 1] = offsets[i] + heights[i];
  const total = offsets[heights.length];
  const top = Math.min(Math.max(0, scrollTop), Math.max(0, total - viewportHeight));
  const from = top - overscan;
  const to = top + viewportHeight + overscan;
  let start = 0;
  while (start < heights.length && offsets[start + 1] <= from) start++;
  let end = start;
  while (end < heights.length && offsets[end] < to) end++;
  return { start, end, before: offsets[start], after: total - offsets[end] };
}
