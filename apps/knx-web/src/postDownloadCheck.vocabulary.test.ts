/** Pins the run-state vocabulary: admission and EN/DE messages match the Rust codes. */
import { describe, expect, it } from "vitest";
import fixtures from "./postDownloadCheck.fixtures.json";
import vocabulary from "./postDownloadCheck.vocabulary.json";
import { admitPostDownloadCheck, runStateLines, type PostDownloadCheck } from "./postDownloadCheck";
import { messages as en } from "./messages/en";
import { messages as de } from "./messages/de";

const FIXTURES = fixtures as unknown as Record<string, PostDownloadCheck>;
const BOGUS = "notARealCode";

/** A copy of a fixture whose identity run-state record (and optionally runtime) is replaced. */
function withRunState(base: string, runState: unknown, extra: Record<string, unknown> = {}, runtime?: string): unknown {
  const value = JSON.parse(JSON.stringify(FIXTURES[base]));
  Object.assign(value.observation.reach.identity, { runState, ...extra });
  if (runtime !== undefined) value.runtime = runtime;
  return value;
}

const admitted = (value: unknown) => admitPostDownloadCheck(value) !== null;
const hasMessage = (key: string) => key in en && key in de;

describe("run-state wire vocabulary", () => {
  it("admits every observed state with its runtime and refuses an unknown one", () => {
    for (const state of vocabulary.runState) {
      const record = { status: "observed", objectIndex: 3, state };
      expect(admitted(withRunState("runStateRunning", record, {}, state)), state).toBe(true);
      expect(hasMessage(`postDownloadCheck.runState.state.${state}`), state).toBe(true);
    }
    expect(admitted(withRunState("runStateRunning", { status: "observed", objectIndex: 3, state: BOGUS }, {}, BOGUS))).toBe(false);
  });

  it("admits every step/cause pair the contract allows and names each in both languages", () => {
    for (const step of vocabulary.step) {
      expect(hasMessage(`postDownloadCheck.runState.step.${step}`), step).toBe(true);
      for (const cause of vocabulary.cause) {
        const record = { status: "unavailable", step, cause };
        const qualified = step === "description";
        const refusedHere = qualified && (cause === "refused" || cause === "malformed");
        const base = withRunState("exact0700DescriptionUnavailable", record);
        const unqualified = withRunState("runStateUnavailable", record);
        expect(admitted(qualified ? base : unqualified), `${step}/${cause}`).toBe(!refusedHere);
      }
    }
    for (const cause of vocabulary.cause) expect(hasMessage(`postDownloadCheck.runState.cause.${cause}`), cause).toBe(true);
    expect(admitted(withRunState("runStateUnavailable", { status: "unavailable", step: BOGUS, cause: "refused" }))).toBe(false);
    expect(admitted(withRunState("runStateUnavailable", { status: "unavailable", step: "runState", cause: BOGUS }))).toBe(false);
  });

  it("admits every not-read reason and candidate mismatch kind with its own message", () => {
    const unsupported = withRunState("runStateUnsupportedMask", { status: "notRead", reason: "unsupportedMask" });
    const loadFailed = withRunState("runStateLoadFailed", { status: "notRead", reason: "loadReadFailed" });
    expect(vocabulary.notReadReason).toEqual(["unsupportedMask", "loadReadFailed"]);
    expect(admitted(unsupported) && admitted(loadFailed)).toBe(true);
    for (const reason of vocabulary.notReadReason) expect(hasMessage(`postDownloadCheck.runState.notRead.${reason}`), reason).toBe(true);
    expect(admitted(withRunState("runStateUnsupportedMask", { status: "notRead", reason: BOGUS }))).toBe(false);
    for (const kind of vocabulary.candidateMismatchKind) {
      expect(admitted(withRunState("runStateCandidateMismatch", { status: "candidateMismatch", kind })), kind).toBe(true);
      expect(hasMessage(`postDownloadCheck.runState.candidateMismatch.${kind}`), kind).toBe(true);
    }
    expect(admitted(withRunState("runStateCandidateMismatch", { status: "candidateMismatch", kind: BOGUS }))).toBe(false);
  });

  it("admits every qualification reason and run-state mask, refusing an unknown one", () => {
    for (const reason of vocabulary.qualificationReason) {
      const record = reason === "unsupportedDatatype" ? { status: "notQualified", reason, pdt: 5 } : { status: "notQualified", reason };
      expect(admitted(withRunState("exact0700DescriptionUnavailable", record)), reason).toBe(true);
      expect(hasMessage(`postDownloadCheck.runState.notQualified.${reason}`), reason).toBe(true);
      const line = runStateLines(admitPostDownloadCheck(withRunState("exact0700DescriptionUnavailable", record))!);
      expect(line.map((entry) => entry.key), reason).toEqual([`postDownloadCheck.runState.notQualified.${reason}`]);
    }
    expect(admitted(withRunState("exact0700DescriptionUnavailable", { status: "notQualified", reason: BOGUS }))).toBe(false);
    for (const mask of vocabulary.runStateMask) {
      const base = JSON.parse(JSON.stringify(FIXTURES.exact0700DescriptionUnavailable));
      base.observation.reach.identity.runStateMask = mask;
      expect(admitted(base), mask).toBe(true);
    }
    const bogusMask = JSON.parse(JSON.stringify(FIXTURES.exact0700DescriptionUnavailable));
    bogusMask.observation.reach.identity.runStateMask = BOGUS;
    expect(admitted(bogusMask)).toBe(false);
  });
});
