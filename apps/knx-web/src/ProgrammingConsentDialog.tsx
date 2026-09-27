/** The programming confirmation, naming the build's release stage before any write. */
// `useProgrammingConsent.ts` opens this; `programmingConsent.ts` owns the
// stage and the remembered answer. Cancel, Escape and the backdrop all
// mean "no": a question about writing to a building defaults to not
// writing. The confirm button carries no initial focus, so an Enter
// pressed for something else cannot program a device.
import { useRef, useState } from "react";
import Overlay from "./Overlay";
import { useTranslate } from "./i18n";
import type { TranslatableKey } from "./i18n";
import type { ReleaseStage } from "./programmingConsent";

/** The stage as the dialog (and the settings panel) names it. */
export const STAGE_LABEL: Record<ReleaseStage, TranslatableKey> = {
  alpha: "programmingConsent.stage.alpha",
  beta: "programmingConsent.stage.beta",
  releaseCandidate: "programmingConsent.stage.releaseCandidate",
  stable: "programmingConsent.stage.stable",
  preRelease: "programmingConsent.stage.preRelease",
  unknown: "programmingConsent.stage.unknown",
};

/** What the stage means for the user, one sentence per stage. */
const STAGE_RISK: Record<ReleaseStage, TranslatableKey> = {
  alpha: "programmingConsent.risk.alpha",
  beta: "programmingConsent.risk.beta",
  releaseCandidate: "programmingConsent.risk.releaseCandidate",
  stable: "programmingConsent.risk.stable",
  preRelease: "programmingConsent.risk.preRelease",
  unknown: "programmingConsent.risk.unknown",
};

/** Stages for which "don't ask again" is offered — see `programmingConsent.ts`. */
const OFFERS_REMEMBER: ReadonlySet<ReleaseStage> = new Set(["alpha", "beta", "releaseCandidate", "stable"]);

export interface ProgrammingConsentDialogProps {
  stage: ReleaseStage;
  /** The build version as the server reported it, or `null` when it did not. */
  version: string | null;
  /** What is about to be programmed, e.g. "1.1.67 — Switch actuator". Shown verbatim. */
  target: string;
  /** Called with the checkbox's state when the user confirms. */
  onConfirm: (remember: boolean) => void;
  onCancel: () => void;
}

export default function ProgrammingConsentDialog(props: ProgrammingConsentDialogProps) {
  const { stage, version, target, onConfirm, onCancel } = props;
  const t = useTranslate();
  const [remember, setRemember] = useState(false);
  const cancelRef = useRef<HTMLButtonElement | null>(null);
  const stageLabel = t(STAGE_LABEL[stage]);

  return (
    <Overlay
      labelledBy="programming-consent-title"
      className="programming-consent-dialog"
      initialFocusRef={cancelRef}
      onClose={onCancel}
    >
      <h2 id="programming-consent-title">{t("programmingConsent.title")}</h2>
      <p className="programming-consent-target">
        {t("programmingConsent.target", { target })}
      </p>
      <dl className="programming-consent-facts">
        <dt>{t("programmingConsent.stageLabel")}</dt>
        <dd className={`programming-consent-stage programming-consent-stage-${stage}`}>{stageLabel}</dd>
        <dt>{t("programmingConsent.versionLabel")}</dt>
        <dd className="programming-consent-version">
          {version ?? t("programmingConsent.versionUnknown")}
        </dd>
      </dl>
      <p className="programming-consent-risk">{t(STAGE_RISK[stage])}</p>
      <p>{t("programmingConsent.backup")}</p>
      {OFFERS_REMEMBER.has(stage) ? (
        <label className="settings-field settings-field-checkbox programming-consent-remember">
          <input
            type="checkbox"
            checked={remember}
            onChange={(e) => setRemember(e.target.checked)}
          />
          <span>{t("programmingConsent.remember", { stage: stageLabel })}</span>
        </label>
      ) : (
        <p className="programming-consent-no-remember">{t("programmingConsent.rememberUnavailable")}</p>
      )}
      <footer className="programming-consent-footer">
        <button type="button" ref={cancelRef} onClick={onCancel}>
          {t("programmingConsent.cancel")}
        </button>
        <button
          type="button"
          className="programming-consent-confirm"
          onClick={() => onConfirm(OFFERS_REMEMBER.has(stage) && remember)}
        >
          {t("programmingConsent.confirm")}
        </button>
      </footer>
    </Overlay>
  );
}
