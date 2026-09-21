/** Fixed KNX group-address display plus compatible slash/dot input parsing. */
// The KNX model and every user-facing display use slash notation. Dotted
// input remains accepted for paste/search compatibility and is canonicalised
// before it reaches an API. Project files, exports, reports and payloads stay
// unchanged.
//
// Conversion goes through the address levels, never through a blind replace.
// Names, paths, Free-style addresses and DPT ids therefore stay untouched.
// Callers must still pass group addresses only: an individual address also
// consists of three dotted numbers and cannot be distinguished from syntax
// alone.

/** Separator used by KNXBench for display, persistence and transmission. */
export const CANONICAL_SEPARATOR = "/";
const COMPATIBLE_SEPARATOR = ".";

// A back-reference rejects mixed `1/2.3` syntax. Level values are not range
// checked here because GroupAddressStyle owns the bit split and the server
// remains the validation authority.
const THREE_LEVEL = /^(\d{1,5})([./])(\d{1,5})\2(\d{1,5})$/;
const TWO_LEVEL = /^(\d{1,5})[./](\d{1,5})$/;

/** Returns the numeric levels of a two/three-level address, or null. */
export function groupAddressLevels(text: string): string[] | null {
  const three = THREE_LEVEL.exec(text);
  if (three) return [three[1]!, three[3]!, three[4]!];
  const two = TWO_LEVEL.exec(text);
  if (two) return [two[1]!, two[2]!];
  return null;
}

/** Renders a recognised group address in fixed KNX slash notation. */
export function formatGroupAddress(address: string): string {
  const levels = groupAddressLevels(address);
  return levels === null ? address : levels.join(CANONICAL_SEPARATOR);
}

/** Canonicalises slash or dotted input before it reaches an API. */
export function canonicalGroupAddress(text: string): string {
  const levels = groupAddressLevels(text.trim());
  return levels === null ? text : levels.join(CANONICAL_SEPARATOR);
}

/** Returns accepted search spellings without changing display notation. */
export function groupAddressSpellings(text: string): string[] {
  const levels = groupAddressLevels(text);
  if (levels === null) return [text];
  return [levels.join(CANONICAL_SEPARATOR), levels.join(COMPATIBLE_SEPARATOR)];
}

/** Case-insensitive substring match against both accepted spellings. */
export function groupAddressMatches(address: string, needle: string): boolean {
  const lower = needle.toLowerCase();
  return groupAddressSpellings(address).some((form) => form.toLowerCase().includes(lower));
}

/** Stable formatter used by group-address render sites. */
export function useGroupAddressFormat(): (address: string) => string {
  return formatGroupAddress;
}
