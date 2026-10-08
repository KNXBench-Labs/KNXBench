/** Read-only summary step of the new-project wizard, with links back to steps that need fixing. */
import type { GroupAddressStyle } from "./api";
import type { MessageKey, Translate } from "./i18n";
import type { StructureCounts, StructureIssue, WizardStructureStep } from "./projectSeed";

export const STRUCTURE_STEP_LABEL: Record<WizardStructureStep, MessageKey> = {
  topology: "projectWizard.step.topology",
  building: "projectWizard.step.building",
  groups: "projectWizard.step.groups",
};

export default function ProjectWizardReview(props: {
  name: string;
  installationName: string;
  language: string;
  style: GroupAddressStyle;
  counts: StructureCounts;
  issues: readonly StructureIssue[];
  onGoTo: (step: WizardStructureStep) => void;
  t: Translate;
}) {
  const { name, installationName, language, style, counts, issues, onGoTo, t } = props;
  const stepsWithIssues = (Object.keys(STRUCTURE_STEP_LABEL) as WizardStructureStep[])
    .filter((step) => issues.some((issue) => issue.step === step));
  return (
    <div className="project-wizard-step">
      <p className="project-wizard-intro">{t("projectWizard.review.intro")}</p>
      <dl className="project-wizard-summary">
        <dt>{t("newProject.name")}</dt><dd>{name.trim()}</dd>
        <dt>{t("newProject.installation")}</dt><dd>{installationName.trim()}</dd>
        <dt>{t("newProject.language")}</dt><dd>{language.trim()}</dd>
        <dt>{t("newProject.style")}</dt><dd>{t(`newProject.style.${style}`)}</dd>
        <dt>{t("projectWizard.review.areas")}</dt><dd>{counts.areas}</dd>
        <dt>{t("projectWizard.review.lines")}</dt><dd>{counts.lines}</dd>
        <dt>{t("projectWizard.review.buildingParts")}</dt><dd>{counts.buildingParts}</dd>
        <dt>{t("projectWizard.review.groupRanges")}</dt><dd>{counts.groupRanges}</dd>
      </dl>
      {stepsWithIssues.length === 0 ? (
        <p className="project-wizard-ready">{t("projectWizard.review.ready")}</p>
      ) : (
        <div role="alert">
          <p className="field-error">{t("projectWizard.review.issues", { count: issues.length })}</p>
          <ul className="project-wizard-issue-links">
            {stepsWithIssues.map((step) => (
              <li key={step}>
                <button type="button" onClick={() => onGoTo(step)}>
                  {t("projectWizard.review.goTo", { step: t(STRUCTURE_STEP_LABEL[step]) })}
                </button>
              </li>
            ))}
          </ul>
        </div>
      )}
      <p className="settings-field-hint">{t("projectWizard.review.notSaved")}</p>
    </div>
  );
}
