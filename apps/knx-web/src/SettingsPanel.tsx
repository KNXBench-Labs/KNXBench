import type { ThemeDef } from "./theme";
import type { MotionLevelDef, MotionStyleDef } from "./motion";
import type { ProductLanguage } from "./api";
import Overlay from "./Overlay";

/**
 * The application's one settings surface (design D32): Theme, Motion
 * style, Motion level and Product data language, each a registry-backed
 * `<select>` that applies immediately — no Save button, no reload.
 *
 * Built on the shared `Overlay` shell (T31), the same one `Search.tsx`
 * and `CommandPalette.tsx` use — dismissal by backdrop click and by
 * `Escape`, initial focus, a focus trap and focus restoration all come
 * from there now, closing the gap KNOWN_LIMITATIONS.md §20 used to
 * describe.
 *
 * Purely presentational (T26): `App.tsx` owns every setting's state and
 * fetches `productLanguages` itself; this component only renders props
 * and forwards `onChange`, same as the other three fields.
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

  return (
    <Overlay labelledBy="settings-panel-title" className="settings-panel" onClose={onClose}>
      <h2 className="settings-panel-title" id="settings-panel-title">Settings</h2>
      <label className="settings-field">
        <span className="settings-field-label">Theme</span>
        <select value={activeThemeId} onChange={(e) => onSelectTheme(e.target.value)} aria-label="Theme">
          {themes.map((t) => (
            <option key={t.id} value={t.id}>
              {t.name}
            </option>
          ))}
        </select>
      </label>
      <label className="settings-field">
        <span className="settings-field-label">Motion style</span>
        <select
          value={activeMotionStyle}
          onChange={(e) => onSelectMotionStyle(e.target.value)}
          aria-label="Motion style"
        >
          {motionStyles.map((s) => (
            <option key={s.id} value={s.id}>
              {s.name}
            </option>
          ))}
        </select>
      </label>
      <label className="settings-field">
        <span className="settings-field-label">Motion level</span>
        <select
          value={activeMotionLevel}
          onChange={(e) => onSelectMotionLevel(e.target.value)}
          aria-label="Motion level"
        >
          {motionLevels.map((l) => (
            <option key={l.id} value={l.id}>
              {l.name}
            </option>
          ))}
        </select>
      </label>
      <label className="settings-field">
        <span className="settings-field-label">Product data language</span>
        {productLanguages.length === 0 ? (
          <select aria-label="Product data language" disabled>
            <option>No product database installed</option>
          </select>
        ) : (
          <select
            value={activeProductLanguage ?? ""}
            onChange={(e) => onSelectProductLanguage(e.target.value === "" ? null : e.target.value)}
            aria-label="Product data language"
          >
            <option value="">Package default</option>
            {productLanguages.map((l) => (
              <option key={l.language} value={l.language}>
                {`${l.language} (${l.rows} strings)`}
              </option>
            ))}
          </select>
        )}
      </label>
    </Overlay>
  );
}
