/** Inspector section: which parts the project believes the device holds, and marking by hand. */
import { useState } from "react";
import * as api from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import { useTranslate } from "./i18n";
import { PART_LABELS, PROGRAMMING_PARTS } from "./programmingState";
import { collectDevices } from "./treeUtils";

export default function DeviceProgrammingField({ deviceId, tree, onApplied }: {
  deviceId: number; tree: ProjectTree; onApplied: (tree: ProjectTree) => void;
}) {
  const t = useTranslate();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const state = collectDevices(tree).get(deviceId)?.pending_programming;
  async function mark(programmed: boolean) {
    setBusy(true); setError(null);
    try {
      onApplied(await api.markDeviceProgramming(deviceId, programmed, tree));
    } catch (reason) {
      setError(api.errorMessage(reason));
    } finally {
      setBusy(false);
    }
  }
  return <section className="device-programming" aria-labelledby={`device-programming-${deviceId}`}>
    <h3 id={`device-programming-${deviceId}`}>{t("programming.title")}</h3>
    {state ? <>
      <dl className="device-programming-parts">
        {PROGRAMMING_PARTS.map((part) => <div key={part} data-part={part} data-pending={state[part] ? "true" : "false"}>
          <dt>{t(PART_LABELS[part].long)}</dt>
          <dd>{t(state[part] ? "programming.pending" : "programming.programmed")}</dd>
        </div>)}
      </dl>
      <p className="inspector-description">{t("programming.explanation")}</p>
      <div className="device-programming-actions">
        <button type="button" disabled={busy || PROGRAMMING_PARTS.every((part) => !state[part])}
          onClick={() => void mark(true)}>{t("programming.markProgrammed")}</button>
        <button type="button" disabled={busy || PROGRAMMING_PARTS.every((part) => state[part])}
          onClick={() => void mark(false)}>{t("programming.markNotProgrammed")}</button>
      </div>
    </> : <p className="inspector-description">{t("programming.unknown")}</p>}
    {error && <p className="field-error" role="alert">{error}</p>}
  </section>;
}
