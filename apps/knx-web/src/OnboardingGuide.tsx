/** The first-run guide: what this build is, what it does, where to start, where help is. */
// ADR-0084. `useOnboardingGuide` decides when this opens and remembers it
// as seen when it closes; this component only says its four pages. Built
// on `Overlay`, so the dialog role, focus trap, Escape and focus return
// are the shell's. Every way out (Skip, Escape, the backdrop, Get started,
// or one of the task buttons) is a close, and none of them asks anything:
// the guide is advice, not a gate.
//
// The task buttons run the command palette's own commands (`COMMANDS`),
// so the guide has no second route to anything. They close the guide
// first, so the dialog the command opens is the only one on screen.
import { emitAchievementEvent } from "./achievementEvents";
import { useEffect, useRef, useState } from "react";
import { COMMANDS, type CommandContext } from "./commandRegistry";
import { requestHelpTopic } from "./help";
import { useTranslate } from "./i18n";
import type { MessageKey } from "./messages/en";
import { ONBOARDING_TASKS } from "./onboardingGuide";
import Overlay from "./Overlay";
import type { ReleaseStage } from "./programmingConsent";
import { STAGE_LABEL } from "./ProgrammingConsentDialog";
import { AVAILABLE_UI_LANGUAGES, useUiLanguage } from "./uiLanguage";
import { BUNDLED_LANGUAGE_PACKS } from "./bundledLanguagePacks";
import { languageSelfName } from "./languageSelfName";

/** What the stage means, one paragraph each. Coarse on purpose (see `messages/en.ts`). */
const STAGE_MEANING: Record<ReleaseStage, MessageKey> = {
  alpha: "onboarding.stage.alpha",
  beta: "onboarding.stage.beta",
  releaseCandidate: "onboarding.stage.releaseCandidate",
  stable: "onboarding.stage.stable",
  preRelease: "onboarding.stage.preRelease",
  unknown: "onboarding.stage.unknown",
};

const WORKS: readonly MessageKey[] = [
  "onboarding.step.scope.works.import",
  "onboarding.step.scope.works.edit",
  "onboarding.step.scope.works.products",
  "onboarding.step.scope.works.bus",
  "onboarding.step.scope.works.export",
];

const NOT_YET: readonly MessageKey[] = [
  "onboarding.step.scope.notYet.export",
  "onboarding.step.scope.notYet.secure",
  "onboarding.step.scope.notYet.programming",
];

const STEP_TITLES: readonly MessageKey[] = [
  "onboarding.step.about.title",
  "onboarding.step.scope.title",
  "onboarding.step.start.title",
  "onboarding.step.help.title",
];

export interface OnboardingGuideProps {
  stage: ReleaseStage;
  /** The command palette's context; the task buttons run its commands. */
  ctx: CommandContext;
  onClose: () => void;
}

export default function OnboardingGuide(props: OnboardingGuideProps) {
  const { stage, ctx, onClose } = props;
  const t = useTranslate();
  const [language, setLanguage] = useUiLanguage();
  const [step, setStep] = useState(0);
  const nextRef = useRef<HTMLButtonElement | null>(null);
  const last = step === STEP_TITLES.length - 1;

  // Back disappears on the first page. If it had focus, focus would fall to
  // the document and out of the dialog; put it on Next instead.
  useEffect(() => {
    const active = document.activeElement;
    if (active === null || active === document.body) nextRef.current?.focus();
  }, [step]);

  function runCommand(commandId: string) {
    const command = COMMANDS.find((entry) => entry.id === commandId);
    onClose();
    if (command?.isEnabled(ctx)) command.run(ctx);
  }

  function openLimitsHelp() {
    onClose();
    requestHelpTopic("limits");
  }

  return (
    <Overlay labelledBy="onboarding-title" className="onboarding-guide" initialFocusRef={nextRef} onClose={onClose}>
      <header className="onboarding-header">
        <h2 id="onboarding-title">{t("onboarding.title")}</h2>
        <p className="onboarding-progress" aria-live="polite">
          {t("onboarding.progress", { current: step + 1, total: STEP_TITLES.length })}
        </p>
      </header>
      <section className="onboarding-step" aria-labelledby="onboarding-step-title">
        <h3 id="onboarding-step-title">{t(STEP_TITLES[step])}</h3>
        {step === 0 && (
          <>
            <p>{t("onboarding.step.about.p1")}</p>
            <dl className="onboarding-facts">
              <dt>{t("onboarding.step.about.stageLabel")}</dt>
              <dd className={`onboarding-stage onboarding-stage-${stage}`}>{t(STAGE_LABEL[stage])}</dd>
            </dl>
            <p className="onboarding-stage-meaning">{t(STAGE_MEANING[stage])}</p>
            <p>{t("onboarding.step.about.trust")}</p>
            <div className="onboarding-language" role="group" aria-labelledby="onboarding-language-label">
              <span id="onboarding-language-label">{t("onboarding.step.about.language")}</span>
              {AVAILABLE_UI_LANGUAGES.map((id) => (
                <button key={id} type="button" aria-pressed={language === id} onClick={() => setLanguage(id)}>
                  {languageSelfName(id)}
                </button>
              ))}
              {BUNDLED_LANGUAGE_PACKS.map((pack) => (
                <button key={pack.tag} type="button" aria-pressed={language === pack.tag} onClick={() => setLanguage(pack.tag)}>
                  {pack.name}
                </button>
              ))}
            </div>
          </>
        )}
        {step === 1 && (
          <>
            <h4>{t("onboarding.step.scope.worksTitle")}</h4>
            <ul className="onboarding-list onboarding-list-works">
              {WORKS.map((key) => <li key={key}>{t(key)}</li>)}
            </ul>
            <h4>{t("onboarding.step.scope.notYetTitle")}</h4>
            <ul className="onboarding-list onboarding-list-not-yet">
              {NOT_YET.map((key) => <li key={key}>{t(key)}</li>)}
            </ul>
            <button type="button" className="onboarding-link" onClick={openLimitsHelp}>
              {t("onboarding.step.scope.more")}
            </button>
          </>
        )}
        {step === 2 && (
          <>
            <p>{t("onboarding.step.start.intro")}</p>
            <div className="onboarding-tasks">
              {ONBOARDING_TASKS.map((task) => (
                <button
                  key={task.id}
                  type="button"
                  className="welcome-card onboarding-task"
                  // Named by its title alone, like the welcome cards; the
                  // sentence under it is the description, not the name.
                  aria-labelledby={`onboarding-task-${task.id}-title`}
                  aria-describedby={`onboarding-task-${task.id}-description`}
                  onClick={() => runCommand(task.commandId)}
                >
                  <span id={`onboarding-task-${task.id}-title`} className="welcome-card-title">{t(task.titleKey)}</span>
                  <span id={`onboarding-task-${task.id}-description`} className="welcome-card-description">
                    {t(task.descriptionKey)}
                  </span>
                </button>
              ))}
            </div>
            <p className="onboarding-hint">{t("onboarding.step.start.palette")}</p>
          </>
        )}
        {step === 3 && (
          <>
            <p>{t("onboarding.step.help.p1")}</p>
            <p>{t("onboarding.step.help.p2")}</p>
            <p>{t("onboarding.step.help.p3")}</p>
            <button type="button" className="onboarding-link" onClick={() => runCommand("open-help")}>
              {t("onboarding.step.help.openHelp")}
            </button>
          </>
        )}
      </section>
      <footer className="onboarding-footer">
        {!last && (
          <button type="button" className="onboarding-skip" onClick={onClose}>
            {t("onboarding.skip")}
          </button>
        )}
        {step > 0 && (
          <button type="button" onClick={() => setStep(step - 1)}>
            {t("onboarding.back")}
          </button>
        )}
        <button
          type="button"
          ref={nextRef}
          className="primary-action"
          onClick={() => {
            if (!last) { setStep(step + 1); return; }
            emitAchievementEvent({ type: "onboardingCompleted" });
            onClose();
          }}
        >
          {last ? t("onboarding.finish") : t("onboarding.next")}
        </button>
      </footer>
    </Overlay>
  );
}
