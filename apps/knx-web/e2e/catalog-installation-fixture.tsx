/** Renders the real catalogue with synthetic installation targets. */
import { createRoot } from "react-dom/client";
import CatalogBrowser from "../src/CatalogBrowser";
import type { InstallationNode } from "../src/bindings/InstallationNode";
import { setSetting } from "../src/settingsStore";
import { UI_LANGUAGE_STORAGE_KEY } from "../src/uiLanguage";
import "@fontsource/inter/400.css";
import "@fontsource/inter/500.css";
import "@fontsource/inter/600.css";
import "../src/styles.css";

const query = new URLSearchParams(location.search);
const language = query.get("lang") === "de" ? "de" : "en";
document.documentElement.lang = language;
document.documentElement.dataset.theme = query.get("theme") === "graphite" ? "graphite" : "porcelain";
document.documentElement.dataset.density = query.get("density") === "compact" ? "compact" : "comfortable";
setSetting(UI_LANGUAGE_STORAGE_KEY, language);

const installation = (id: number, name: string) => ({ id, name, topology: [], buildings: [], unassigned: [],
  group_addresses: [], group_ranges: [] }) satisfies InstallationNode;
const longName = "Imported installation with a very long descriptive name that keeps going well past any narrow screen width";
const installations = query.get("installations") === "2"
  ? [installation(0, "Main house"), installation(1, query.get("long") === "1" ? longName : "Garden")]
  : [installation(0, "Main house")];

createRoot(document.getElementById("root")!).render(
  <main className="workbench">
    <CatalogBrowser lineId={null} installations={installations} serverIncarnation="fixture-incarnation-1"
      onCreated={() => {}} onClose={() => {}} />
  </main>,
);
