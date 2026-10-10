/** Renders the real LogPanel with intercepted API only; no backend/device. */
import { createRoot } from "react-dom/client";
import LogPanel from "../src/LogPanel";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import { setSetting } from "../src/settingsStore";
import { UI_LANGUAGE_STORAGE_KEY } from "../src/uiLanguage";
import "../src/styles.css";
const language = new URLSearchParams(location.search).get("lang") === "de" ? "de" : "en";
document.documentElement.dataset.theme = "porcelain";
document.documentElement.lang = language;
setSetting(UI_LANGUAGE_STORAGE_KEY, language);
const tree: ProjectTree = { schema_version: 12, errors: 0, warnings: 0, can_undo: false, can_redo: false, is_modified: false, group_address_style: "ThreeLevel", installations: [] };
createRoot(document.getElementById("root")!).render(<LogPanel tree={tree} refreshKey={0} />);
