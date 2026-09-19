/** The load banner's flavour line: its strings, its order, and whether it moves. */
// Decoration, and nothing but (ADR-0023). Nothing in this module reads a
// snapshot, a phase, a count or a clock-derived position, so no change
// here can make the banner claim a load got further than the server
// said. It owns fifty strings, a shuffle and one interval — that is the
// whole of its authority, and the phase label beside it stays the truth.

import type { MessageKey } from "./i18n";

/**
 * How long one flavour line stays on screen.
 *
 * Deliberately slower than the 250 ms poll: the phase label is the thing
 * a person is reading, and a joke that changes faster than the news
 * competes with it. At this interval the measured 114 ms release load
 * shows exactly one line and the ~1.2 s debug load shows one or two —
 * the list is a reward for slow loads and large projects, not something
 * a normal import will ever cycle through.
 */
export const FLAVOUR_INTERVAL_MS = 1800;

/**
 * Every flavour key, in catalogue order.
 *
 * Written out rather than generated from a counter on purpose: each
 * literal is checked against `MessageKey`, so a key that is missing from
 * `messages/en.ts` — or a typo in one — is a compile error here instead
 * of an empty line on screen. `messages/de.ts` is typed against English,
 * which makes the German half a compile error too.
 */
export const FLAVOUR_KEYS = [
  "loadProgress.flavour.01",
  "loadProgress.flavour.02",
  "loadProgress.flavour.03",
  "loadProgress.flavour.04",
  "loadProgress.flavour.05",
  "loadProgress.flavour.06",
  "loadProgress.flavour.07",
  "loadProgress.flavour.08",
  "loadProgress.flavour.09",
  "loadProgress.flavour.10",
  "loadProgress.flavour.11",
  "loadProgress.flavour.12",
  "loadProgress.flavour.13",
  "loadProgress.flavour.14",
  "loadProgress.flavour.15",
  "loadProgress.flavour.16",
  "loadProgress.flavour.17",
  "loadProgress.flavour.18",
  "loadProgress.flavour.19",
  "loadProgress.flavour.20",
  "loadProgress.flavour.21",
  "loadProgress.flavour.22",
  "loadProgress.flavour.23",
  "loadProgress.flavour.24",
  "loadProgress.flavour.25",
  "loadProgress.flavour.26",
  "loadProgress.flavour.27",
  "loadProgress.flavour.28",
  "loadProgress.flavour.29",
  "loadProgress.flavour.30",
  "loadProgress.flavour.31",
  "loadProgress.flavour.32",
  "loadProgress.flavour.33",
  "loadProgress.flavour.34",
  "loadProgress.flavour.35",
  "loadProgress.flavour.36",
  "loadProgress.flavour.37",
  "loadProgress.flavour.38",
  "loadProgress.flavour.39",
  "loadProgress.flavour.40",
  "loadProgress.flavour.41",
  "loadProgress.flavour.42",
  "loadProgress.flavour.43",
  "loadProgress.flavour.44",
  "loadProgress.flavour.45",
  "loadProgress.flavour.46",
  "loadProgress.flavour.47",
  "loadProgress.flavour.48",
  "loadProgress.flavour.49",
  "loadProgress.flavour.50",
] as const satisfies readonly MessageKey[];

/**
 * The flavour keys in a fresh random order, drawn from `random`.
 *
 * `random` is a parameter rather than a direct `Math.random()` call so a
 * test can pin the sequence: a component that rolled its own dice would
 * force every test here to be either flaky or vacuous. A Fisher-Yates
 * shuffle, clamped against a `random` that returns exactly 1 — which
 * `Math.random` never does, and an injected stub very much can.
 */
export function shuffleFlavourKeys(random: () => number): MessageKey[] {
  const keys: MessageKey[] = [...FLAVOUR_KEYS];
  for (let i = keys.length - 1; i > 0; i -= 1) {
    const j = Math.min(i, Math.max(0, Math.floor(random() * (i + 1))));
    const held = keys[i];
    keys[i] = keys[j];
    keys[j] = held;
  }
  return keys;
}

/**
 * Whether the flavour line is allowed to swap itself for the next one.
 *
 * A line of text that changes every 1 800 ms is motion, even though no
 * pixel slides, so it answers to the same two switches the banner's
 * shuttle does: the operating system's `prefers-reduced-motion` and the
 * application's own "Motion: off" level. Under either one the banner
 * shows a single flavour line for the whole load — still a joke, just a
 * patient one — because hiding the line entirely would punish the people
 * who asked for calm by giving them less, not less movement.
 *
 * A runtime with no `matchMedia` is not a runtime asking for stillness,
 * so the absence resolves to "rotate".
 */
export function flavourRotates(
  win: Pick<Window, "matchMedia">,
  root: Pick<Element, "getAttribute">,
): boolean {
  if (root.getAttribute("data-motion-level") === "off") return false;
  if (typeof win.matchMedia !== "function") return true;
  return !win.matchMedia("(prefers-reduced-motion: reduce)").matches;
}
