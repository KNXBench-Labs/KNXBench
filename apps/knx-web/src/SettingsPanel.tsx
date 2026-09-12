import type { ThemeDef } from "./theme";
import type { MotionLevelDef, MotionStyleDef } from "./motion";
import type { ProductLanguage } from "./api";
import { AVAILABLE_UI_LANGUAGES, useUiLanguage } from "./uiLanguage";
import type { UiLanguage } from "./uiLanguage";
import { useTranslate } from "./i18n";
import Overlay from "./Overlay";

/**
 * The application's one settings surface (design D32): Theme, Motion
 * style, Motion level, Product data language and UI language, each a
 * registry-backed `<select>` that applies immediately — no Save button,
 * no reload.
 *
 * Built on the shared `Overlay` shell (T31), the same one `Search.tsx`
 * and `CommandPalette.tsx` use — dismissal by backdrop click and by
 * `Escape`, initial focus, a focus trap and focus restoration all come
 * from there now, closing the gap KNOWN_LIMITATIONS.md §20 used to
 * describe.
 *
 * Purely presentational (T26) for the first four fields: `App.tsx` owns
 * their state and fetches `productLanguages` itself; this component only
 * renders props and forwards `onChange`. UI language (T25 task 2) is the
 * deliberate exception — it reads `uiLanguage.ts`'s shared store directly
 * via `useUiLanguage()` instead of taking it as a prop, precisely so that
 * changing it needs no `App.tsx` wiring and reaches every other mounted
 * consumer of that store (any future `useTranslate()` caller) without a
 * remount.
 */
export default function SettingsPanel(props: {
  themes: readonly ThemeDef[];
  activeThemeId: string;
  onSelectTheme: (id: string) => void;
  motionStyles: readonly MotionStyleDef[];
  activeMotionStyle: string;
  onSelectMotionStyle: (id: string) => void;
  motionLevels: readonly MotionLevelDef[];
  activeMotionLevel: string;
  onSelectMotionLevel: (id: string) => void;
  productLanguages: readonly ProductLanguage[];
  activeProductLanguage: string | null;
  onSelectProductLanguage: (language: string | null) => void;
  onClose: () => void;
}) {
  const {
    themes,
    activeThemeId,
    onSelectTheme,
    motionStyles,
    activeMotionStyle,
    onSelectMotionStyle,
    motionLevels,
    activeMotionLevel,
    onSelectMotionLevel,
    productLanguages,
    activeProductLanguage,
    onSelectProductLanguage,
    onClose,
  } = props;

  const [uiLanguage, setUiLanguage] = useUiLanguage();
  const t = useTranslate();

  return (
    <Overlay labelledBy="settings-panel-title" className="settings-panel" onClose={onClose}>
      <h2 className="settings-panel-title" id="settings-panel-title">
        {t("settings.title")}
      </h2>
      <label className="settings-field">
        <span className="settings-field-label">{t("settings.theme")}</span>
        <select
          value={activeThemeId}
          onChange={(e) => onSelectTheme(e.target.value)}
          aria-label={t("settings.theme")}
        >
          {themes.map((theme) => (
            <option key={theme.id} value={theme.id}>
              {theme.name}
            </option>
          ))}
        </select>
      </label>
      <label className="settings-field">
        <span className="settings-field-label">{t("settings.motionStyle")}</span>
        <select
          value={activeMotionStyle}
          onChange={(e) => onSelectMotionStyle(e.target.value)}
          aria-label={t("settings.motionStyle")}
        >
          {motionStyles.map((s) => (
            <option key={s.id} value={s.id}>
              {s.name}
            </option>
          ))}
        </select>
      </label>
      <label className="settings-field">
        <span className="settings-field-label">{t("settings.motionLevel")}</span>
        <select
          value={activeMotionLevel}
          onChange={(e) => onSelectMotionLevel(e.target.value)}
          aria-label={t("settings.motionLevel")}
        >
          {motionLevels.map((l) => (
            <option key={l.id} value={l.id}>
              {l.name}
            </option>
          ))}
        </select>
      </label>
      <label className="settings-field">
        <span className="settings-field-label">{t("settings.productDataLanguage")}</span>
        {productLanguages.length === 0 ? (
          <select aria-label={t("settings.productDataLanguage")} disabled>
            <option>{t("settings.noProductDatabase")}</option>
          </select>
        ) : (
          <select
            value={activeProductLanguage ?? ""}
            onChange={(e) => onSelectProductLanguage(e.target.value === "" ? null : e.target.value)}
            aria-label={t("settings.productDataLanguage")}
          >
            <option value="">{t("settings.packageDefault")}</option>
            {productLanguages.map((l) => (
              <option key={l.language} value={l.language}>
                {t("settings.productLanguageOption", { language: l.language, count: l.rows })}
              </option>
            ))}
          </select>
        )}
      </label>
      <label className="settings-field">
        <span className="settings-field-label">{t("settings.uiLanguage")}</span>
        <select
          value={uiLanguage}
          onChange={(e) => setUiLanguage(e.target.value as UiLanguage)}
          aria-label={t("settings.uiLanguage")}
        >
          {AVAILABLE_UI_LANGUAGES.map((language) => (
            <option key={language} value={language}>
              {t(`language.${language}`)}
            </option>
          ))}
        </select>
      </label>
    </Overlay>
  );
}
