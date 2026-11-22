/** Exact chronological ordering of RFC 3339 timestamps, including sub-millisecond fractions. */
// SPDX-License-Identifier: AGPL-3.0-or-later

/**
 * An instant as whole UTC seconds plus the fractional-second digits. The
 * server emits 0, 3, 6 or 9 fractional digits; `Date.parse` would drop
 * everything below a millisecond and a string compare reads `.100` before
 * `.099999`. Policy: every fractional digit is significant, with no
 * precision ceiling; fractions of different width compare as if right-padded
 * with zeros, so `.1`, `.100` and `.100000000` are the same instant.
 */
export interface Instant {
  seconds: number;
  /** Fractional digits with trailing zeros removed ("" for a whole second). */
  fraction: string;
}

const RFC3339 = /^(\d{4})-(\d{2})-(\d{2})[Tt](\d{2}):(\d{2}):(\d{2})(?:\.(\d+))?([Zz]|[+-]\d{2}:\d{2})$/;

/** `null` when the text is not a valid RFC 3339 timestamp with an explicit offset. */
export function parseInstant(text: string): Instant | null {
  const m = RFC3339.exec(text);
  if (!m) return null;
  const [year, month, day, hour, minute, second] = m.slice(1, 7).map(Number);
  if (month < 1 || month > 12 || day < 1 || hour > 23 || minute > 59 || second > 60) return null;
  const date = new Date(Date.UTC(year, month - 1, day, hour, minute, Math.min(second, 59)));
  // Refuse calendar dates `Date.UTC` would silently roll over (31 February).
  if (date.getUTCMonth() !== month - 1 || date.getUTCDate() !== day) return null;
  // A leap second (:60) is placed one second after :59, i.e. equal to the next :00.
  const utc = date.getTime() + (second === 60 ? 1000 : 0);
  let offsetSeconds = 0;
  const zone = m[8];
  if (zone !== "Z" && zone !== "z") {
    const sign = zone[0] === "-" ? -1 : 1;
    const zoneHour = Number(zone.slice(1, 3));
    const zoneMinute = Number(zone.slice(4, 6));
    if (zoneHour > 23 || zoneMinute > 59) return null;
    offsetSeconds = sign * (zoneHour * 3600 + zoneMinute * 60);
  }
  return { seconds: utc / 1000 - offsetSeconds, fraction: (m[7] ?? "").replace(/0+$/, "") };
}

/** Negative, zero or positive; digit strings are compared exactly, never as floats. */
export function compareInstants(a: Instant, b: Instant): number {
  if (a.seconds !== b.seconds) return a.seconds < b.seconds ? -1 : 1;
  const width = Math.max(a.fraction.length, b.fraction.length);
  const left = a.fraction.padEnd(width, "0");
  const right = b.fraction.padEnd(width, "0");
  return left === right ? 0 : left < right ? -1 : 1;
}

/**
 * Chronological comparison of two timestamp strings. Unparsable text is the
 * greatest value (and equals other unparsable text): it sorts after every
 * valid instant ascending and, because descending negates, before them
 * descending. A damaged value never lands in the middle of the list.
 */
export function compareTimestamps(a: string, b: string): number {
  const left = parseInstant(a);
  const right = parseInstant(b);
  if (left && right) return compareInstants(left, right);
  if (left) return -1;
  if (right) return 1;
  return 0;
}
