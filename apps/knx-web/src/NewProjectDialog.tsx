/** Overlay that creates a project from scratch, including the honest 409 unsaved-changes prompt. */
import { useEffect, useRef, useState } from "react";
import * as api from "./api";
import type { GroupAddressStyle } from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import Overlay from "./Overlay";
import { isWellFormedBcp47Tag, useAvailableLanguagePacks } from "./languagePack";
import { languageSelfName } from "./languageSelfName";
import { useTranslate } from "./i18n";
import { AVAILABLE_UI_LANGUAGES, useUiLanguage } from "./uiLanguage";

// Wire values, in the order a user is most likely to want them: the ETS
// default first, the rest after. `newProject.style.<value>` is the label
// key per member (messages/en.ts), so this array and the catalogue cannot
// drift apart without failing `tsc`.
const STYLES: readonly GroupAddressStyle[] = ["ThreeLevel", "TwoLevel", "Free"];
// This is not a language tag (so it cannot collide with a custom BCP-47
// value). Project text languages are not limited to the UI catalogues.
const CUSTOM_LANGUAGE_OPTION = "__custom__";

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
}) {
  const { onCreated, onClose, onSaveFirst } = props;
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
  function dismiss() {
    dismissedRef.current = true;
    onClose();
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
  const canSubmit = nameError === null && languageError === null && !busy;

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
      });
      onCreated(tree);
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

  return (
    <Overlay labelledBy="new-project-title" className="new-project-panel" onClose={dismiss} initialFocusRef={nameRef}>
      <h2 className="settings-panel-title" id="new-project-title">
        {t("newProject.title")}
      </h2>
      <p className="new-project-intro">{t("newProject.intro")}</p>
      <p className="new-project-filename-hint" id="new-project-filename-hint">
        {t("newProject.filenameHint")}
      </p>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          void submit(false);
        }}
      >
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
            onChange={(e) => setName(e.target.value)}
          />
          {nameError && <span className="field-error">{nameError}</span>}
        </label>
        <label className="settings-field">
          <span className="settings-field-label">{t("newProject.installation")}</span>
          <input
            value={installationName}
            aria-label={t("newProject.installation")}
            aria-describedby="new-project-filename-hint"
            onChange={(e) => setInstallationName(e.target.value)}
          />
        </label>
        <label className="settings-field">
          <span className="settings-field-label">{t("newProject.language")}</span>
          <select
            value={customSelected ? CUSTOM_LANGUAGE_OPTION : languageChoice}
            aria-label={t("newProject.language")}
            aria-describedby="new-project-language-hint"
            onChange={(e) => setLanguageChoice(e.target.value)}
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
              onChange={(e) => { setLanguageChoice(CUSTOM_LANGUAGE_OPTION); setCustomLanguage(e.target.value); }}
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
            onChange={(e) => setStyle(e.target.value as GroupAddressStyle)}
          >
            {STYLES.map((option) => (
              <option key={option} value={option}>
                {t(`newProject.style.${option}`)}
              </option>
            ))}
          </select>
          <span className="settings-field-hint">{t("newProject.styleHint")}</span>
        </label>
        {conflict === null && (
          <div className="new-project-actions">
            <button type="button" onClick={dismiss}>
              {t("newProject.cancel")}
            </button>
            <button type="submit" className="primary-action" disabled={!canSubmit}>
              {busy ? t("newProject.creating") : t("newProject.create")}
            </button>
          </div>
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
