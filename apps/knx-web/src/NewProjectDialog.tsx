/** Overlay that creates a project from scratch, including the honest 409 unsaved-changes prompt. */
import { useRef, useState } from "react";
import * as api from "./api";
import type { GroupAddressStyle } from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import Overlay from "./Overlay";
import { isWellFormedBcp47Tag } from "./languagePack";
import { useTranslate } from "./i18n";
import { useUiLanguage } from "./uiLanguage";

// Wire values, in the order a user is most likely to want them: the ETS
// default first, the rest after. `newProject.style.<value>` is the label
// key per member (messages/en.ts), so this array and the catalogue cannot
// drift apart without failing `tsc`.
const STYLES: readonly GroupAddressStyle[] = ["ThreeLevel", "TwoLevel", "Free"];

/**
 * The only way to a project that never came from a file — an ETS import
 * and a `.knxdb` open are the other two — and therefore the entry point
 * for installing a device from the product catalogue with no `.knxproj`
 * anywhere in sight. `POST /api/project/new` has existed and been tested
 * since 2026-09-08; until now nothing in the UI called it.
 *
 * Every field is pre-filled with something valid, so the fast path is
 * "open it, press Enter". The group address style is the one field that
 * genuinely has to be asked rather than defaulted quietly: nothing in the
 * domain restyles a project after group addresses exist, so a wrong
 * default is discovered a hundred addresses too late.
 *
 * Built on the shared `Overlay` shell (T31) — `role="dialog"`, Escape,
 * focus trap and focus restoration all come from there.
 */
export default function NewProjectDialog(props: {
  onCreated: (tree: ProjectTree) => void;
  onClose: () => void;
}) {
  const { onCreated, onClose } = props;
  const t = useTranslate();
  const [uiLanguage] = useUiLanguage();
  // Seeded once, at open. A UI-language switch while the dialog is up must
  // not rewrite a name the user may already have typed over.
  const [name, setName] = useState(() => t("newProject.defaultName"));
  const [installationName, setInstallationName] = useState(() =>
    t("newProject.defaultInstallation"),
  );
  // The project's own text language, not the chrome's — they merely start
  // out agreeing, because a user writing German labels is usually reading
  // a German interface.
  const [language, setLanguage] = useState(uiLanguage);
  const [style, setStyle] = useState<GroupAddressStyle>("ThreeLevel");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // The server's own 409 sentence, held rather than shown as an error:
  // this is a question ("shall I throw your edits away?"), and it is the
  // only state in which `discardChanges: true` may ever leave this file.
  const [conflict, setConflict] = useState<string | null>(null);
  const inFlightRef = useRef(false);
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

  async function submit(discardChanges: boolean) {
    if (!canSubmit) return;
    // A second non-discarding attempt while the conflict prompt is up
    // would only earn the same 409 back; the two buttons below are the
    // only ways out of that state.
    if (conflict !== null && !discardChanges) return;
    if (inFlightRef.current) return;
    inFlightRef.current = true;
    setBusy(true);
    setError(null);
    try {
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
    <Overlay labelledBy="new-project-title" className="new-project-panel" onClose={onClose} initialFocusRef={nameRef}>
      <h2 className="settings-panel-title" id="new-project-title">
        {t("newProject.title")}
      </h2>
      <p className="new-project-intro">{t("newProject.intro")}</p>
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
            onChange={(e) => setInstallationName(e.target.value)}
          />
        </label>
        <label className="settings-field">
          <span className="settings-field-label">{t("newProject.language")}</span>
          <input
            value={language}
            aria-label={t("newProject.language")}
            aria-invalid={languageError !== null}
            onChange={(e) => setLanguage(e.target.value)}
          />
          <span className="settings-field-hint">{t("newProject.languageHint")}</span>
          {languageError && <span className="field-error">{languageError}</span>}
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
            <button type="button" onClick={onClose}>
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
            <button type="button" onClick={onClose}>
              {t("newProject.conflictKeep")}
            </button>
            <button type="button" disabled={busy} onClick={() => void submit(true)}>
              {busy ? t("newProject.creating") : t("newProject.conflictDiscard")}
            </button>
          </div>
        </section>
      )}
      {error && <span className="field-error">{error}</span>}
    </Overlay>
  );
}
