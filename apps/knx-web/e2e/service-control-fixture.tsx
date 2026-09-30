/** Browser fixture uses only intercepted local mock API responses; never a KNX gateway. */
import { createRoot } from "react-dom/client";
import ServiceControlDebugSetting from "../src/ServiceControlDebugSetting";
import ServiceControlPanel from "../src/ServiceControlPanel";
import { setSetting } from "../src/settingsStore";
import { UI_LANGUAGE_STORAGE_KEY } from "../src/uiLanguage";
import "../src/styles.css";

const language = new URLSearchParams(location.search).get("lang") === "de" ? "de" : "en";
document.documentElement.lang = language;
setSetting(UI_LANGUAGE_STORAGE_KEY, language);
createRoot(document.getElementById("root")!).render(
  <main className="workbench">
    <ServiceControlDebugSetting />
    <ServiceControlPanel projectOpen />
  </main>,
);
