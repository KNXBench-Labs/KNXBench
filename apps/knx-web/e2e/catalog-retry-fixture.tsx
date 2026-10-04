/** Renders the real CatalogBrowser for one server incarnation; Playwright mocks every API call. */
import { createRoot } from "react-dom/client";
import CatalogBrowser from "../src/CatalogBrowser";
import { setSetting } from "../src/settingsStore";
import { UI_LANGUAGE_STORAGE_KEY } from "../src/uiLanguage";
import "../src/styles.css";

const language = new URLSearchParams(location.search).get("lang") === "de" ? "de" : "en";
document.documentElement.lang = language;
setSetting(UI_LANGUAGE_STORAGE_KEY, language);

createRoot(document.getElementById("root")!).render(
  <main className="workbench">
    <CatalogBrowser lineId={7} serverIncarnation="fixture-incarnation-1" onCreated={() => {}} onClose={() => {}} />
  </main>,
);
