/** Tests the release-stage parse and the remembered programming consent. */
// @vitest-environment happy-dom
//
// The consent is a safety question, so the defaults are pinned here:
// anything the parser cannot name asks every time, and a remembered
// answer covers only the release stage it was given for.
import { afterEach, beforeEach, expect, it } from "vitest";
import {
  PROGRAMMING_CONSENT_KEY,
  forgetProgrammingConsent,
  isProgrammingConsentRemembered,
  releaseStageOf,
  rememberProgrammingConsent,
  type ReleaseStage,
} from "./programmingConsent";
import { getSetting, resetSettingsForTests, setSetting } from "./settingsStore";

beforeEach(() => resetSettingsForTests());
afterEach(() => resetSettingsForTests());

it.each<[string | null, ReleaseStage]>([
  ["0.1.0-alpha.1", "alpha"],
  ["0.1.0-alpha.1+ge2e539a", "alpha"],
  ["0.1.0-alpha", "alpha"],
  ["1.0.0-beta.3", "beta"],
  ["1.0.0-rc.1", "releaseCandidate"],
  ["1.0.0", "stable"],
  ["1.0.0+gabc123", "stable"],
  // SemVer pre-release identifiers are case-sensitive and ASCII; "Alpha"
  // is not "alpha", so it is an unnamed pre-release, not a guess.
  ["1.0.0-Alpha.1", "preRelease"],
  ["1.0.0-nightly.20260927", "preRelease"],
  ["not a version", "unknown"],
  ["", "unknown"],
  [null, "unknown"],
])("%s is the %s stage", (version, stage) => {
  expect(releaseStageOf(version)).toBe(stage);
});

it("asks by default: nothing has been remembered", () => {
  expect(isProgrammingConsentRemembered("alpha")).toBe(false);
});

it("remembers a consent for the stage it was given in", () => {
  rememberProgrammingConsent("alpha", "0.1.0-alpha.1");
  expect(isProgrammingConsentRemembered("alpha")).toBe(true);
  expect(getSetting(PROGRAMMING_CONSENT_KEY)).toEqual({
    stage: "alpha",
    version: "0.1.0-alpha.1",
  });
});

it("asks again when the release stage changes", () => {
  rememberProgrammingConsent("alpha", "0.1.0-alpha.1");
  expect(isProgrammingConsentRemembered("beta")).toBe(false);
  expect(isProgrammingConsentRemembered("stable")).toBe(false);
});

it("never remembers a consent for a stage it cannot name", () => {
  // A server that did not answer, or a version string nobody can read, is
  // exactly the situation in which the user must be asked every time.
  for (const stage of ["unknown", "preRelease"] as const) {
    rememberProgrammingConsent(stage, null);
    expect(isProgrammingConsentRemembered(stage)).toBe(false);
    expect(getSetting(PROGRAMMING_CONSENT_KEY)).toBeUndefined();
  }
});

it("forgets a remembered consent", () => {
  rememberProgrammingConsent("alpha", "0.1.0-alpha.1");
  forgetProgrammingConsent();
  expect(isProgrammingConsentRemembered("alpha")).toBe(false);
  expect(getSetting(PROGRAMMING_CONSENT_KEY)).toBeUndefined();
});

it.each([
  ["a bare true", true],
  ["a string", "alpha"],
  ["a missing stage", { version: "0.1.0-alpha.1" }],
  ["an invented stage", { stage: "gamma", version: "1" }],
  ["an unknown stage", { stage: "unknown", version: null }],
])("a stored value that is %s counts as no consent", (_label, value) => {
  setSetting(PROGRAMMING_CONSENT_KEY, value);
  expect(isProgrammingConsentRemembered("alpha")).toBe(false);
  expect(isProgrammingConsentRemembered("unknown")).toBe(false);
});
