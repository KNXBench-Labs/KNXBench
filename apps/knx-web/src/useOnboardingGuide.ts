/** Decides once per start whether the first-run guide opens, and opens it on request. */
// ADR-0084. The rules are `onboardingGuide.ts`'s; this hook only gathers
// their inputs at the one moment they are asked: when the settings record
// has been read from the server for the first time. One decision per start
// — a settings refresh, a closed project or a dismissed dialog later in the
// session never brings the guide up by itself.
//
// The version is asked only after the settings answer is usable, so a
// start that cannot remember the guide being seen costs no extra request.
// React's StrictMode runs the effect twice in development; the decision
// is recorded when the answer is *used*, not when the request leaves, so
// the cancelled first run cannot swallow it.
import { useCallback, useEffect, useRef, useState } from "react";
import * as api from "./api";
import { ONBOARDING_GUIDE_KEY, rememberOnboardingGuideSeen, shouldOpenOnboardingGuide } from "./onboardingGuide";
import { releaseStageOf, type ReleaseStage } from "./programmingConsent";
import { getAcknowledgedSettings, useSettingsState } from "./settingsStore";

/** The open guide: the stage it describes and the version it was read from. */
export interface OpenOnboardingGuide {
  stage: ReleaseStage;
  version: string | null;
}

export interface OnboardingGuideControls {
  /** The guide while it is open, otherwise `null`. */
  open: OpenOnboardingGuide | null;
  /** Opens the guide now, whatever the start-up rules said (palette, File menu). */
  show: () => void;
  /** Closes it and remembers it as seen for its stage — every way out counts. */
  close: () => void;
}

/** Whether a dialog, or the file picker's separate root, is on screen. */
export function anotherDialogIsOpen(root: ParentNode = document): boolean {
  return root.querySelector('[role="dialog"], dialog[open], .fs-picker') !== null;
}

async function runningVersion(): Promise<string | null> {
  try {
    return (await api.serverVersion()).version;
  } catch {
    return null;
  }
}

export function useOnboardingGuide(startup: { projectOpen: boolean; busy: boolean }): OnboardingGuideControls {
  const { hydration } = useSettingsState();
  const [open, setOpen] = useState<OpenOnboardingGuide | null>(null);
  // Read when the version answer lands, not when it was asked for: a
  // project opened in between must still keep the guide closed.
  const startupRef = useRef(startup);
  useEffect(() => {
    startupRef.current = startup;
  });
  const decided = useRef(false);
  const showRequest = useRef(0);

  useEffect(() => {
    if (decided.current || hydration === "cached") return;
    if (hydration !== "hydrated" || !getAcknowledgedSettings([ONBOARDING_GUIDE_KEY]).ok) {
      decided.current = true;
      return;
    }
    let cancelled = false;
    void runningVersion().then((version) => {
      if (cancelled || decided.current) return;
      decided.current = true;
      const stage = releaseStageOf(version);
      const { projectOpen, busy } = startupRef.current;
      const opens = shouldOpenOnboardingGuide({
        stage,
        settings: getAcknowledgedSettings([ONBOARDING_GUIDE_KEY]),
        projectOpen,
        busy,
        otherDialogOpen: anotherDialogIsOpen(),
      });
      if (opens) setOpen({ stage, version });
    });
    return () => {
      cancelled = true;
    };
  }, [hydration]);

  const show = useCallback(() => {
    // Asked for by the user: the automatic decision has nothing left to do.
    decided.current = true;
    const request = ++showRequest.current;
    void runningVersion().then((version) => {
      if (request !== showRequest.current) return;
      setOpen((current) => current ?? { stage: releaseStageOf(version), version });
    });
  }, []);

  const close = useCallback(() => {
    if (open) rememberOnboardingGuideSeen(open.stage, open.version);
    setOpen(null);
  }, [open]);

  return { open, show, close };
}
