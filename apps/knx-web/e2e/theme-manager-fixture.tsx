/** Real Settings/root presentation with synthetic intercepted settings only. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { StrictMode, useState } from "react";
import { createRoot } from "react-dom/client";
import SettingsPanel from "../src/SettingsPanel";
import { getThemeDefinitions, useThemeId, type ThemePreview } from "../src/theme";
import { useAppearance } from "../src/appearance";
import { MOTION_LEVELS, MOTION_STYLES, useMotion } from "../src/motion";
import { initSettings, startSettingsRefresh } from "../src/settingsStore";
import "@fontsource/inter/400.css";
import "@fontsource/space-grotesk/400.css";
import "@fontsource/jetbrains-mono/400.css";
import "../src/styles.css";

function Fixture() {
  const [preview, setPreview] = useState<ThemePreview>();
  const [themeId, selectTheme] = useThemeId(preview);
  const appearance = useAppearance();
  useMotion();
  const [open, setOpen] = useState(false);
  return <>
    <main><h1>Offline Appearance fixture</h1>
      <button onClick={() => setOpen(true)}>Open settings</button>
      <output data-testid="saved-selection">{themeId}</output>
    </main>
    {open && <SettingsPanel themes={getThemeDefinitions()} activeThemeId={themeId} onSelectTheme={selectTheme}
      onPreviewTheme={setPreview} previewTheme={preview} appearance={appearance}
      motionStyles={MOTION_STYLES} activeMotionStyle="apple" onSelectMotionStyle={() => {}}
      motionLevels={MOTION_LEVELS} activeMotionLevel="off" onSelectMotionLevel={() => {}}
      productLanguages={[]} activeProductLanguage={null} onSelectProductLanguage={() => {}}
      autosaveEnabled={true} onSelectAutosaveEnabled={() => {}}
      autosaveIntervalMinutes={5} onSelectAutosaveIntervalMinutes={() => {}}
      onClose={() => setOpen(false)} />}
  </>;
}

// The fixture server has no proxy. The browser test must intercept this sole
// endpoint before navigation. No production server is started by this fixture.
await initSettings();
startSettingsRefresh();
createRoot(document.getElementById("root")!).render(<StrictMode><Fixture /></StrictMode>);
