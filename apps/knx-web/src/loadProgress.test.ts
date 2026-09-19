/** Tests the load-progress helpers and that the phase list matches the Rust pipeline. */
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { describe, expect, it } from "vitest";
import type { LoadProgressSnapshot } from "./api";
import { LOAD_PHASES, isKnownPhase, loadFraction, phaseMessageKey } from "./loadProgress";
import { messages as enMessages } from "./messages/en";
import { messages as deMessages } from "./messages/de";

function snapshot(overrides: Partial<LoadProgressSnapshot> = {}): LoadProgressSnapshot {
  return {
    operationId: 1,
    kind: "import",
    source: "villa.knxproj",
    phase: "parseTopology",
    completed: null,
    total: null,
    status: "running",
    error: null,
    ...overrides,
  };
}

describe("loadFraction", () => {
  it("is a real fraction when the server sent a real pair", () => {
    expect(loadFraction(snapshot({ completed: 9, total: 36 }))).toEqual({
      completed: 9,
      total: 36,
      value: 0.25,
      percent: 25,
    });
  });

  it("is null for every phase that reported no count", () => {
    expect(loadFraction(snapshot())).toBeNull();
    expect(loadFraction(snapshot({ completed: 3, total: null }))).toBeNull();
    expect(loadFraction(snapshot({ completed: null, total: 3 }))).toBeNull();
  });

  // The point of the whole feature: there is no input — not a duration, not
  // a file size, not a phase ordinal — from which this function will
  // manufacture a percentage. A reviewer mutating the caller to pass
  // elapsed milliseconds gets `null` out of a malformed pair, not a bar.
  it("is null for a pair that is not a measurement", () => {
    expect(loadFraction(snapshot({ completed: 1, total: 0 }))).toBeNull();
    expect(loadFraction(snapshot({ completed: 7, total: 6 }))).toBeNull();
    expect(loadFraction(snapshot({ completed: -1, total: 6 }))).toBeNull();
    expect(loadFraction(snapshot({ completed: 1, total: Number.POSITIVE_INFINITY }))).toBeNull();
  });

  it("is null when there is no snapshot at all", () => {
    expect(loadFraction(null)).toBeNull();
  });

  it("rounds only for display and keeps the exact ratio too", () => {
    const fraction = loadFraction(snapshot({ completed: 1, total: 3 }));
    expect(fraction!.percent).toBe(33);
    expect(fraction!.value).toBeCloseTo(1 / 3, 10);
  });
});

describe("phaseMessageKey", () => {
  it("maps every known phase to a key both catalogues have", () => {
    for (const phase of LOAD_PHASES) {
      const key = phaseMessageKey(phase);
      expect(key, `no key for phase "${phase}"`).not.toBeNull();
      expect(enMessages[key!], `en.ts is missing ${key}`).toBeTruthy();
      expect(deMessages[key!], `de.ts is missing ${key}`).toBeTruthy();
    }
  });

  it("returns null for a phase this build does not know", () => {
    expect(phaseMessageKey("parseSomethingNewer")).toBeNull();
    expect(isKnownPhase("parseSomethingNewer")).toBe(false);
  });
});

// `LOAD_PHASES` is a hand-kept mirror of three Rust enums. Nothing in the
// type system connects them, so this reads the Rust sources and compares.
// A phase added on the server without a translation here would otherwise
// only be noticed by whoever happened to be watching the banner at the
// moment that phase ran.
describe("the phase list and the Rust pipeline", () => {
  const repo = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
  const sources = [
    join(repo, "crates", "knx-etsproj", "src", "progress.rs"),
    join(repo, "crates", "knx-app", "src", "progress.rs"),
    join(repo, "apps", "knx-server", "src", "load_progress.rs"),
  ];
  // Every `Variant => "name",` arm in those three files' `as_str`
  // implementations — which also picks up `LoadKind` and `LoadStatus`,
  // both listed below so an unexpected new name still fails.
  const wireNames = new Set(
    sources.flatMap((path) =>
      [...readFileSync(path, "utf-8").matchAll(/=>\s*"([a-zA-Z]+)"/g)].map((match) => match[1]),
    ),
  );
  const nonPhases = ["import", "open", "running", "succeeded", "failed"];

  it("has an entry for every phase the server can send", () => {
    const missing = [...wireNames].filter(
      (name) => !nonPhases.includes(name) && !(LOAD_PHASES as readonly string[]).includes(name),
    );
    expect(missing, `phases the Rust pipeline emits with no entry in LOAD_PHASES: ${missing}`).toEqual([]);
  });

  it("invents no phase the server cannot send", () => {
    const surplus = LOAD_PHASES.filter((phase) => !wireNames.has(phase));
    expect(surplus, `LOAD_PHASES names phases no Rust enum emits: ${surplus}`).toEqual([]);
  });
});
