/** Browser fixture for the server's read-only, fail-closed K6 recovery status. */
import { createRoot } from "react-dom/client";
import AddressProgrammingPanel from "../src/AddressProgrammingPanel";
import { setSetting } from "../src/settingsStore";
import { UI_LANGUAGE_STORAGE_KEY } from "../src/uiLanguage";
import "../src/styles.css";

const language = new URLSearchParams(location.search).get("lang") === "de" ? "de" : "en";
document.documentElement.lang = language;
setSetting(UI_LANGUAGE_STORAGE_KEY, language);

createRoot(document.getElementById("root")!).render(
  <main className="workbench"><AddressProgrammingPanel project={null} /></main>,
);
