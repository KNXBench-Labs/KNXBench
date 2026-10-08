/** ADR-0094 (L3): asks for a legacy product database password; offers to remember it. */
import { useRef, useState } from "react";
import Overlay from "./Overlay";
import { useTranslate } from "./i18n";
import type { MessageKey } from "./i18n";
import type { LegacyPasswordRefusal } from "./legacyInstall";

const REASON: Record<LegacyPasswordRefusal, MessageKey> = {
  required: "legacyPassword.required",
  wrong: "legacyPassword.wrong",
  rememberedDoesNotFit: "legacyPassword.rememberedDoesNotFit",
  rememberedUnusable: "legacyPassword.rememberedUnusable",
};

export default function LegacyPasswordDialog(props: {
  fileName: string;
  reason: LegacyPasswordRefusal;
  onSubmit: (password: string, remember: boolean) => void;
  onCancel: () => void;
}) {
  const { fileName, reason, onSubmit, onCancel } = props;
  const t = useTranslate();
  const fieldRef = useRef<HTMLInputElement | null>(null);
  // Held only here until handed to the one import request; the server keeps
  // it only when "remember" is ticked and the file opened with it.
  const [password, setPassword] = useState("");
  const [remember, setRemember] = useState(false);

  return (
    <Overlay labelledBy="legacy-password-title" className="project-password-panel" onClose={onCancel}
      initialFocusRef={fieldRef}>
      <h2 className="settings-panel-title" id="legacy-password-title">
        {t("legacyPassword.title", { file: fileName })}
      </h2>
      <p>{t(REASON[reason])}</p>
      <form onSubmit={(event) => {
        event.preventDefault();
        if (password === "") return;
        const entered = password;
        setPassword("");
        onSubmit(entered, remember);
      }}>
        <label className="settings-field">
          <span className="settings-field-label">{t("legacyPassword.label")}</span>
          <input ref={fieldRef} type="password" autoComplete="off" value={password}
            aria-invalid={reason === "wrong"} onChange={(event) => setPassword(event.target.value)} />
        </label>
        <label className="settings-field settings-field-checkbox">
          <input type="checkbox" checked={remember} onChange={(event) => setRemember(event.target.checked)} />
          <span className="settings-field-label">{t("legacyPassword.remember")}</span>
        </label>
        <p className="settings-field-hint">{t("legacyPassword.rememberHint")}</p>
        <div className="dialog-actions">
          <button type="button" onClick={onCancel}>{t("legacyPassword.cancel")}</button>
          <button type="submit" disabled={password === ""}>{t("legacyPassword.import")}</button>
        </div>
      </form>
    </Overlay>
  );
}
