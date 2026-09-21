/** The group-address notation preference: which separator the UI renders, and how input is read. */
// A display preference, not project data. The KNX model stores a 16-bit
// value and the project stores a `GroupAddressStyle` (how many levels);
// neither has an opinion about punctuation. `knx-core` formats every
// group address with `/` and always will — `GroupAddress::format` is
// unchanged by this module, so the project file, the CSV export, the
// documentation export, the debug report and every API payload stay
// byte-identical whatever a user picks here.
//
// What changes is the last step before a person reads the string. The
// server sends `"1/2/3"`; `formatGroupAddress` turns that into `"1.2.3"`
// for a user who reads dots, and `canonicalGroupAddress` turns whatever
// they type back into `"1/2/3"` before it goes anywhere.
//
// The conversion goes through the address's parts, never through a blind
// `replace`. A device named `Kitchen / Ceiling`, a filesystem path, a
// `Free`-style address with no separator at all and a DPT id like
// `DPST-1-1` all fail `groupAddressLevels` and come back untouched.
//
// The one thing this module cannot tell you is whether the string you
// handed it is a group address at all: an individual address is three
// dotted numbers and matches the same grammar. `formatGroupAddress` on a
// `DeviceNode.address` would render it with slashes and make it a lie, so
// call sites are the guard — group addresses only, which in practice
// means `GroupAddressNode.address`, `GroupRangeNode.start`/`end`,
// `GroupLinkNode`/`GroupAddressLinkNode.address` and the bus monitor's
// `destination`, never `device_address`, `AreaNode.address` or
// `LineNode.address`.

import { useMemo } from "react";
import { settingsStorage, useSettingsRevision } from "./settingsStore";

/** The two notations, as stored ids rather than raw punctuation — the
 * same idiom as `ACCENTS`/`DENSITIES`, and it keeps `settings.json`
 * readable and its values validatable. */
export const GA_NOTATIONS = ["slash", "dot"] as const;
export type GaNotation = (typeof GA_NOTATIONS)[number];

/** The key inside the settings document the server keeps (ADR-0029), not
 * a `localStorage` key — `settingsStorage` resolves it against the
 * record. New in this build, so it has no `BROWSER_ERA_KEYS` entry:
 * there is no browser-era value of it to adopt. */
export const GA_NOTATION_KEY = "groupAddressNotation";

/** `/` — what every existing user sees today, what `knx-core` emits, and
 * what KNX documentation uses. */
export const DEFAULT_GA_NOTATION: GaNotation = "slash";

const SEPARATOR: Record<GaNotation, string> = { slash: "/", dot: "." };

/** The separator `knx-core` formats with and every persisted or
 * transmitted string uses. Never a preference. */
export const CANONICAL_SEPARATOR = "/";

// Both notations, in both level counts, with a back-reference so a mixed
// `1/2.3` is rejected rather than half-converted. Level *values* are not
// range-checked here: `GroupAddressStyle` decides the bit split and the
// server owns that verdict, so a UI renderer that second-guessed it would
// only invent a disagreement. Five digits is the widest a 16-bit level
// can be under any style.
const THREE_LEVEL = /^(\d{1,5})([./])(\d{1,5})\2(\d{1,5})$/;
const TWO_LEVEL = /^(\d{1,5})[./](\d{1,5})$/;

/**
 * The numeric levels of a group address written in either notation, or
 * `null` for anything that is not one — a `Free`-style address (plain
 * decimal, no separator), a name, a path, a DPT id, an empty string.
 *
 * `null` is the "leave it alone" signal every caller here relies on.
 */
export function groupAddressLevels(text: string): string[] | null {
  const three = THREE_LEVEL.exec(text);
  if (three) return [three[1]!, three[3]!, three[4]!];
  const two = TWO_LEVEL.exec(text);
  if (two) return [two[1]!, two[2]!];
  return null;
}

/**
 * Renders a group address in the chosen notation. Input is whatever the
 * server sent (canonical `/`), output is the same address with the
 * preferred separator — or the input unchanged when it is not a
 * level-style group address at all.
 */
export function formatGroupAddress(address: string, notation: GaNotation): string {
  const levels = groupAddressLevels(address);
  return levels === null ? address : levels.join(SEPARATOR[notation]);
}

/**
 * Reads a group address a user typed, in either notation, and returns the
 * canonical `/` form the API expects. Both notations are accepted
 * whatever the display preference says: someone pasting an address out of
 * a document, a colleague's email or ETS must not have to translate it
 * first, and a preference about reading is no reason to refuse a
 * spelling.
 *
 * Anything unrecognised is handed back untouched, so the server's own
 * validation names what the user actually typed rather than a mangled
 * version of it.
 */
export function canonicalGroupAddress(text: string): string {
  const levels = groupAddressLevels(text.trim());
  return levels === null ? text : levels.join(CANONICAL_SEPARATOR);
}

/**
 * Every spelling of one address, for search and filter. Both notations,
 * always, regardless of which one is displayed — a user who types `1/1`
 * must find an address shown as `1.1.5`, and a user who types `1.1` must
 * find one shown as `1/1/5`. Non-addresses yield themselves, so the
 * caller can use this on any haystack without checking first.
 */
export function groupAddressSpellings(text: string): string[] {
  const levels = groupAddressLevels(text);
  if (levels === null) return [text];
  return [levels.join(CANONICAL_SEPARATOR), levels.join(SEPARATOR.dot)];
}

/** Case-insensitive substring match against both spellings of `address`. */
export function groupAddressMatches(address: string, needle: string): boolean {
  const lower = needle.toLowerCase();
  return groupAddressSpellings(address).some((form) => form.toLowerCase().includes(lower));
}

/** Reads the preference, falling back to the default for anything
 * unrecognised — same rule every other preference module applies. */
export function loadGaNotation(storage: Pick<Storage, "getItem">): GaNotation {
  let raw: string | null = null;
  try {
    raw = storage.getItem(GA_NOTATION_KEY);
  } catch {
    // Private browsing or storage switched off: the default reads fine.
  }
  return (GA_NOTATIONS as readonly string[]).includes(raw ?? "")
    ? (raw as GaNotation)
    : DEFAULT_GA_NOTATION;
}

export function saveGaNotation(storage: Pick<Storage, "setItem">, notation: GaNotation): void {
  storage.setItem(GA_NOTATION_KEY, notation);
}

/**
 * The preference, re-read whenever the record changes — most importantly
 * when the server's answer lands after a component mounted on the cached
 * document. No `useState` mirror: unlike `useAppearance`, nothing here
 * writes back a DOM attribute, so the record is the only state there is.
 *
 * Deliberately not part of `index.html`'s pre-mount bootstrap. That script
 * exists for the attributes the first paint is styled by; a group address
 * needs no such head start. `App` never fetches a project at mount (`tree`
 * starts `null`; the only mount-time fetch is `App.tsx`'s product-languages
 * request), and `settingsStorage` reads the synchronous `localStorage` cache
 * (`settingsStore.ts`), not a network round trip — `initSettings()`'s own
 * fetch just refreshes that cache once the server answers. So a returning
 * window already has the right notation on first paint, without needing
 * `initSettings()` to have won any race to get there.
 */
export function useGaNotation(): GaNotation {
  const revision = useSettingsRevision();
  return useMemo(() => loadGaNotation(settingsStorage), [revision]);
}

export function setGaNotation(notation: GaNotation): void {
  try {
    saveGaNotation(settingsStorage, notation);
  } catch {
    // The session keeps the change in memory; `settingsStore` has already
    // said so on the console if the record could not be updated.
  }
}

/**
 * A formatter bound to the current preference — what render sites call.
 * One function every call site goes through, rather than the preference
 * threaded through twenty components.
 */
export function useGroupAddressFormat(): (address: string) => string {
  const notation = useGaNotation();
  return useMemo(() => (address: string) => formatGroupAddress(address, notation), [notation]);
}
