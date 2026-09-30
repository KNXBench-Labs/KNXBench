/** Local fixture: readiness and comparison HTTP calls are intercepted by Playwright. */
import { createRoot } from "react-dom/client";
import DeviceInspectionPanel from "../src/DeviceInspectionPanel";
import { setSetting } from "../src/settingsStore";
import { UI_LANGUAGE_STORAGE_KEY } from "../src/uiLanguage";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import "../src/styles.css";

const language = new URLSearchParams(location.search).get("lang") === "de" ? "de" : "en";
document.documentElement.lang = language;
setSetting(UI_LANGUAGE_STORAGE_KEY, language);
const project = { installations: [] } as unknown as ProjectTree;
createRoot(document.getElementById("root")!).render(
  <main className="workbench"><DeviceInspectionPanel project={project} /></main>,
);
