/** KL-43: independent, inherit-by-default category preferences. */
import { loadMotionCategory, saveMotionCategory, MOTION_CATEGORIES, type CategoryMotion } from "./motion";
import { settingsStorage, useSettingsRevision } from "./settingsStore";
import { useTranslate } from "./i18n";

export default function MotionCategorySettings() {
  const t = useTranslate();
  useSettingsRevision();
  return <fieldset className="motion-category-settings"><legend>{t("settings.motionCategories")}</legend>
    <p className="settings-hint">{t("settings.motionPriority")}</p>
    {MOTION_CATEGORIES.map(category => <label className="settings-field" key={category}>
      <span className="settings-field-label">{t(`settings.motionCategory.${category}`)}</span>
      <select data-motion-category={category} aria-label={t(`settings.motionCategory.${category}`)}
        value={loadMotionCategory(settingsStorage, category)}
        onChange={event => saveMotionCategory(settingsStorage, category, event.target.value as CategoryMotion)}>
        {(["inherit", "subtle", "off"] as const).map(value => <option key={value} value={value}>{t(`settings.categoryMotion.${value}`)}</option>)}
      </select>
    </label>)}
  </fieldset>;
}
