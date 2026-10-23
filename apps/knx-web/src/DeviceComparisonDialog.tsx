/** One gateway review for a frozen device list, progress, cancellation and decoded results. */
import { useState } from "react";
import Overlay from "./Overlay";
import ComparisonAdoptionPanel from "./ComparisonAdoptionPanel";
import OpenDownloadButton from "./OpenDownloadButton";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { DeviceComparisons } from "./useDeviceComparisons";
import type { ComparisonEntry } from "./deviceComparisons";
import { loadPreferredGateway } from "./gatewayPreference";
import { splitGatewayEndpoint, validateGatewayFields } from "./gatewayEndpoint";
import { useTranslate } from "./i18n";

export function ComparisonTag({ entry, onClick }: { entry: ComparisonEntry | null; onClick: () => void }) {
  const t = useTranslate();
  if (!entry) return null;
  return <button type="button" className={`comparison-tag comparison-tag-${entry.tag}`} onClick={onClick}
    title={entry.tag === "outdated" ? t("compare.outdatedHint") : entry.reason ?? t("compare.scope")}>
    {t(`compare.tag.${entry.tag}`)}
  </button>;
}

export default function DeviceComparisonDialog({ controller: c, onTreeUpdate, snapshotRevision }: { controller: DeviceComparisons;
  onTreeUpdate?: (tree: ProjectTree) => void | Promise<void>; snapshotRevision?: number | null }) {
  const t = useTranslate();
  const [gateway, setGateway] = useState(loadPreferredGateway);
  const [gatewayError, setGatewayError] = useState<string | null>(null);
  const [adopting, setAdopting] = useState(false);
  const [adoptedToken, setAdoptedToken] = useState<string | null>(null);
  const review = c.review;
  if (!review) return null;
  const selected = c.selected === null ? null : c.entry(c.selected);
  function confirm() {
    const value = validateGatewayFields(splitGatewayEndpoint(gateway));
    if (!value.ok) { setGatewayError(t(value.reason === "hostRequired" ? "busMonitor.connectNeedsGateway" : value.reason === "invalidPort" ? "busMonitor.invalidPort" : "busMonitor.invalidHost")); return; }
    setGatewayError(null); void c.confirm(value.endpoint);
  }
  return <Overlay className="device-comparison-dialog" labelledBy="comparison-title" onClose={() => { if (!adopting) c.close(); }}>
    <header><p className="eyebrow">{t("compare.readOnly")}</p><h2 id="comparison-title">{t("compare.title")}</h2>
      <p>{t("compare.scope")}</p></header>
    {!review.started ? <>
      <p>{t("compare.confirmHint", { count: review.targets.length })}</p>
      <ul className="comparison-targets">{review.targets.map((target) => <li key={target.id}><code>{target.address ?? "—"}</code> — {target.name}</li>)}</ul>
      <label className="comparison-gateway-label">{t("deviceDownload.gateway")}
        <input className="comparison-gateway" value={gateway} onChange={(event) => { setGateway(event.target.value); setGatewayError(null); }} placeholder="192.0.2.10:3671" /></label>
      {gatewayError && <p className="field-error" role="alert">{gatewayError}</p>}
      <div className="dialog-actions"><button type="button" className="primary-action" onClick={confirm}>{t("compare.confirm")}</button>
        <button type="button" onClick={c.close}>{t("deviceChecks.cancel")}</button></div>
    </> : <>
      <p role="status">{t("compare.progress", { completed: c.progress.completed, total: review.targets.length })}
        {c.progress.current && <> — {c.progress.current.address ?? "—"} · {c.progress.current.name}</>}</p>
      <progress max={review.targets.length || 1} value={c.progress.completed} aria-label={t("compare.progressLabel")} />
      {c.running && <><p>{t(c.stopping ? "compare.stopping" : "compare.running")}</p>
        <button type="button" onClick={c.cancel} disabled={c.stopping}>{t("deviceChecks.cancel")}</button></>}
      <ul className="comparison-results">{review.targets.map((target) => {
        const entry = c.entry(target.id);
        return <li key={target.id}><button type="button" onClick={() => c.setSelected(target.id)} disabled={!entry || adopting}
          aria-pressed={c.selected === target.id}>{target.address ?? "—"} — {target.name}</button>
          <ComparisonTag entry={entry} onClick={() => { if (!adopting) c.setSelected(target.id); }} /></li>;
      })}</ul>
      {selected?.reason && <p className="comparison-reason">{selected.reason}</p>}
      {selected?.tag === "outdated" && <p className="form-warning">{t("compare.outdatedHint")}</p>}
      {selected?.result?.observationToken && selected.result.observationToken === adoptedToken && <p role="status">{t("compare.adoption.applied")}</p>}
      {selected?.result && <ComparisonAdoptionPanel
        key={`${selected.result.observationToken ?? "none"}:${selected.tag}:${snapshotRevision ?? "none"}`}
        result={selected.result} disabled={c.running || selected.tag === "outdated"} onBusy={setAdopting}
        onApplied={onTreeUpdate ? (tree) => { setAdoptedToken(selected.result!.observationToken ?? null); return onTreeUpdate(tree); } : undefined} />}
      {selected?.result && !c.running && !adopting && c.selected !== null && <div className="comparison-download-handoff">
        <p>{t("compare.adoption.downloadHint")}</p>
        <OpenDownloadButton deviceId={c.selected} address={selected.result.address} />
      </div>}
      {!c.running && <button type="button" disabled={adopting} onClick={c.close}>{t("compare.close")}</button>}
    </>}
    {c.error && <p className="field-error" role="alert">{c.error}</p>}
  </Overlay>;
}
