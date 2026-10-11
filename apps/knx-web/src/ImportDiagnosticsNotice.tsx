/** Missing historical import evidence is not a fresh compatibility assessment. */
import { useTranslate } from "./i18n";
import "./ImportDiagnosticsNotice.css";

export default function ImportDiagnosticsNotice(props: {
  available?: boolean;
  compact?: boolean;
  onOpen?: () => void;
}) {
  const t = useTranslate();
  if (props.available !== false) return null;
  const label = t("diagnostics.importReportUnavailable");
  const detail = t("diagnostics.nativeImportReportUnavailableExplanation");
  if (props.onOpen) {
    return <button type="button" className="import-evidence-notice" title={detail} onClick={props.onOpen}>{label}</button>;
  }
  return <p className="import-evidence-notice" role="note" title={props.compact ? detail : undefined}>
    <strong>{label}</strong>{!props.compact && <> {detail}</>}
  </p>;
}
