/** Mounts synthetic data for offline keyboard and tooltip tests. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { useState } from "react";
import { createRoot } from "react-dom/client";
import Search from "../src/Search";
import CommandPalette from "../src/CommandPalette";
import CatalogBrowser from "../src/CatalogBrowser";
import Overlay from "../src/Overlay";
import HelpTip from "../src/HelpTip";
import type { CommandContext } from "../src/commandRegistry";
import { setSetting } from "../src/settingsStore";
import { UI_LANGUAGE_STORAGE_KEY } from "../src/uiLanguage";
import { initialTree } from "./site-fixture-data";
import "../src/styles.css";

const params = new URLSearchParams(location.search);
const language = params.get("lang") === "de" ? "de" : "en";
setSetting(UI_LANGUAGE_STORAGE_KEY, language);
document.documentElement.lang = language;
document.documentElement.style.setProperty("--app-ui-scale", params.get("scale") === "1.5" ? "1.5" : "1");
const tree = structuredClone(initialTree);
tree.installations[0].topology[0].lines[0].devices = Array.from({ length: 40 }, (_, index) => ({ id: index + 100, name: `Fixture device ${String(index).padStart(2, "0")}`, address: `1.1.${index + 1}`, description: null, com_object_count: 0 }));
const noop = () => {};
const ctx: CommandContext = { tree, newProject: noop, pickProject: noop, openNativeProject: noop, saveProject: noop, saveProjectAs: noop, undo: noop, redo: noop, openSearch: noop, openLog: noop, openBusMonitor: noop, openSettings: noop, openCompanion: noop, openHelp: noop };
function Fixture() {
  const [view, setView] = useState<string | null>(null);
  const [nested, setNested] = useState(false);
  return <>
    <main data-background="true">
      {["search", "palette", "catalog", "modal"].map((name) => <button key={name} onClick={() => setView(name)}>{name}</button>)}
      <button id="background-action">Background action</button>
      <div style={{ position: "fixed", right: 8, bottom: 8, width: 60, height: 40, overflow: "hidden", transform: "translateZ(0)" }}>
        <HelpTip labelKey="help.tip.comFlags.label" textKey="help.tip.comFlags.text" />
      </div>
    </main>
    {view === "search" && <Search tree={tree} onSelect={noop} onClose={() => setView(null)} />}
    {view === "palette" && <CommandPalette ctx={ctx} onClose={() => setView(null)} />}
    {view === "catalog" && <CatalogBrowser lineId={null} onCreated={noop} onClose={() => setView(null)} />}
    {view === "modal" && <Overlay label="Fixture modal" onClose={() => setView(null)}>
      <button onClick={() => setNested(true)}>Nested dialog</button>
      <HelpTip labelKey="help.tip.comFlags.label" textKey="help.tip.comFlags.text" />
    </Overlay>}
    {nested && <Overlay label="Nested fixture" onClose={() => setNested(false)}><button onClick={() => setNested(false)}>Close nested</button></Overlay>}
  </>;
}
createRoot(document.getElementById("root")!).render(<Fixture />);
