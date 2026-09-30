/** Server-confirmed, default-off opt-in for the separate service-control procedure. */
import { useEffect, useRef, useState } from "react";
import { useTranslate } from "./i18n";
import { readPersistedBooleanSetting, setPersistedBooleanSetting } from "./settingsStore";

const KEY = "debugIndividualAddressWriteEnable";
type Status = "loading" | "ready" | "unavailable";
type ErrorKey = "settings.debugServiceControlUnavailable" | "settings.debugServiceControlNotSaved";

export default function ServiceControlDebugSetting() {
  const t = useTranslate();
  const alive = useRef(true);
  const generation = useRef(0);
  const [status, setStatus] = useState<Status>("loading");
  const [enabled, setEnabled] = useState(false);
  const [error, setError] = useState<ErrorKey | null>(null);

  async function refresh() {
    const active = ++generation.current;
    setStatus("loading");
    setEnabled(false);
    setError(null);
    try {
      const confirmed = await readPersistedBooleanSetting(KEY);
      if (alive.current && generation.current === active) {
        setEnabled(confirmed);
        setStatus("ready");
      }
    } catch {
      if (alive.current && generation.current === active) {
        setStatus("unavailable");
        setError("settings.debugServiceControlUnavailable");
      }
    }
  }

  useEffect(() => {
    alive.current = true;
    void refresh();
    return () => { alive.current = false; ++generation.current; };
  }, []);

  async function change(wanted: boolean) {
    if (status !== "ready") return;
    const active = ++generation.current;
    setStatus("loading");
    setError(null);
    try {
      await setPersistedBooleanSetting(KEY, wanted);
      if (alive.current && generation.current === active) {
        setEnabled(wanted);
        setStatus("ready");
      }
    } catch {
      if (alive.current && generation.current === active) {
        setEnabled(false);
        setStatus("unavailable");
        setError("settings.debugServiceControlNotSaved");
      }
    }
  }

  return <section className="settings-section settings-section-debug">
    <h3>{t("settings.section.debug")}</h3>
    <p className="settings-debug-warning">{t("settings.debugServiceControlWarning")}</p>
    <label className="settings-field settings-field-checkbox">
      <input type="checkbox" checked={enabled} disabled={status !== "ready"}
        onChange={(event) => void change(event.target.checked)} />
      <span className="settings-field-label">{t("settings.debugServiceControl")}</span>
    </label>
    {error && <p role="alert" className="form-error">{t(error)}</p>}
    {status === "unavailable" && <button className="settings-debug-retry" type="button" onClick={() => void refresh()}>
      {t("settings.debugServiceControlRetry")}
    </button>}
  </section>;
}
