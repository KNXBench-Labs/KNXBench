/** Asks before a project change would drop an unapplied project-note draft. */
// A note draft is not a project modification, so the project's own
// "save first" dialog cannot speak for it: saving the project does not
// apply a draft. Two answers only; "Keep editing" comes first so Overlay's
// initial focus lands on the answer that loses nothing.
import Overlay from "./Overlay";
import { useTranslate } from "./i18n";
import "./UnsavedChangesDialog.css";

export default function NoteDraftGuardDialog({ onKeep, onDiscard }: { onKeep: () => void; onDiscard: () => void }) {
  const t = useTranslate();
  return (
    <Overlay labelledBy="note-draft-guard-title" className="quit-confirm unsaved-changes-dialog note-draft-guard" onClose={onKeep}>
      <h2 id="note-draft-guard-title">{t("notes.draftGuard.title")}</h2>
      <p>{t("notes.draftGuard.message")}</p>
      <footer className="quit-confirm-footer unsaved-changes-actions">
        <button type="button" className="dialog-action dialog-action--secondary" onClick={onKeep}>
          {t("notes.draftGuard.keep")}
        </button>
        <button type="button" className="dialog-action dialog-action--danger" onClick={onDiscard}>
          {t("notes.draftGuard.discard")}
        </button>
      </footer>
    </Overlay>
  );
}
