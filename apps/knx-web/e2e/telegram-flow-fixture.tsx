/** Renders the real BusMonitorPanel with a resolved theme, as index.html's bootstrap does. */
import { createRoot } from "react-dom/client";
import BusMonitorPanel from "../src/BusMonitorPanel";
import FlowWindow from "../src/FlowWindow";
import { isFlowWindow } from "../src/flowWindow";
import { setSetting } from "../src/settingsStore";
import { UI_LANGUAGE_STORAGE_KEY } from "../src/uiLanguage";
import "../src/styles.css";

const params = new URLSearchParams(location.search);
const language = params.get("lang") === "de" ? "de" : "en";
document.documentElement.lang = language;
// The app always carries a resolved `data-theme`; without it no theme
// variable is defined and the drawing would not be what users see.
document.documentElement.dataset.theme = params.get("theme") ?? "porcelain";
// The app's bootstrap sets the Motion level before mount, too. An init
// script cannot: its attribute is lost when the document is parsed.
if (params.get("motion")) document.documentElement.setAttribute("data-motion-level", params.get("motion")!);
setSetting(UI_LANGUAGE_STORAGE_KEY, language);
createRoot(document.getElementById("root")!).render(
  isFlowWindow(location.search) ? <FlowWindow /> : <main className="workbench">
    <BusMonitorPanel projectOpen />
  </main>,
);
