/** Browser fixture exercises existing commands with intercepted local HTTP only. */
import { useState } from "react";
import { createRoot } from "react-dom/client";
import type { ProjectTree } from "../src/bindings/ProjectTree";
import type { Selection } from "../src/selection";
import StructureWorkspace from "../src/StructureWorkspace";
import { setSetting } from "../src/settingsStore";
import { UI_LANGUAGE_STORAGE_KEY } from "../src/uiLanguage";
import { initialTree } from "./site-fixture-data";
import "../src/styles.css";

const language = new URLSearchParams(location.search).get("lang") === "de" ? "de" : "en";
document.documentElement.lang = language;
setSetting(UI_LANGUAGE_STORAGE_KEY, language);

function Fixture() {
  const [tree, setTree] = useState<ProjectTree>(initialTree);
  const [selection, setSelection] = useState<Selection | null>(null);
  return <main className="workbench">
    <StructureWorkspace tree={tree} view="buildings" selection={selection}
      multiSelection={null} onItemClick={() => {}} onTreeUpdate={setTree} onDeleted={setTree}
      onSelect={setSelection} onCatalog={() => {}} />
  </main>;
}

createRoot(document.getElementById("root")!).render(<Fixture />);
