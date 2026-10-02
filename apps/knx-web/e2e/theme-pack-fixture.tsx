/** Mounts only the real presentation hooks on a synthetic offline settings cache. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { getThemeDefinitions, readThemeSelection, useThemeId } from "../src/theme";
import { useAppearance } from "../src/appearance";
import { useMotion } from "../src/motion";
import { getSetting, settingsStorage } from "../src/settingsStore";
import "@fontsource/inter/400.css";
import "@fontsource/space-grotesk/400.css";
import "@fontsource/jetbrains-mono/400.css";
import "../src/styles.css";

function Fixture() {
  const [id, select] = useThemeId(); useAppearance(); useMotion();
  const selection = readThemeSelection(settingsStorage, getSetting("uiThemePacks"));
  return <main>
    <h1>Offline theme runtime</h1>
    <p data-testid="selected">{id}</p>
    <p data-testid="diagnostics">{selection.diagnostics.map((entry) => entry.diagnostic.kind).join(",")}</p>
    {getThemeDefinitions().map((theme) => <button key={theme.id} data-choice={theme.id} onClick={() => select(theme.id)}>{theme.name}</button>)}
    <table><tbody><tr><td>Fixture table</td><td><input aria-label="Fixture input" defaultValue="Text remains text" /></td></tr></tbody></table>
  </main>;
}
createRoot(document.getElementById("root")!).render(<StrictMode><Fixture /></StrictMode>);
