import type { MotionLevel, PaletteSettings, TokenName } from "./palette";
import { TOKENS } from "./palette";

const TOKEN_LABELS: Record<TokenName, string> = {
  accent: "Accent",
  bg: "Background",
  surface: "Surface",
  text: "Text",
};

const MOTION_LABELS: Record<MotionLevel, string> = {
  off: "Off",
  subtle: "Subtle",
  standard: "Standard",
};

// Shown in the <input type="color"> swatch while a token has no override —
// the widget needs some concrete value, but this is not the color that's
// actually rendered (the base theme's own token value is, via CSS fallthrough).
const PICKER_PLACEHOLDER = "#808080";

export default function ThemePanel(props: {
  settings: PaletteSettings;
  onSetColor: (token: TokenName, value: string | undefined) => void;
  onSetMotion: (motion: MotionLevel) => void;
  onResetAll: () => void;
  onClose: () => void;
}) {
  const { settings, onSetColor, onSetMotion, onResetAll, onClose } = props;
  return (
    <div className="theme-panel-overlay" onClick={onClose}>
      <div className="theme-panel" onClick={(e) => e.stopPropagation()}>
        <h2>Customize theme</h2>
        <ul className="theme-panel-tokens">
          {TOKENS.map((token) => (
            <li key={token}>
              <label>
                {TOKEN_LABELS[token]}
                <input
                  type="color"
                  value={settings.colors[token] ?? PICKER_PLACEHOLDER}
                  onChange={(e) => onSetColor(token, e.target.value)}
                />
              </label>
              {settings.colors[token] && (
                <button type="button" onClick={() => onSetColor(token, undefined)}>
                  Reset
                </button>
              )}
            </li>
          ))}
        </ul>
        <label className="theme-panel-motion">
          Motion
          <select value={settings.motion} onChange={(e) => onSetMotion(e.target.value as MotionLevel)}>
            {(Object.keys(MOTION_LABELS) as MotionLevel[]).map((level) => (
              <option key={level} value={level}>
                {MOTION_LABELS[level]}
              </option>
            ))}
          </select>
        </label>
        <div className="theme-panel-actions">
          <button type="button" onClick={onResetAll}>
            Reset all
          </button>
          <button type="button" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
