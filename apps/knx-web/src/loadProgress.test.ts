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
    clientToken: "own-token",
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

// The client half of ADR-0023's ownership test (fix round 3, F9). Three
// rounds of a server-side heuristic — no filter, an id-only filter, an
// id-and-source filter — each let one client's poll adopt another
// client's operation. This replaces all of it with exact equality on an
// opaque token the client generates for itself.
describe("ownsOperation", () => {
  const ours = "own-token";

  it("owns a snapshot carrying exactly this load's token", () => {
    expect(ownsOperation({ clientToken: ours }, snapshot({ clientToken: ours }))).toBe(true);
  });

  it("disowns a snapshot carrying a different token", () => {
    expect(ownsOperation({ clientToken: ours }, snapshot({ clientToken: "someone-elses-token" }))).toBe(false);
  });

  // F9's exact failure: two clients loading files with the same base name
  // from different directories both land in the same baseline-to-adoption
  // window the old heuristic could not see past. The token makes that
  // window irrelevant — a snapshot is never ours because of *when* it
  // appeared or *what* it is named, only because of the id it carries.
  it("disowns a same-basename stranger's operation even mid-window", () => {
    expect(
      ownsOperation(
        { clientToken: ours },
        snapshot({ operationId: 9, source: "villa.knxproj", clientToken: "strangers-token" }),
      ),
    ).toBe(false);
  });

  it("owns nothing when the operation carries no token at all", () => {
    // An operation nobody sent a token for belongs to nobody — not to
    // whoever happens to be polling, which would be the old heuristic's
    // mistake wearing a new name.
    expect(ownsOperation({ clientToken: ours }, snapshot({ clientToken: null }))).toBe(false);
  });

  it("owns nothing when there is no snapshot", () => {
    expect(ownsOperation({ clientToken: ours }, null)).toBe(false);
  });

  // Fix round 4, F12. The three cases below are the ones the validity
  // guard exists for, and the only ones that can fail if it is deleted:
  // every other case above passes a real token on both sides, where plain
  // equality already answers correctly. "Absent" has three spellings on
  // this wire — `null`, a missing field, and `""` — and none of them may
  // ever match another absence.
  it("owns nothing when neither side has a token", () => {
    // A server build that omits `clientToken` altogether sends
    // `undefined`, which is not `null`; a client whose token ref was
    // never filled reads the same. `undefined === undefined` is exactly
    // the accidental match the old null check could not stop.
    const missing = snapshot();
    delete (missing as Partial<LoadProgressSnapshot>).clientToken;
    expect(ownsOperation({ clientToken: undefined as unknown as string }, missing)).toBe(false);
  });

  it("owns nothing when this load never generated a token of its own", () => {
    // `runLoad`'s ref defaults to `""` before `crypto.randomUUID()` runs.
    // An empty string is not an id, on either side of the comparison.
    expect(ownsOperation({ clientToken: "" }, snapshot({ clientToken: "" }))).toBe(false);
    expect(ownsOperation({ clientToken: "" }, snapshot({ clientToken: ours }))).toBe(false);
  });

  it("owns nothing when the operation's token is empty rather than absent", () => {
    expect(ownsOperation({ clientToken: ours }, snapshot({ clientToken: "" }))).toBe(false);
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
