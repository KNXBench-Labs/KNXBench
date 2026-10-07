/** The first-run guide's rules: when it opens by itself, what it remembers, what it offers. */
// ADR-0084. The guide (`OnboardingGuide.tsx`) tells a new user what this
// build is — above all, how far along it is — and where to start. It is
// advice, not a gate: nothing waits for it, and the safety question before
// a device write stays with `useProgrammingConsent` and the library's
// `WriteAuthorisation`. Nothing here renders anything.
//
// **When it opens by itself.** Once per *release stage*, the same unit
// the programming consent is remembered by (`programmingConsent.ts`): an
// alpha user sees it once, and sees it again when the build becomes a
// beta, because what the guide says about the build has changed. A stage
// the UI cannot name never opens it by itself — a build that cannot say
// what it is would otherwise open the guide on every start.
//
// It also stays closed unless the settings record is one the server has
// acknowledged (`getAcknowledgedSettings`): "seen" is remembered there,
// and a guide that cannot remember being seen is a guide on every start.
// And it never opens over something the user is already doing — a
// project, a load in progress, another dialog. Whatever the reason, it
// does not queue itself for later in the session; it waits for the next
// start, and the command palette and the File menu open it any time.
//
// **What it remembers.** `{ seenStage, version }` under one key of the
// server's `settings.json`, so per installation, not per person (KL §160).
// `version` is for whoever reads the file; only the stage is compared.

import type { MessageKey } from "./messages/en";
import { isRememberableStage, type ReleaseStage } from "./programmingConsent";
import { setSetting, type SettingsAcknowledgedSnapshot } from "./settingsStore";

/** The settings-document key for "the guide was seen in this stage". */
export const ONBOARDING_GUIDE_KEY = "onboardingGuide";

/** What the guide's start-up decision looks at, gathered by `useOnboardingGuide`. */
export interface OnboardingStartup {
  /** The running server's release stage (`releaseStageOf(GET /api/version)`). */
  stage: ReleaseStage;
  /** `getAcknowledgedSettings([ONBOARDING_GUIDE_KEY])`. */
  settings: SettingsAcknowledgedSnapshot;
  projectOpen: boolean;
  /** A project load or import is in progress. */
  busy: boolean;
  /** Any dialog (or the file picker) is already open. */
  otherDialogOpen: boolean;
}

/** The stage the guide was last seen in, or `undefined` when none (or none valid) is stored. */
export function seenOnboardingStage(value: unknown): ReleaseStage | undefined {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return undefined;
  const stage = (value as { seenStage?: unknown }).seenStage;
  if (typeof stage !== "string" || !isRememberableStage(stage as ReleaseStage)) return undefined;
  return stage as ReleaseStage;
}

/** Whether the guide opens by itself on this start. Everything else waits for the next one. */
export function shouldOpenOnboardingGuide(startup: OnboardingStartup): boolean {
  if (!isRememberableStage(startup.stage)) return false;
  if (!startup.settings.ok) return false;
  if (startup.projectOpen || startup.busy || startup.otherDialogOpen) return false;
  return seenOnboardingStage(startup.settings.settings[ONBOARDING_GUIDE_KEY]) !== startup.stage;
}

/** Records that the guide was seen in `stage`. A stage the UI cannot name is never recorded. */
export function rememberOnboardingGuideSeen(stage: ReleaseStage, version: string | null): void {
  if (!isRememberableStage(stage)) return;
  setSetting(ONBOARDING_GUIDE_KEY, { seenStage: stage, version });
}

/** One "where to start" entry: a short explanation and the palette command it runs. */
export interface OnboardingTask {
  id: string;
  titleKey: MessageKey;
  descriptionKey: MessageKey;
  /** A `COMMANDS` id; `onboardingGuide.test.ts` proves each exists and runs with no project. */
  commandId: string;
}

/**
 * The guide's "where to start" step, in the order a new user needs them:
 * get a project in, get product data in, look at the bus. Saving is not a
 * task here because there is nothing to save yet; the step's text says
 * where a project goes once it exists.
 */
export const ONBOARDING_TASKS = [
  {
    id: "importEts",
    titleKey: "onboarding.task.importEts.title",
    descriptionKey: "onboarding.task.importEts.description",
    commandId: "open-project",
  },
  {
    id: "newProject",
    titleKey: "onboarding.task.newProject.title",
    descriptionKey: "onboarding.task.newProject.description",
    commandId: "new-project",
  },
  {
    id: "openNative",
    titleKey: "onboarding.task.openNative.title",
    descriptionKey: "onboarding.task.openNative.description",
    commandId: "open-native",
  },
  {
    id: "productData",
    titleKey: "onboarding.task.productData.title",
    descriptionKey: "onboarding.task.productData.description",
    commandId: "open-catalog",
  },
  {
    id: "busMonitor",
    titleKey: "onboarding.task.busMonitor.title",
    descriptionKey: "onboarding.task.busMonitor.description",
    commandId: "open-bus-monitor",
  },
] as const satisfies readonly OnboardingTask[];
