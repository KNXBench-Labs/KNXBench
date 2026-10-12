/** Missing historical import evidence is not a fresh compatibility assessment. */
import { useTranslate } from "./i18n";
import "./ImportDiagnosticsNotice.css";

export default function ImportDiagnosticsNotice(props: {
  available?: boolean;
  historical?: boolean;
  compact?: boolean;
  onOpen?: () => void;
}) {
  const t = useTranslate();
  const historical = props.available === true && props.historical === true;
  if (props.available !== false && !historical) return null;
  const label = t(historical ? "importEvidence.historical" : "diagnostics.importReportUnavailable");
  const detail = t(historical ? "importEvidence.explanation" : "diagnostics.nativeImportReportUnavailableExplanation");
  if (props.onOpen) {
    return <button type="button" className="import-evidence-notice" title={detail} onClick={props.onOpen}>{label}</button>;
  }
  return <p className="import-evidence-notice" role="note" title={props.compact ? detail : undefined}>
    <strong>{label}</strong>{!props.compact && <> {detail}</>}
  </p>;
}
