/** Explicit local access to the full source-bound historical import report. */
import { useEffect, useRef, useState } from "react";
import type { ProjectTree } from "./bindings/ProjectTree";
import { serializeImportEvidence, REPORT_PREVIEW_CHARACTERS, type ImportEvidence } from "./importEvidence";
import * as api from "./api";
import { useTranslate } from "./i18n";
import { downloadLocalJson } from "./localJsonDownload";
import "./ImportEvidencePanel.css";

export default function ImportEvidencePanel(props: { tree: ProjectTree }) {
  const t = useTranslate();
  const [evidence, setEvidence] = useState<ImportEvidence | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const epoch = useRef(0);
  const pending = useRef(false);
  const mounted = useRef(false);
  useEffect(() => {
    mounted.current = true;
    epoch.current += 1;
    setEvidence(null);
    setError(null);
    return () => { mounted.current = false; epoch.current += 1; };
  // Original evidence is immutable across ordinary edits/refreshes. Replacement
  // identity or availability, not the full tree object, invalidates the read.
  }, [props.tree.server_incarnation, props.tree.project_incarnation, props.tree.import_report_available]);

  async function load() {
    if (pending.current) return;
    pending.current = true;
    setLoading(true);
    setError(null);
    const requestedEpoch = epoch.current;
    try {
      const result = await api.getImportEvidence();
      if (epoch.current !== requestedEpoch) return;
      if (result.server_incarnation !== props.tree.server_incarnation ||
          result.project_incarnation !== props.tree.project_incarnation) {
        throw new Error(t("importEvidence.changed"));
      }
      setEvidence(result);
    } catch (failure) {
      if (epoch.current === requestedEpoch) setError(api.errorMessage(failure));
    } finally {
      pending.current = false;
      if (mounted.current) setLoading(false);
    }
  }
  if (props.tree.import_report_available !== true) return null;
  const serialized = evidence?.report_json ?? "";
  return <div className="import-report-panel">
    <button type="button" onClick={() => void load()} disabled={loading}>{t("importEvidence.show")}</button>
    {error && <p role="alert">{error}</p>}
    {evidence && !evidence.available && <p role="note">{t("importEvidence.absent")}</p>}
    {evidence?.available && <div>
      <h4>{t(evidence.historical ? "importEvidence.historical" : "importEvidence.current")}</h4>
      <p>{t("importEvidence.explanation")}</p>
      <p>{t("importEvidence.privacy")}</p>
      <button type="button" onClick={() => downloadLocalJson(serializeImportEvidence(evidence), "import-report.json")}>{t("importEvidence.download")}</button>
      <details><summary>{t("importEvidence.preview")}</summary>
        <p>{t("importEvidence.previewLimit", {count:REPORT_PREVIEW_CHARACTERS})}</p>
        <pre>{serialized.slice(0, REPORT_PREVIEW_CHARACTERS)}</pre>
      </details>
    </div>}
  </div>;
}
