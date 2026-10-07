/** The build's release stage and the per-stage "don't ask again" programming consent. */
// Programming a device rewrites something in a building. This build is
// pre-release software, so before it does that the user must say yes, and
// the question has to name the build's release stage (alpha, beta, …)
// rather than a generic "are you sure". `ProgrammingConsentDialog.tsx`
// asks; `useProgrammingConsent.ts` decides when; this module owns the two
// facts both of them need.
//
// **Scope.** This guards *programming*: individual-address writes,
// application downloads, unloads, restarts. It does not guard group-value
// sends from the bus monitor, which stay as they are. The application has
// no programming entry point yet (the procedures exist only in `knx-net`
// and its opt-in live tests), so nothing calls this today; the first
// programming feature must go through `useProgrammingConsent().request()`.
//
// **Where the stage comes from.** `GET /api/version`, the running
// server's own build version — the same string the About dialog shows.
// The frontend's `package.json` is deliberately not consulted: nothing
// keeps it in step with the build (see `AboutDialog.tsx`).
//
// **What "don't ask again" remembers.** The stage, not a bare `true`. A
// consent given to an alpha build does not carry over to a beta or to a
// stable release; each of those asks once more. A build whose stage
// cannot be named (no answer from the server, an unreadable version, a
// pre-release label that is neither alpha, beta nor rc) never has a
// remembered answer: it asks every time.
//
// This is a UI confirmation, not an authorisation. The library-level gate
// — `knx_core::commissioning::mutation::WriteAuthorisation`, with its
// device-specific confirmation phrase and its hardware allowlist — stays
// the load-bearing check, and nothing here can widen it.

import { getSetting, setSetting } from "./settingsStore";

/** The settings-document key for the remembered consent. */
export const PROGRAMMING_CONSENT_KEY = "programmingConsent";

/**
 * A build's release stage, read from its SemVer pre-release label.
 * `preRelease` is a pre-release with some other label; `unknown` is no
 * version at all, or a string that is not SemVer.
 */
export type ReleaseStage = "alpha" | "beta" | "releaseCandidate" | "stable" | "preRelease" | "unknown";

/** Stages a consent may be remembered for: those the dialog can name. */
const REMEMBERABLE: ReadonlySet<ReleaseStage> = new Set(["alpha", "beta", "releaseCandidate", "stable"]);

/**
 * Whether `stage` is one a remembered answer may be keyed by: a stage the
 * UI can name. The first-run guide (`onboardingGuide.ts`) remembers "seen"
 * by the same rule, so the two cannot disagree about what a stage is.
 */
export function isRememberableStage(stage: ReleaseStage): boolean {
  return REMEMBERABLE.has(stage);
}

// SemVer 2.0.0 core, optional pre-release, optional build metadata. Kept
// deliberately small: leading-zero rules and the like do not change which
// stage a version is in.
const SEMVER = /^\d+\.\d+\.\d+(?:-([0-9A-Za-z.-]+))?(?:\+[0-9A-Za-z.-]+)?$/;

/** The release stage of `version`, e.g. `"0.1.0-alpha.1+ge2e539a"` → `"alpha"`. */
export function releaseStageOf(version: string | null | undefined): ReleaseStage {
  if (!version) return "unknown";
  const match = SEMVER.exec(version);
  if (!match) return "unknown";
  const preRelease = match[1];
  if (preRelease === undefined) return "stable";
  // SemVer identifiers are case-sensitive; the first one names the stage.
  switch (preRelease.split(".")[0]) {
    case "alpha":
      return "alpha";
    case "beta":
      return "beta";
    case "rc":
      return "releaseCandidate";
    default:
      return "preRelease";
  }
}

interface StoredConsent {
  stage: ReleaseStage;
  version: string | null;
}

function storedConsent(): StoredConsent | undefined {
  const value = getSetting(PROGRAMMING_CONSENT_KEY);
  if (typeof value !== "object" || value === null || Array.isArray(value)) return undefined;
  const stage = (value as { stage?: unknown }).stage;
  if (typeof stage !== "string" || !REMEMBERABLE.has(stage as ReleaseStage)) return undefined;
  const version = (value as { version?: unknown }).version;
  return { stage: stage as ReleaseStage, version: typeof version === "string" ? version : null };
}

/** The stage a "don't ask again" was given for, or `undefined` when none (or none valid) is stored. */
export function rememberedProgrammingConsentStage(): ReleaseStage | undefined {
  return storedConsent()?.stage;
}

/** Whether the user said "don't ask again" for exactly this stage. */
export function isProgrammingConsentRemembered(stage: ReleaseStage): boolean {
  if (!REMEMBERABLE.has(stage)) return false;
  return storedConsent()?.stage === stage;
}

/**
 * Persists "don't ask again" for `stage`. A stage the dialog cannot name
 * is never remembered, so this is a no-op for `unknown`/`preRelease`.
 * `version` is recorded for the reader of `settings.json`, not compared.
 */
export function rememberProgrammingConsent(stage: ReleaseStage, version: string | null): void {
  if (!REMEMBERABLE.has(stage)) return;
  setSetting(PROGRAMMING_CONSENT_KEY, { stage, version });
}

/** Clears a remembered consent, so the next programming request asks again. */
export function forgetProgrammingConsent(): void {
  setSetting(PROGRAMMING_CONSENT_KEY, null);
}
