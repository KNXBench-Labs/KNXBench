/** Suggests a range's next free group address (issue #20); the server still decides. */
// The suggestion is presentation only: `CreateGroupAddress` in the core
// refuses a duplicate in the installation and an address outside its range,
// so a stale or wrong suggestion can never be written.

const LEVELS: Record<string, readonly number[]> = {
  // Bits per level, most significant first; each style splits the same 16 bits.
  ThreeLevel: [5, 3, 8],
  TwoLevel: [5, 11],
  Free: [16],
};

/** The 16-bit value of `text` in the project's `style`, or `null` if it is not one. */
export function groupAddressRaw(text: string, style: string): number | null {
  const bits = LEVELS[style];
  if (!bits) return null;
  const parts = text.split("/");
  if (parts.length !== bits.length) return null;
  let raw = 0;
  for (const [index, part] of parts.entries()) {
    if (!/^\d+$/.test(part)) return null;
    const value = Number(part);
    if (value >= 2 ** bits[index]) return null;
    raw = raw * 2 ** bits[index] + value;
  }
  return raw;
}

/** `raw` written in the project's `style`, the way the tree shows addresses. */
export function formatGroupAddressRaw(raw: number, style: string): string {
  const bits = LEVELS[style] ?? LEVELS.Free;
  const parts: number[] = [];
  let rest = raw;
  for (const width of [...bits].reverse()) {
    parts.unshift(rest % 2 ** width);
    rest = Math.floor(rest / 2 ** width);
  }
  return parts.join("/");
}

/**
 * The address a new group address in `range` should get, as the reporter of
 * issue #20 asked for:
 *
 * 1. the first gap in the range's own sequence (between its lowest and
 *    highest existing address), else
 * 2. the address right after the highest one, else (the range end is reached)
 * 3. the lowest free address of the range;
 * 4. `null` when the range is full or unreadable.
 *
 * An empty range starts at its first address. `used` is every address of the
 * installation: one inside the range is taken whether or not it is filed
 * under this range. Raw 0 (`0/0/0`) is never suggested: a group destination
 * of 0 is the KNX broadcast address.
 */
export function nextFreeGroupAddress(
  range: { start: string; end: string },
  used: readonly string[],
  style: string,
): string | null {
  const start = groupAddressRaw(range.start, style);
  const end = groupAddressRaw(range.end, style);
  if (start === null || end === null || start > end) return null;
  const lowest = Math.max(start, 1);
  const taken = new Set<number>();
  for (const address of used) {
    const raw = groupAddressRaw(address, style);
    if (raw !== null && raw >= start && raw <= end) taken.add(raw);
  }
  const free = (raw: number) => raw >= lowest && raw <= end && !taken.has(raw);
  const sequence = [...taken].sort((a, b) => a - b);
  const result = (raw: number) => formatGroupAddressRaw(raw, style);
  if (sequence.length === 0) return lowest <= end ? result(lowest) : null;
  const last = sequence[sequence.length - 1];
  for (let raw = Math.max(sequence[0], lowest); raw <= last; raw++) {
    if (free(raw)) return result(raw);
  }
  if (free(last + 1)) return result(last + 1);
  for (let raw = lowest; raw <= end; raw++) {
    if (free(raw)) return result(raw);
  }
  return null;
}
