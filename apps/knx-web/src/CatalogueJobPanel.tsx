/** Shared, bounded view of retained catalogue progress, bound consent and complete paged reports. */
import { useEffect, useId, useState } from "react";
import { useCatalogueJobs } from "./CatalogueJobsContext";
import { useTranslate } from "./i18n";
import type { PublicRecord } from "./catalogueJobsTypes";

export default function CatalogueJobPanel() {
 const {controller,state} = useCatalogueJobs(); const t = useTranslate(); const id = useId();
 const s = state.status; const [consent,setConsent] = useState<string | null>(null);
 const preview = s?.preview; const artifact = s?.result?.artifact ?? preview?.artifact;
 const [offsets,setOffsets] = useState<string[]>([]);
 useEffect(() => { setOffsets([]); },[artifact?.artifactId]);
 const page = state.page?.artifactId === artifact?.artifactId ? state.page : null;
 return <section className="catalog-update-preview catalogue-job" aria-label={t("catalogJob.title")}>
  <p className="catalog-update-notice">{t("catalogJob.lifetime")}</p>
  {state.previousUncertain && <p className="catalog-update-warning" role="alert">{t("catalogJob.previousUnknown")}</p>}
  {s && <>
   <h3>{s.source.name}</h3>
   <p role="status" className={s.state === "running" && s.phase !== "ready" ? "task-status-text" : undefined}>{t(`catalogJob.state.${s.state}`)}{s.phase && <> · {t(`catalogJob.phase.${s.phase}`)}</>}</p>
   <p>{t("catalogJob.received",{received:s.source.receivedBytes,total:s.source.declaredBytes})}</p>
   {s.progress && <p role="status">{t(`catalogJob.stage.${s.progress.stage}`)} · {t("catalogJob.pass",{pass:s.progress.pass})}: {s.progress.completed}{s.progress.total !== null && <> / {s.progress.total}</>} {t(`catalogJob.unit.${s.progress.unit}`)}</p>}
   {s.phase === "ready" && <p>{t("catalogUpdate.nothingYet")}</p>}
   {preview && <>
    <h4>{t("catalogJob.preview")}</h4>
    <p>{t("catalogJob.summary",{packages:preview.packages,programs:preview.programs,products:preview.productsAdded,conflicts:preview.idConflicts})}</p>
    {preview.packageAlreadyInstalled && <p>{t("catalogUpdate.already")}</p>}
    <p className="catalog-update-hash">{t("catalogJob.sourceHash")} <code>{s.source.sha256}</code></p>
    <p className="catalog-update-warning">{t("catalogJob.growth")}</p>
   </>}
   {artifact && <>
    <p>{t("catalogJob.redaction",{fields:artifact.withheldFields,records:artifact.withheldRecords,occurrences:artifact.withheldOccurrences})}</p>
    <p>{t("catalogJob.completeProjection",{total:artifact.records})}</p>
    <button type="button" disabled={state.pageBusy || state.unavailable} onClick={() => {setOffsets([]); void controller.details(artifact);}}>{t("catalogJob.details")}</button>
    {state.pageBusy && <p role="status">{t("catalogJob.loadingDetails")}</p>}
    {page && <div className="catalogue-job-report">
     <p role="status">{t("catalogJob.page",{offset:page.offset,count:page.records.length,total:page.totalRecords})}</p>
     <ol className="catalog-update-programs">{page.records.map(record => <li key={record.ordinal}><ReportRecord record={record} /></li>)}</ol>
     <div className="catalog-update-actions">
      <button type="button" disabled={state.pageBusy || offsets.length === 0} onClick={() => {const prev=offsets.at(-1)!;setOffsets(offsets.slice(0,-1));void controller.details(artifact,prev);}}>{t("catalogJob.previousPage")}</button>
      <button type="button" disabled={state.pageBusy || page.nextOffset === null} onClick={() => {setOffsets([...offsets,page.offset]);void controller.details(artifact,page.nextOffset!);}}>{t("catalogJob.nextPage")}</button>
     </div>
     {page.nextOffset === null && <p>{t("catalogJob.lastPage")}</p>}
    </div>}
   </>}
   {s.phase === "ready" && preview && <>
    <label className="catalogue-job-check" htmlFor={id}><input id={id} type="checkbox" checked={consent === preview.consentId} onChange={event => setConsent(event.target.checked ? preview.consentId : null)} />{t("catalogJob.confirm")}</label>
    <button type="button" className="primary-action" disabled={state.pending || state.applyUncertain || state.uncertain || consent !== preview.consentId} onClick={() => {setConsent(null);void controller.apply(preview);}}>{t("catalogUpdate.apply")}</button>
   </>}
   {s.state === "running" && <button type="button" disabled={state.unavailable || s.gate === "cancelAccepted"} onClick={() => void controller.cancel()}>{t("catalogJob.requestCancel")}</button>}
   {state.cancelOutcome && <p role="status">{t(`catalogJob.cancel.${state.cancelOutcome}`)}</p>}
   {s.phase === "settling" && <p className="catalog-update-warning">{t("catalogJob.settling")}</p>}
   {s.gate === "commitClaimed" && s.state === "running" && <p className="catalog-update-warning">{t("catalogJob.tooLate")}</p>}
   <dl className="project-wizard-summary catalogue-job-facts">
    <dt>{t("catalogJob.commit")}</dt><dd>{t(`catalogJob.commit.${s.commit.outcome}`)} · <code>{s.commit.evidence}</code></dd>
    <dt>{t("catalogJob.cleanup")}</dt><dd>{t(`catalogJob.cleanup.${s.cleanup.disposition}`)}{s.cleanup.failureKind && <> · {t(`catalogJob.error.${s.cleanup.failureKind}`)}</>}</dd>
    <dt>{t("catalogJob.backup")}</dt><dd>{t(`catalogJob.backup.${s.backup.state}`)}{s.backup.name && <> · <code>{s.backup.name}</code></>}</dd>
   </dl>
   {s.commit.outcome === "unknown" && <p role="alert">{t("catalogJob.unknown")}</p>}
   {s.result?.nextJobInputLimitExceeded && <p role="alert">{t("catalogJob.growthExceeded")}</p>}
   {s.failure && <p role="alert" className="field-error">{t(`catalogJob.error.${s.failure.kind}`)}</p>}
   {s.state !== "running" && s.cleanup.disposition === "complete" && <button type="button" disabled={state.pending || state.unavailable} onClick={() => void controller.dispose()}>{t("catalogJob.dispose")}</button>}
  </>}
  {state.selecting && <p role="status">{t("catalogJob.preparing")}</p>}
  {!s && (state.selecting || state.uncertain) && <button type="button" disabled={state.cancelQueued} onClick={() => void controller.cancel()}>{t("catalogJob.requestCancel")}</button>}
  {state.cancelQueued && <p role="status">{t("catalogJob.cancelQueued")}</p>}
  {state.error && <p role="alert" className="field-error">{t(`catalogJob.error.${state.error}` as Parameters<typeof t>[0])}</p>}
  {state.uncertain && <p className="catalog-update-warning">{t("catalogJob.unknown")}</p>}
  {state.applyUncertain && <p role="alert">{t("catalogJob.applyUncertain")}</p>}
  {s && !state.unavailable && <button type="button" onClick={() => void controller.refresh()}>{t("catalogJob.checkSame")}</button>}
  {!s && state.uncertain && !state.unavailable && <button type="button" disabled={state.pending} onClick={() => void controller.recoverCreate()}>{t("catalogJob.recoverCreate")}</button>}
  {(state.unavailable || !s && state.uncertain) && <button type="button" onClick={() => controller.forgetUnavailable()}>{t("catalogJob.forget")}</button>}
 </section>;
}
function ReportRecord({record}: {record: PublicRecord}) {
 const t = useTranslate();
 const render = (value: unknown): React.ReactNode => {
  if (value === null) return <span>{t("catalogJob.notAvailable")}</span>;
  if (Array.isArray(value)) return <ul>{value.map((item,i) => <li key={i}>{render(item)}</li>)}</ul>;
  if (typeof value === "object") return <dl>{Object.entries(value as Record<string,unknown>).map(([key,val]) => <div key={key}><dt>{t(`catalogJob.field.${key}` as Parameters<typeof t>[0])}</dt><dd>{render(val)}</dd></div>)}</dl>;
  if (typeof value === "boolean") return t(value ? "catalogJob.yes" : "catalogJob.no");
  return <code>{String(value)}</code>;
 };
 return <article className="catalog-update-installed">
  <h4>{t(`catalogJob.record.${record.kind}`)}</h4>
  {record.kind === "member" && record.role === "Signature" && <p className="catalog-update-warning">{t("catalogJob.signatureNotVerified")}</p>}
  <dl>{Object.entries(record).filter(([key]) => key !== "kind").map(([key,value]) => <div key={key}><dt>{t(`catalogJob.field.${key}` as Parameters<typeof t>[0])}</dt><dd>{render(value)}</dd></div>)}</dl>
 </article>;
}
