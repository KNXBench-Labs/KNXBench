/** Local monitor-control browser fixture; API responses are supplied only by Playwright. */
import { createRoot } from "react-dom/client";
import BusMonitorPanel from "../src/BusMonitorPanel";
import { setSetting } from "../src/settingsStore";
import { UI_LANGUAGE_STORAGE_KEY } from "../src/uiLanguage";
import "../src/styles.css";

const language = new URLSearchParams(location.search).get("lang") === "de" ? "de" : "en";
document.documentElement.lang = language;
setSetting(UI_LANGUAGE_STORAGE_KEY, language);
createRoot(document.getElementById("root")!).render(
  <main className="workbench">
    <BusMonitorPanel projectOpen={false} />
  </main>,
);
