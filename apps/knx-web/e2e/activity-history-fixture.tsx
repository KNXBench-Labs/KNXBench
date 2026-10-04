/** Actual diagnostics-parent fixture with intercepted HTTP only, never a KNX backend. */
import { createRoot } from "react-dom/client";
import BusDiagnosticsPanel from "../src/BusDiagnosticsPanel";
import { setSetting } from "../src/settingsStore";
import { UI_LANGUAGE_STORAGE_KEY } from "../src/uiLanguage";
import "../src/styles.css";
const language = new URLSearchParams(location.search).get("lang") === "de" ? "de" : "en";
document.documentElement.lang = language;
setSetting(UI_LANGUAGE_STORAGE_KEY, language);
createRoot(document.getElementById("root")!).render(
  <main className="workbench"><BusDiagnosticsPanel project={null} onTreeUpdate={() => undefined} /></main>,
);
