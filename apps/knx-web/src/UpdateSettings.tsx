/** Persists the user's public-release discovery preference through the existing settings store. */
import { getSetting, setSetting, useSettingsRevision } from "./settingsStore";
import { useTranslate } from "./i18n";
import { UPDATE_CHECK_SETTING, releaseChecksEnabled } from "./releaseUpdatePreference";
export default function UpdateSettings() {
  const t = useTranslate(); useSettingsRevision();
  return <section className="settings-section settings-section-updates">
    <h2 id="settings-updates" tabIndex={-1}>{t("updates.section")}</h2>
    <label className="settings-field settings-field-checkbox">
      <input type="checkbox" checked={releaseChecksEnabled(getSetting(UPDATE_CHECK_SETTING))} onChange={event => setSetting(UPDATE_CHECK_SETTING, event.target.checked)} />
      <span className="settings-field-label">{t("updates.enabled")}</span>
    </label>
    <p className="settings-field-hint">{t("updates.privacy")}</p>
  </section>;
}
