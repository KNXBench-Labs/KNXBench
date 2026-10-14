/** Asks before Save As replaces a file that already exists. */
// Owner request 2026-10-14. Only the browser's server-mount picker needs
// it: the desktop's native save dialog asks by itself (rfd's GTK backend
// sets `do_overwrite_confirmation`). The server refuses an unconfirmed
// replacement with `409 fileExists`, so this dialog is the user's answer
// to a refusal, never the only thing standing between a click and a lost
// file. It reuses the unsaved-changes dialog's frame and button roles.
import Overlay from "./Overlay";
import { useTranslate } from "./i18n";
import "./UnsavedChangesDialog.css";

export default function OverwriteConfirmDialog(props: { path: string; onAnswer: (replace: boolean) => void }) {
  const t = useTranslate();
  return (
    <Overlay labelledBy="overwrite-confirm-title" className="quit-confirm unsaved-changes-dialog overwrite-confirm"
      onClose={() => props.onAnswer(false)}>
      <h2 id="overwrite-confirm-title">{t("overwrite.title")}</h2>
      <p>{t("overwrite.message", { path: props.path })}</p>
      <p className="quit-confirm-hint">{t("overwrite.history")}</p>
      {/* Cancel first, so the initial focus lands on the answer that loses nothing. */}
      <footer className="quit-confirm-footer unsaved-changes-actions">
        <button type="button" className="dialog-action dialog-action--secondary" onClick={() => props.onAnswer(false)}>
          {t("overwrite.cancel")}
        </button>
        <button type="button" className="dialog-action dialog-action--danger overwrite-confirm-replace"
          onClick={() => props.onAnswer(true)}>
          {t("overwrite.replace")}
        </button>
      </footer>
    </Overlay>
  );
}
