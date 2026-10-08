/** Settings: whether a legacy product database password is remembered; forget it. */
// ADR-0094 (L3). The server keeps at most one, in a 0600 file under its
// configuration directory; this section never sees the value, only whether
// there is one, and removes it on request.
import { useEffect, useState } from "react";
import * as api from "./api";
import type { LegacyPasswordStatus } from "./api";
import { useTranslate } from "./i18n";

/** How Settings reaches the server's remembered password (injected, like the achievement tracker). */
export interface LegacyPasswordAccess {
  status: () => Promise<LegacyPasswordStatus>;
  forget: () => Promise<LegacyPasswordStatus>;
}

/** The real routes; `App` hands this to Settings. */
export const serverLegacyPassword: LegacyPasswordAccess = {
  status: () => api.legacyPasswordStatus(),
  forget: () => api.forgetLegacyPassword(),
};

export default function LegacyPasswordSettings(props: { access: LegacyPasswordAccess }) {
  const { access } = props;
  const t = useTranslate();
  const [status, setStatus] = useState<LegacyPasswordStatus | null>(null);
  const [failed, setFailed] = useState(false);
  const [busy, setBusy] = useState(false);
  const [forgot, setForgot] = useState(false);

  useEffect(() => {
    let live = true;
    access.status()
      .then((reply) => { if (live) setStatus(reply); })
      .catch(() => { if (live) setFailed(true); });
    return () => { live = false; };
  }, [access]);

  async function forget() {
    setBusy(true);
    setFailed(false);
    try {
      const reply = await access.forget();
      setStatus(reply);
      setForgot(reply.forgot === true);
    } catch {
      setFailed(true);
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="settings-section settings-section-legacy-password">
      <h3>{t("settings.section.legacyPassword")}</h3>
      <span className="settings-field-hint">{t("settings.legacyPasswordHint")}</span>
      {status && (
        <p className="settings-diagnostic" role="status">
          {forgot ? t("settings.legacyPasswordForgotten")
            : !status.available ? t("settings.legacyPasswordUnavailable")
            : status.remembered ? t("settings.legacyPasswordRemembered")
            : t("settings.legacyPasswordNone")}
        </p>
      )}
      {status?.problem && <p className="field-error">{status.problem}</p>}
      {status?.remembered && (
        <button type="button" onClick={() => void forget()} disabled={busy}>
          {t("settings.legacyPasswordForget")}
        </button>
      )}
      {failed && <p className="field-error" role="alert">{t("settings.legacyPasswordFailed")}</p>}
    </section>
  );
}
