/** Separate project-edit review over a retained, opaque device observation. */
import { useEffect, useRef, useState } from "react";
import ComparisonTable from "./ComparisonTable";
import type { BoundDeviceComparison } from "./deviceComparisons";
import type { ProjectTree } from "./bindings/ProjectTree";
import { applyComparisonAdoption, previewComparisonAdoption, type ComparisonAdoptionPreview } from "./comparisonAdoptionApi";
import { errorMessage } from "./api";
import { useTranslate } from "./i18n";

export default function ComparisonAdoptionPanel({ result, disabled, onBusy, onApplied }: {
  result: BoundDeviceComparison; disabled: boolean; onBusy: (busy: boolean) => void;
  onApplied?: (tree: ProjectTree) => void | Promise<void>;
}) {
  const t = useTranslate();
  const [selected, setSelected] = useState<number[]>([]);
  const [preview, setPreview] = useState<ComparisonAdoptionPreview | null>(null);
  const [confirmed, setConfirmed] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const live = useRef(true);
  useEffect(() => { live.current = true; return () => { live.current = false; }; }, []);
  const available = !disabled && Boolean(result.observationToken) && Boolean(onApplied);
  function pending(value: boolean) { setBusy(value); onBusy(value); }
  function toggle(index: number) {
    setSelected((previous) => previous.includes(index) ? previous.filter((i) => i !== index) : [...previous, index]);
    setPreview(null); setConfirmed(false); setError(null);
  }
  async function review() {
    if (!available || busy || !result.observationToken || selected.length === 0) return;
    pending(true); setError(null); setPreview(null); setConfirmed(false);
    try {
      const value = await previewComparisonAdoption(result.observationToken, selected);
      if (live.current) setPreview(value);
    } catch (reason) { if (live.current) setError(errorMessage(reason)); }
    finally { onBusy(false); if (live.current) setBusy(false); }
  }
  async function adopt() {
    if (!available || busy || !preview?.changed || !confirmed) return;
    pending(true); setError(null);
    try {
      const outcome = await applyComparisonAdoption(preview.previewToken);
      if (live.current) { setPreview(null); setSelected([]); setConfirmed(false); }
      // A confirmed server edit must reach the root even if this leaf detached.
      // Local publication failure cannot turn an acknowledged commit into rollback.
      try { await onApplied?.(outcome.tree); }
      catch { if (live.current) setError(t("compare.adoption.refreshFailed")); }
    } catch (reason) {
      if (live.current) { setError(`${t("compare.adoption.applyFailed")} ${errorMessage(reason)}`); setPreview(null); setConfirmed(false); }
    } finally { onBusy(false); if (live.current) setBusy(false); }
  }
  return <section className="comparison-adoption" aria-label={t("compare.adoption.title")}>
    <h3>{t("compare.adoption.title")}</h3>
    <p>{t("compare.adoption.scope")}</p>
    {!available && <p className="form-warning">{t("compare.adoption.unavailable")}</p>}
    <ComparisonTable rows={result.decoded} changes={result.changes}
      selection={available ? { indices: selected, disabled: busy, toggle } : undefined} />
    {available && <button type="button" onClick={() => void review()} disabled={busy || selected.length === 0}>{t("compare.adoption.preview")}</button>}
    {preview && <div className="comparison-adoption-review">
      <h4>{t("compare.adoption.review")}</h4>
      <p>{t(preview.changed ? "compare.adoption.reviewHint" : "compare.adoption.noChange", { count: preview.selected.length, seconds: preview.expiresInSeconds })}</p>
      <ul>{preview.selected.map((index) => { const row = preview.rows[index]; return row && <li key={index}>
        <strong>{row.kind === "association" ? `${t("compare.kind.association")} · ${row.label}` : row.label}</strong>: {row.project ?? t("compare.unknown")} → {row.device ?? t("compare.unknown")}
      </li>; })}</ul>
      {preview.linkChanges.length > 0 && <>
        <h5>{t("compare.adoption.links")}</h5>
        <ul className="comparison-adoption-links">{preview.linkChanges.map((change) => <li key={`${change.add ? "+" : "-"}${change.comObjectId}:${change.groupAddressId}`}
          data-change={change.add ? "add" : "remove"}>
          {t(change.add ? "compare.adoption.linkAdd" : "compare.adoption.linkRemove", { object: change.object, address: change.groupAddress })}
          {" "}({t(change.sending ? "compare.adoption.linkSending" : "compare.adoption.linkReceiving")})
        </li>)}</ul>
      </>}
      {preview.changed && <>
        <label><input type="checkbox" checked={confirmed} disabled={busy} onChange={(event) => setConfirmed(event.target.checked)} />{t("compare.adoption.confirm")}</label>
        <button type="button" className="primary-action" disabled={busy || !confirmed || !available} onClick={() => void adopt()}>{t("compare.adoption.apply")}</button>
      </>}
    </div>}
    {busy && <p role="status">{t("compare.adoption.busy")}</p>}
    {error && <p className="field-error" role="alert">{error}</p>}
  </section>;
}
