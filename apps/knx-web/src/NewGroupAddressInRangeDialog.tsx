/** Creates a group address inside one range, typed in or as the next free one (issue #20). */
import { useRef, useState, type KeyboardEvent } from "react";
import * as api from "./api";
import type { GroupRangeNode } from "./bindings/GroupRangeNode";
import type { ProjectTree } from "./bindings/ProjectTree";
import Overlay from "./Overlay";
import { canonicalGroupAddress, useGroupAddressFormat } from "./gaNotation";
import { useTranslate } from "./i18n";
import { nextFreeGroupAddress } from "./nextFreeGroupAddress";

type Mode = "next" | "manual";

export default function NewGroupAddressInRangeDialog(props: {
  range: GroupRangeNode;
  /** The range's display name (an unnamed range gets its fallback label). */
  rangeLabel: string;
  /** Every address of the installation, in the project's style. */
  usedAddresses: readonly string[];
  /** `ProjectTree.group_address_style`. */
  style: string;
  onCreated: (tree: ProjectTree) => void;
  onClose: () => void;
}) {
  const { range, style } = props;
  const t = useTranslate();
  const formatGa = useGroupAddressFormat();
  const suggestion = nextFreeGroupAddress(range, props.usedAddresses, style);
  const [mode, setMode] = useState<Mode>(suggestion === null ? "manual" : "next");
  const [manual, setManual] = useState("");
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const nameRef = useRef<HTMLInputElement | null>(null);
  const address = mode === "next" ? suggestion : manual.trim() === "" ? null : manual.trim();
  const canCreate = !busy && address !== null && name.trim() !== "";

  async function create() {
    if (!canCreate || address === null) return;
    setBusy(true);
    setError(null);
    try {
      // The API takes the canonical `/` spelling whatever is on screen.
      const tree = await api.createGroupAddress(name, canonicalGroupAddress(address), range.id);
      props.onCreated(tree);
      props.onClose();
    } catch (e) {
      // The dialog stays with everything typed; nothing was created.
      setError(api.errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  const submitOnEnter = (e: KeyboardEvent) => {
    if (e.key === "Enter") {
      e.preventDefault();
      void create();
    }
  };

  return (
    <Overlay className="ga-new-in-range" labelledBy="ga-new-in-range-title" initialFocusRef={nameRef}
      onClose={busy ? () => {} : props.onClose}>
      <h2 id="ga-new-in-range-title">{t("gaWorkspace.newInRange.title")}</h2>
      <p className="ga-new-in-range-range">
        {props.rangeLabel} <span className="mono">{formatGa(range.start)}–{formatGa(range.end)}</span>
      </p>
      <fieldset className="ga-new-in-range-address" disabled={busy}>
        <legend>{t("gaWorkspace.newInRange.address")}</legend>
        <label>
          <input type="radio" name="ga-new-in-range-mode" value="next" checked={mode === "next"}
            disabled={suggestion === null} onChange={() => setMode("next")} />
          {t("gaWorkspace.newInRange.next")}{" "}
          {suggestion === null
            ? <em>{t("gaWorkspace.newInRange.full")}</em>
            : <strong className="mono" data-ga-next-free="">{formatGa(suggestion)}</strong>}
        </label>
        <label>
          <input type="radio" name="ga-new-in-range-mode" value="manual" checked={mode === "manual"}
            onChange={() => setMode("manual")} />
          {t("gaWorkspace.newInRange.manual")}
        </label>
        <input className="ga-new-in-range-manual mono" value={manual} aria-label={t("workbench.address")}
          placeholder={formatGa(range.start)}
          onFocus={() => setMode("manual")} onChange={(e) => { setMode("manual"); setManual(e.target.value); }}
          onKeyDown={submitOnEnter} />
      </fieldset>
      <label className="ga-new-in-range-name">
        {t("workbench.name")}
        <input ref={nameRef} value={name} disabled={busy} placeholder={t("explorer.newGroupAddressPlaceholder")}
          onChange={(e) => setName(e.target.value)} onKeyDown={submitOnEnter} />
      </label>
      {error && <p className="field-error" role="alert">{error}</p>}
      <div className="ga-new-in-range-actions">
        <button type="button" onClick={props.onClose} disabled={busy}>{t("gaWorkspace.newInRange.cancel")}</button>
        <button type="button" className="primary-action ga-new-in-range-create" disabled={!canCreate}
          onClick={() => void create()}>{t("gaWorkspace.newInRange.create")}</button>
      </div>
    </Overlay>
  );
}
