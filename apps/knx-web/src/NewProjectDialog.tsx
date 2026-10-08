/** Stepped wizard that creates a project from scratch, with the honest 409 prompt. */
import { useEffect, useRef, useState } from "react";
import * as api from "./api";
import type { GroupAddressStyle } from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import Overlay from "./Overlay";
import { isWellFormedBcp47Tag, useAvailableLanguagePacks } from "./languagePack";
import { languageSelfName } from "./languageSelfName";
import { useTranslate, type MessageKey } from "./i18n";
import { AVAILABLE_UI_LANGUAGES, useUiLanguage } from "./uiLanguage";
import {
  defaultStructure,
  structureCounts,
  toSeed,
  validateStructure,
  type ProjectStructureDraft,
  type WizardStructureStep,
} from "./projectSeed";
import ProjectWizardTopology from "./ProjectWizardTopology";
import ProjectWizardBuilding from "./ProjectWizardBuilding";
import ProjectWizardGroups from "./ProjectWizardGroups";
import ProjectWizardReview from "./ProjectWizardReview";

// Wire values, in the order a user is most likely to want them: the ETS
// default first, the rest after. `newProject.style.<value>` is the label
// key per member (messages/en.ts), so this array and the catalogue cannot
// drift apart without failing `tsc`.
const STYLES: readonly GroupAddressStyle[] = ["ThreeLevel", "TwoLevel", "Free"];
// This is not a language tag (so it cannot collide with a custom BCP-47
// value). Project text languages are not limited to the UI catalogues.
const CUSTOM_LANGUAGE_OPTION = "__custom__";

type WizardStep = "details" | WizardStructureStep | "review";
const STEP_LABEL: Record<WizardStep, MessageKey> = {
  details: "projectWizard.step.details",
  topology: "projectWizard.step.topology",
  building: "projectWizard.step.building",
  groups: "projectWizard.step.groups",
  review: "projectWizard.step.review",
};

/** Free-style projects have no group ranges, so their wizard skips that step. */
function stepsFor(style: GroupAddressStyle): WizardStep[] {
  return style === "Free"
    ? ["details", "topology", "building", "review"]
    : ["details", "topology", "building", "groups", "review"];
}

/** The first line of the first installation, where "Add devices now" points. */
function firstLineId(tree: ProjectTree): number | null {
  for (const installation of tree.installations) {
    for (const area of installation.topology) {
      if (area.lines.length > 0) return area.lines[0].id;
    }
  }
  return null;
}

/**
 * The only way to a project that never came from a file — an ETS import
 * and a `.knxdb` open are the other two — and therefore the entry point
 * for installing a device from the product catalogue with no `.knxproj`
 * anywhere in sight. `POST /api/project/new` has existed and been tested
 * since 2026-09-08; until now nothing in the UI called it.
 *
 * Every field is pre-filled with something valid, so the fast path is
 * "open it, press Enter". The group address style is still asked rather
 * than defaulted quietly, because it is the field a reader thinks in and a
 * wrong one is noticed a hundred addresses later — but as of T4 it is no
 * longer a one-way door: `SetGroupAddressStyle` restyles a project after
 * the fact, and the style is a rendering choice rather than a capacity
 * limit, so no address is lost either way.
 *
 * Built on the shared `Overlay` shell (T31) — `role="dialog"`, Escape,
 * focus trap and focus restoration all come from there.
 *
 * Since ADR-0093 it is a wizard: after the details come optional steps for
 * a starting topology (area 1 / line 1.1 pre-filled), a building tree and a
 * group-range skeleton, then a review. "Create project" is available on
 * every step, so the fast path is unchanged; the structure travels as one
 * `seed` the server applies together with the new project, or refuses
 * without replacing anything. After success the wizard stays open on a
 * short "created, not yet saved" page that offers adding devices.
 */
export default function NewProjectDialog(props: {
  onCreated: (tree: ProjectTree) => void;
  onClose: () => void;
  /**
   * The workbench's own Save (ISSUE-04 Save-and-create). Resolves `true`
   * only when the open project is on disk *and* clean; anything else — a
   * failed save, a cancelled Save-As picker, a save that raced a newer
   * edit — keeps this prompt up and creates nothing.
   */
  onSaveFirst: () => Promise<boolean>;
  /** "Add devices now" on the final page: the new project's first line, or `null`. */
  onAddDevices: (lineId: number | null) => void;
}) {
  const { onCreated, onClose, onSaveFirst, onAddDevices } = props;
  const t = useTranslate();
  const [uiLanguage] = useUiLanguage();
  const languagePacks = useAvailableLanguagePacks().filter(
    ({ tag }) => !(AVAILABLE_UI_LANGUAGES as readonly string[]).includes(tag),
  );
  const listedLanguages: readonly string[] = [
    ...AVAILABLE_UI_LANGUAGES,
    ...languagePacks.map(({ tag }) => tag),
  ];
  // Seeded once, at open. A UI-language switch while the dialog is up must
  // not rewrite a name the user may already have typed over.
  const [name, setName] = useState(() => t("newProject.defaultName"));
  const [installationName, setInstallationName] = useState(() =>
    t("newProject.defaultInstallation"),
  );
  // The project's text language is not restricted to the UI catalogues.
  // Keep an arbitrary well-formed tag when one is active, or when the user
  // chooses "another language"; selecting a preset never discards it.
  const [languageChoice, setLanguageChoice] = useState(() =>
    listedLanguages.includes(uiLanguage) ? uiLanguage : CUSTOM_LANGUAGE_OPTION,
  );
  const [customLanguage, setCustomLanguage] = useState(() =>
    listedLanguages.includes(uiLanguage) ? "" : uiLanguage,
  );
  const customSelected = languageChoice === CUSTOM_LANGUAGE_OPTION || !listedLanguages.includes(languageChoice);
  const language = customSelected
    ? languageChoice === CUSTOM_LANGUAGE_OPTION ? customLanguage : languageChoice
    : languageChoice;
  const [style, setStyle] = useState<GroupAddressStyle>("ThreeLevel");
  const [step, setStep] = useState<WizardStep>("details");
  const [structure, setStructure] = useState<ProjectStructureDraft>(() => defaultStructure({
    area: (area) => t("projectWizard.topology.defaultArea", { area }),
    line: (area, line) => t("projectWizard.topology.defaultLine", { area, line }),
  }));
  // Anything typed or changed since opening; only then does leaving ask first.
  const [touched, setTouched] = useState(false);
  const [confirmDiscard, setConfirmDiscard] = useState(false);
  const [created, setCreated] = useState<{ name: string; lineId: number | null } | null>(null);
  const doneRef = useRef<HTMLButtonElement>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // The server's own 409 sentence, held rather than shown as an error:
  // this is a question ("shall I throw your edits away?"), and it is the
  // only state in which `discardChanges: true` may ever leave this file.
  const [conflict, setConflict] = useState<string | null>(null);
  const inFlightRef = useRef(false);
  // Set once the dialog is dismissed (Keep editing, Escape, backdrop) or
  // unmounted. Save and create awaits a save first; if the user walked
  // away meanwhile, that save still lands but nothing is created. Reset on
  // mount so StrictMode's simulated unmount/remount does not latch it.
  const dismissedRef = useRef(false);
  useEffect(() => {
    dismissedRef.current = false;
    return () => {
      dismissedRef.current = true;
    };
  }, []);
  function close() {
    dismissedRef.current = true;
    onClose();
  }
  // Entered data is not dropped on a single stray key: the first Escape or
  // backdrop click asks, a second Escape on the question keeps editing.
  function dismiss() {
    if (created || conflict !== null || !touched) {
      close();
    } else {
      setConfirmDiscard((showing) => !showing);
    }
  }
  // Cancel always leads to the question (or straight out with nothing typed).
  function cancel() {
    if (!touched) close();
    else setConfirmDiscard(true);
  }
  function edit<T>(setter: (value: T) => void): (value: T) => void {
    return (value) => {
      setTouched(true);
      setConfirmDiscard(false);
      setter(value);
    };
  }
  const nameRef = useRef<HTMLInputElement>(null);

  const trimmedName = name.trim();
  const trimmedLanguage = language.trim();
  const nameError = trimmedName === "" ? t("newProject.nameRequired") : null;
  // Well-formedness only, never a registry lookup — `knx_core::Language`
  // does not validate at all, and a private-use tag is a legitimate thing
  // to store texts under (same rule `languagePack.ts` applies to packs).
  const languageError = isWellFormedBcp47Tag(trimmedLanguage)
    ? null
    : t("newProject.languageInvalid");
  const structureIssues = validateStructure(structure, style);
  const steps = stepsFor(style);
  const visibleStep: WizardStep = steps.includes(step) ? step : "review";
  const stepIndex = steps.indexOf(visibleStep);
  const canSubmit = nameError === null && languageError === null && structureIssues.length === 0 && !busy;

  async function submit(discardChanges: boolean, saveFirst = false) {
    if (!canSubmit) return;
    // A second plain attempt while the conflict prompt is up would only
    // earn the same 409 back; the three buttons below are the only ways
    // out of that state.
    if (conflict !== null && !discardChanges && !saveFirst) return;
    if (inFlightRef.current) return;
    inFlightRef.current = true;
    setBusy(true);
    setError(null);
    try {
      // Save-and-create never sends `discardChanges`: if the save left
      // anything unsaved (or failed), the server's 409 simply comes back
      // and the prompt stays — nothing is thrown away on a guess.
      if (saveFirst && !(await onSaveFirst())) return;
      // Checked only before the request: once `newProject` is sent the
      // server has replaced the project, and the UI must follow it.
      if (dismissedRef.current) return;
      const tree = await api.newProject({
        name: trimmedName,
        installationName: installationName.trim(),
        language: trimmedLanguage,
        groupAddressStyle: style,
        discardChanges,
        seed: toSeed(structure),
      });
      onCreated(tree);
      setConflict(null);
      setCreated({ name: trimmedName, lineId: firstLineId(tree) });
    } catch (e) {
      if (api.isUnsavedChangesConflict(e)) {
        setConflict(api.errorMessage(e));
      } else {
        setError(api.errorMessage(e));
      }
    } finally {
      inFlightRef.current = false;
      setBusy(false);
    }
  }

  useEffect(() => {
    if (created) doneRef.current?.focus();
  }, [created]);

  function goTo(next: WizardStep) {
    setConfirmDiscard(false);
    setStep(next);
  }

  const setStructurePart = <K extends keyof ProjectStructureDraft>(key: K) =>
    edit((value: ProjectStructureDraft[K]) => setStructure((current) => ({ ...current, [key]: value })));
  const issuesFor = (which: WizardStructureStep) => structureIssues.filter((issue) => issue.step === which);

  if (created) {
    return (
      <Overlay labelledBy="new-project-title" className="new-project-panel" onClose={close} initialFocusRef={doneRef}>
        <h2 className="settings-panel-title" id="new-project-title">{t("projectWizard.done.title")}</h2>
        <p role="status">{t("projectWizard.done.body", { name: created.name })}</p>
        <div className="new-project-actions">
          <button type="button" onClick={() => { close(); onAddDevices(created.lineId); }}>
            {t("projectWizard.done.addDevices")}
          </button>
          <button ref={doneRef} type="button" className="primary-action" onClick={close}>
            {t("projectWizard.done.close")}
          </button>
        </div>
      </Overlay>
    );
  }

  return (
    <Overlay labelledBy="new-project-title" className="new-project-panel project-wizard" onClose={dismiss} initialFocusRef={nameRef}>
      <h2 className="settings-panel-title" id="new-project-title">
        {t("newProject.title")}
      </h2>
      <ol className="project-wizard-steps" aria-label={t("projectWizard.stepsLabel")}>
        {steps.map((which, index) => {
          const problems = which === "details"
            ? Number(nameError !== null) + Number(languageError !== null)
            : which === "review" ? 0 : issuesFor(which).length;
          return (
            <li key={which}>
              <button type="button" aria-current={which === visibleStep ? "step" : undefined}
                className={problems > 0 ? "project-wizard-step-link has-issues" : "project-wizard-step-link"}
                onClick={() => goTo(which)}>
                <span className="project-wizard-step-number" aria-hidden="true">{index + 1}</span>
                {t(STEP_LABEL[which])}
                {problems > 0 && <span className="project-wizard-step-issues">{t("projectWizard.stepIssues", { count: problems })}</span>}
              </button>
            </li>
          );
        })}
      </ol>
      <form
        onKeyDown={(e) => {
          // Enter creates the project from the details and the review only;
          // in a structure editor it would fire halfway through typing a
          // room name. Blocking the key also blocks the browser's implicit
          // submission, which would otherwise "click" Create.
          const structureStep = visibleStep !== "details" && visibleStep !== "review";
          if (structureStep && e.key === "Enter" && e.target instanceof HTMLInputElement) e.preventDefault();
        }}
        onSubmit={(e) => {
          e.preventDefault();
          if (visibleStep === "details" || visibleStep === "review") void submit(false);
        }}
      >
        <h3 className="project-wizard-step-title">
          {t("projectWizard.stepOf", { current: stepIndex + 1, total: steps.length, step: t(STEP_LABEL[visibleStep]) })}
        </h3>
        {visibleStep === "details" && (
          <>
            <p className="new-project-intro">{t("newProject.intro")}</p>
            <p className="new-project-filename-hint" id="new-project-filename-hint">
              {t("newProject.filenameHint")}
            </p>
          <label className="settings-field">
            <span className="settings-field-label">{t("newProject.name")}</span>
            <input
              ref={nameRef}
              value={name}
              aria-label={t("newProject.name")}
              aria-describedby="new-project-filename-hint"
              aria-invalid={nameError !== null}
              // Select-on-focus so the seeded default is one keystroke from
              // gone — it exists to make Enter work, not to be deleted by
              // hand first.
              onFocus={(e) => e.currentTarget.select()}
              onChange={(e) => edit(setName)(e.target.value)}
            />
            {nameError && <span className="field-error">{nameError}</span>}
          </label>
          <label className="settings-field">
            <span className="settings-field-label">{t("newProject.installation")}</span>
            <input
              value={installationName}
              aria-label={t("newProject.installation")}
              aria-describedby="new-project-filename-hint"
              onChange={(e) => edit(setInstallationName)(e.target.value)}
            />
          </label>
          <label className="settings-field">
            <span className="settings-field-label">{t("newProject.language")}</span>
            <select
              value={customSelected ? CUSTOM_LANGUAGE_OPTION : languageChoice}
              aria-label={t("newProject.language")}
              aria-describedby="new-project-language-hint"
              onChange={(e) => edit(setLanguageChoice)(e.target.value)}
            >
              {AVAILABLE_UI_LANGUAGES.map((tag) => (
                <option key={tag} value={tag}>{languageSelfName(tag)}</option>
              ))}
              {languagePacks.map((pack) => (
                <option key={pack.tag} value={pack.tag}>{pack.name}</option>
              ))}
              <option value={CUSTOM_LANGUAGE_OPTION}>{t("newProject.languageOther")}</option>
            </select>
            {customSelected && (
              <input
                value={language}
                aria-label={t("newProject.customLanguage")}
                aria-invalid={languageError !== null}
                aria-describedby={languageError ? "new-project-language-hint new-project-language-error" : "new-project-language-hint"}
                onChange={(e) => { setTouched(true); setLanguageChoice(CUSTOM_LANGUAGE_OPTION); setCustomLanguage(e.target.value); }}
              />
            )}
            <span className="settings-field-hint" id="new-project-language-hint">{t("newProject.languageHint")}</span>
            {languageError && <span className="field-error" id="new-project-language-error" role="alert">{languageError}</span>}
          </label>
          <label className="settings-field">
            <span className="settings-field-label">{t("newProject.style")}</span>
            <select
              value={style}
              aria-label={t("newProject.style")}
              onChange={(e) => edit(setStyle)(e.target.value as GroupAddressStyle)}
            >
              {STYLES.map((option) => (
                <option key={option} value={option}>
                  {t(`newProject.style.${option}`)}
                </option>
              ))}
            </select>
            <span className="settings-field-hint">{t("newProject.styleHint")}</span>
          </label>
          </>
        )}
        {visibleStep === "topology" && (
          <ProjectWizardTopology areas={structure.areas} issues={issuesFor("topology")}
            onChange={setStructurePart("areas")} t={t} />
        )}
        {visibleStep === "building" && (
          <ProjectWizardBuilding buildings={structure.buildings} issues={issuesFor("building")}
            onChange={setStructurePart("buildings")} t={t} />
        )}
        {visibleStep === "groups" && (
          <ProjectWizardGroups groupRanges={structure.groupRanges} buildings={structure.buildings} style={style}
            issues={issuesFor("groups")} onChange={setStructurePart("groupRanges")} t={t} />
        )}
        {visibleStep === "review" && (
          <ProjectWizardReview name={name} installationName={installationName} language={language} style={style}
            counts={structureCounts(structure)} issues={structureIssues} onGoTo={goTo} t={t} />
        )}
        {confirmDiscard && (
          <section className="project-wizard-discard" role="alert">
            <p><strong>{t("projectWizard.discard.title")}</strong> {t("projectWizard.discard.body")}</p>
            <div className="new-project-actions">
              <button type="button" onClick={() => setConfirmDiscard(false)}>{t("projectWizard.discard.keep")}</button>
              <button type="button" onClick={close}>{t("projectWizard.discard.confirm")}</button>
            </div>
          </section>
        )}
        {conflict === null && (
          <div className="new-project-actions">
            <button type="button" onClick={cancel}>
              {t("newProject.cancel")}
            </button>
            <button type="button" disabled={stepIndex === 0} onClick={() => goTo(steps[stepIndex - 1])}>
              {t("projectWizard.back")}
            </button>
            {stepIndex < steps.length - 1 && (
              <button type="button" onClick={() => goTo(steps[stepIndex + 1])}>
                {t("projectWizard.next")}
              </button>
            )}
            <button type="submit" className="primary-action" disabled={!canSubmit}
              onClick={(e) => {
                // A click is always a deliberate create, whatever the step.
                e.preventDefault();
                void submit(false);
              }}>
              {busy ? t("newProject.creating") : t("newProject.create")}
            </button>
          </div>
        )}
        {conflict === null && structureIssues.length > 0 && (
          <p className="field-error">{t("projectWizard.issueSummary", { count: structureIssues.length })}</p>
        )}
      </form>
      {conflict !== null && (
        // `role="alert"`, not `status`: this appears in response to the
        // user's own action and asks for a decision about work that
        // cannot be recovered once made.
        <section className="new-project-conflict" role="alert">
          <h3>{t("newProject.conflictTitle")}</h3>
          <p>{t("newProject.conflictBody")}</p>
          {/* The server's own wording, kept verbatim underneath the
              translated explanation rather than dropped — it names the
              two ways out (save first, or resend discarding). */}
          <p className="new-project-conflict-detail">{conflict}</p>
          <div className="new-project-actions">
            <button type="button" onClick={dismiss}>
              {t("newProject.conflictKeep")}
            </button>
            {/* `!canSubmit`, not `busy`: the fields stay editable while the
                prompt is up, so a name blanked here would make `submit()`
                return silently and the button would do nothing at all. */}
            <button type="button" disabled={!canSubmit} onClick={() => void submit(true)}>
              {busy ? t("newProject.creating") : t("newProject.conflictDiscard")}
            </button>
            <button
              type="button"
              className="primary-action new-project-save-and-create"
              disabled={!canSubmit}
              onClick={() => void submit(false, true)}
            >
              {busy ? t("newProject.creating") : t("newProject.conflictSave")}
            </button>
          </div>
        </section>
      )}
      {error && <span className="field-error">{error}</span>}
    </Overlay>
  );
}
