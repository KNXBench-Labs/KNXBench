/** #13: the Log's one export button opens this; the scope choice lives here, not in the toolbar. */
import { useState } from "react";
import Overlay from "./Overlay";
import { useTranslate } from "./i18n";
import type { LogExportScope } from "./sessionLogExport";

export default function LogExportDialog(props: {
  /** Server entries the current filters and search show, and all retained ones. */
  filteredCount: number;
  allCount: number;
  /** While the log is (re)loading or failed to load, nothing may be exported. */
  disabled: boolean;
  onExport: (scope: LogExportScope) => Promise<void>;
  onClose: () => void;
}) {
  const t = useTranslate();
  const [scope, setScope] = useState<LogExportScope>("filtered");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function exportNow() {
    setBusy(true); setError(null);
    try {
      await props.onExport(scope);
      props.onClose();
    } catch (failure) {
      // The dialog stays; the log is untouched and a retry is one click.
      setError(failure instanceof Error ? failure.message : String(failure));
    } finally {
      setBusy(false);
    }
  }

  return <Overlay className="log-export-panel" labelledBy="log-export-title" onClose={busy ? () => {} : props.onClose}>
    <h2 id="log-export-title">{t("logPanel.exportTitle")}</h2>
    <fieldset className="log-export-scope" disabled={busy}>
      <legend>{t("logPanel.exportScope")}</legend>
      <label><input type="radio" name="log-export-scope" value="filtered" checked={scope === "filtered"}
        onChange={() => setScope("filtered")} />{t("logPanel.exportFiltered", { count: props.filteredCount })}</label>
      <label><input type="radio" name="log-export-scope" value="all" checked={scope === "all"}
        onChange={() => setScope("all")} />{t("logPanel.exportAll", { count: props.allCount })}</label>
    </fieldset>
    <p className="log-panel-hint">{t("logPanel.exportNote")}</p>
    {error && <p className="field-error" role="alert">{error}</p>}
    <div className="log-export-actions">
      <button type="button" onClick={props.onClose} disabled={busy}>{t("logPanel.exportCancel")}</button>
      <button type="button" className="primary-action log-export-confirm" onClick={() => void exportNow()}
        disabled={busy || props.disabled}>{t("logPanel.exportConfirm")}</button>
    </div>
  </Overlay>;
}
