/** Tests the load-progress helpers and that the phase list matches the Rust pipeline. */
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { describe, expect, it } from "vitest";
import type { LoadProgressSnapshot } from "./api";
import { LOAD_PHASES, isKnownPhase, loadFraction, localFailure, ownsOperation, phaseMessageKey } from "./loadProgress";
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

// The client half of ADR-0023's operation id (fix round 1, F2). Without
// this the banner cannot tell its own load from the previous one or from
// another client's, which is what F1 was.
describe("ownsOperation", () => {
  const ours = "villa.knxproj";

  it("adopts an operation newer than everything that predates the load", () => {
    expect(ownsOperation({ baseline: 3, expectedSource: ours, adopted: null }, snapshot({ operationId: 4 }))).toBe(
      true,
    );
    expect(ownsOperation({ baseline: 0, expectedSource: ours, adopted: null }, snapshot({ operationId: 1 }))).toBe(
      true,
    );
  });

  it("disowns the operation that was already there", () => {
    // Probe A and probe C in one line: a finished earlier load and a
    // stranger's running one both carry an id the baseline already knew.
    expect(ownsOperation({ baseline: 3, expectedSource: ours, adopted: null }, snapshot({ operationId: 3 }))).toBe(
      false,
    );
    expect(ownsOperation({ baseline: 7, expectedSource: ours, adopted: null }, snapshot({ operationId: 7 }))).toBe(
      false,
    );
    expect(ownsOperation({ baseline: 7, expectedSource: ours, adopted: null }, snapshot({ operationId: 2 }))).toBe(
      false,
    );
  });

  // Round 2, F8: an id newer than the baseline is not enough by itself — a
  // foreign operation that starts *after* our baseline read clears that
  // test too. `source` is the second fact that keeps it from being
  // adopted. The residual this does not close: two clients loading files
  // with the same base name in the same window are still indistinguishable.
  it("does not adopt a newer id whose source is somebody else's file", () => {
    expect(
      ownsOperation(
        { baseline: 0, expectedSource: ours, adopted: null },
        snapshot({ operationId: 2, source: "someone-elses.knxdb" }),
      ),
    ).toBe(false);
  });

  it("owns exactly one id once it has adopted one", () => {
    expect(ownsOperation({ baseline: 0, expectedSource: ours, adopted: 5 }, snapshot({ operationId: 5 }))).toBe(
      true,
    );
    // A later operation cannot take the banner over, even though it
    // would clear the baseline test.
    expect(ownsOperation({ baseline: 0, expectedSource: ours, adopted: 5 }, snapshot({ operationId: 6 }))).toBe(
      false,
    );
  });

  // After adoption the id is the only thing that decides it — source plays
  // no further part, unchanged from before this round.
  it("ignores source once an id has been adopted", () => {
    expect(
      ownsOperation(
        { baseline: 0, expectedSource: ours, adopted: 5 },
        snapshot({ operationId: 5, source: "someone-elses.knxdb" }),
      ),
    ).toBe(true);
  });

  it("owns nothing when the baseline could not be read", () => {
    // Probe B: the pre-flight read failed, so no id can be attributed to
    // this load. "Starting…" is less informative and a great deal truer.
    expect(ownsOperation({ baseline: null, expectedSource: ours, adopted: null }, snapshot({ operationId: 1 }))).toBe(
      false,
    );
    expect(
      ownsOperation({ baseline: null, expectedSource: ours, adopted: null }, snapshot({ operationId: 99 })),
    ).toBe(false);
  });

  it("owns nothing when there is no snapshot", () => {
    expect(ownsOperation({ baseline: 0, expectedSource: ours, adopted: null }, null)).toBe(false);
  });
});

describe("localFailure", () => {
  it("keeps the last phase this load really saw and drops every count", () => {
    const failure = localFailure(snapshot({ phase: "collectContainerEntries", completed: 9, total: 36 }), "boom");
    expect(failure.status).toBe("failed");
    expect(failure.phase).toBe("collectContainerEntries");
    expect(failure.error).toBe("boom");
    // A count that outlived its phase would be a measurement of nothing.
    expect(failure.completed).toBeNull();
    expect(failure.total).toBeNull();
    expect(loadFraction(failure)).toBeNull();
  });

  it("falls back to starting when the load never saw a phase of its own", () => {
    const failure = localFailure(null, "Failed to fetch");
    expect(failure.phase).toBe("starting");
    expect(failure.status).toBe("failed");
    expect(failure.error).toBe("Failed to fetch");
    // Zero is an id the server never issues: there may be no operation.
    expect(failure.operationId).toBe(0);
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

  // Without this the two tests below would both pass on an empty set —
  // vacuously green the day a refactor stops writing `=> "name"` arms.
  it("really extracted something from each of the three sources", () => {
    for (const path of sources) {
      const names = [...readFileSync(path, "utf-8").matchAll(/=>\s*"([a-zA-Z]+)"/g)];
      expect(names.length, `no wire names extracted from ${path}`).toBeGreaterThan(0);
    }
    expect(wireNames.size, "fewer wire names than phases — the extraction is broken").toBeGreaterThanOrEqual(
      LOAD_PHASES.length,
    );
  });

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
