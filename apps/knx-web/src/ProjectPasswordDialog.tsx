/** AR08: asks for an ETS project password for one import; the secret lives only in this field. */
import { useRef, useState } from "react";
import Overlay from "./Overlay";
import { useTranslate } from "./i18n";
import type { ProjectPasswordRefusal } from "./projectPassword";

export default function ProjectPasswordDialog(props: {
  fileName: string;
  reason: ProjectPasswordRefusal;
  onSubmit: (password: string) => void;
  onCancel: () => void;
}) {
  const { fileName, reason, onSubmit, onCancel } = props;
  const t = useTranslate();
  const fieldRef = useRef<HTMLInputElement | null>(null);
  // The password is held only here, until it is handed to the one import
  // request that needs it; it never reaches settings, storage or a log.
  const [password, setPassword] = useState("");

  return (
    <Overlay labelledBy="project-password-title" className="project-password-panel" onClose={onCancel}
      initialFocusRef={fieldRef}>
      <h2 className="settings-panel-title" id="project-password-title">{t("projectPassword.title", { file: fileName })}</h2>
      <p>{reason === "wrong" ? t("projectPassword.wrong") : t("projectPassword.required")}</p>
      <form onSubmit={(event) => {
        event.preventDefault();
        if (password === "") return;
        const entered = password;
        setPassword("");
        onSubmit(entered);
      }}>
        <label className="settings-field">
          <span className="settings-field-label">{t("projectPassword.label")}</span>
          <input ref={fieldRef} type="password" autoComplete="off" value={password}
            aria-invalid={reason === "wrong"} onChange={(event) => setPassword(event.target.value)} />
        </label>
        <p className="settings-field-hint">{t("projectPassword.notStored")}</p>
        <div className="dialog-actions">
          <button type="button" onClick={onCancel}>{t("projectPassword.cancel")}</button>
          <button type="submit" disabled={password === ""}>{t("projectPassword.import")}</button>
        </div>
      </form>
    </Overlay>
  );
}
