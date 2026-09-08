import type { ThemeDef } from "./theme";

export default function ThemeSwitcher(props: {
  themes: readonly ThemeDef[];
  activeId: string;
  onSelect: (id: string) => void;
}) {
  const { themes, activeId, onSelect } = props;
  return (
    <label className="theme-switcher">
      <span className="sr-only">Theme</span>
      <select value={activeId} onChange={(e) => onSelect(e.target.value)} aria-label="Theme">
        {themes.map((t) => (
          <option key={t.id} value={t.id}>
            {t.name}
          </option>
        ))}
      </select>
    </label>
  );
}
