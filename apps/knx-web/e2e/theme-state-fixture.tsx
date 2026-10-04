/** Real engineering components; fixture-local state, never production KNX or device operations. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { useRef, useState } from "react";
import Inspector from "../src/Inspector";
import GroupAddressTable from "../src/GroupAddressTable";
import Overlay from "../src/Overlay";
import ThemePackDiagnostic from "../src/ThemePackDiagnostic";
import type { Selection } from "../src/selection";
import { stateDevice, stateTree } from "./theme-state-data";

export default function RepresentativeThemeStates() {
  const [selection, setSelection] = useState<Selection | null>(null);
  const [dialog, setDialog] = useState(false);
  const opener = useRef<HTMLButtonElement | null>(null);
  const cancel = useRef<HTMLButtonElement | null>(null);
  return <section aria-label="Representative engineering states">
    <div style={{ display: "grid", gridTemplateColumns: "minmax(0, 1fr) minmax(280px, 360px)", gap: "16px" }}>
      <section className="workspace" aria-label="Representative address table">
        <GroupAddressTable installation={stateTree.installations[0]} selection={selection}
          multiSelection={null} rangeScope={null} onItemClick={(_event, _kind, _id, selected) => setSelection(selected)}
          onTreeUpdate={() => { throw new Error("Unexpected fixture domain mutation"); }} />
      </section>
      <aside aria-label="Representative device inspector">
        <Inspector propertiesOnly selection={{ kind: "device", id: stateDevice.id }} tree={stateTree}
          deviceDetail={stateDevice} onApplied={() => { throw new Error("Unexpected fixture domain mutation"); }}
          onDeleted={() => { throw new Error("Unexpected fixture domain mutation"); }} />
      </aside>
    </div>
    <section className="settings-diagnostic" role="status" aria-label="Representative diagnostic">
      <ThemePackDiagnostic diagnostic={{ kind: "invalidTokens", path: "tokens.--knx-bg" }} />
    </section>
    <button ref={opener} onClick={() => setDialog(true)}>Open sample dialog</button>
    {dialog && <Overlay label="Sample diagnostic dialog" className="settings-panel" initialFocusRef={cancel}
      restoreFocusRef={opener} onClose={() => setDialog(false)}>
      <h2>Sample diagnostic dialog</h2>
      <ThemePackDiagnostic diagnostic={{ kind: "invalidAccents", path: "accents.blue.--knx-accent" }} />
      <button ref={cancel} onClick={() => setDialog(false)}>Close sample dialog</button>
      <button disabled>Unavailable sample action</button>
    </Overlay>}
  </section>;
}
