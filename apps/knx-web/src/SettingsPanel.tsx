import { useEffect } from "react";
import type { ThemeDef } from "./theme";
import type { MotionLevelDef, MotionStyleDef } from "./motion";

/**
 * The application's one settings surface (design D32): Theme, Motion
 * style and Motion level, each a registry-backed `<select>` that applies
 * immediately — no Save button, no reload.
 *
 * Deliberately reuses `Search.tsx`/`CommandPalette.tsx`'s overlay shape
 * (`.search-overlay` wrapping `.search-panel`, click-outside via
 * `onClick`/`stopPropagation`) rather than inventing a fourth overlay
 * implementation — see KNOWN_LIMITATIONS.md §20. It does not fix that
 * gap; it just declines to widen it.
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
    onClose,
  } = props;

  // Search.tsx/CommandPalette.tsx catch Escape on their autofocused
  // `<input>`'s onKeyDown — this panel has no single focusable field to
  // hang that off, so it listens on the window instead for the same
  // effect while it is mounted.
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  return (
    <div className="search-overlay" onClick={onClose}>
      <div className="search-panel settings-panel" onClick={(e) => e.stopPropagation()}>
        <h2 className="settings-panel-title">Settings</h2>
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
      </div>
    </div>
  );
}
