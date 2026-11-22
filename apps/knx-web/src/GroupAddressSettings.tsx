/** The group-address settings section: the switch for last-download marks. */
// WP-48.5 (F07). The marks read every device's detail, so the preference is off by default.
import { useGaLastDownloadMarks } from "./deviceLastDownloads";
import { useTranslate } from "./i18n";

export default function GroupAddressSettings() {
  const t = useTranslate();
  const [enabled, setEnabled] = useGaLastDownloadMarks();
  return (
    <section className="settings-section settings-section-group-addresses">
      <h2 id="settings-group-addresses" tabIndex={-1}>{t("settings.section.groupAddresses")}</h2>
      <label className="settings-field settings-field-checkbox">
        <input type="checkbox" checked={enabled} onChange={(e) => setEnabled(e.target.checked)} />
        <span className="settings-field-label">{t("settings.gaLastDownloadMarks")}</span>
      </label>
      <span className="settings-field-hint">{t("settings.gaLastDownloadMarksHint")}</span>
    </section>
  );
}
