/** Explicit source-scoped report selection and complete local downloads. */
import {useEffect,useRef,useState} from "react";
import type {ProjectTree} from "./bindings/ProjectTree";
import type {SourceReportCatalogue,SourceReportEvidence} from "./sourceReports";
import {serializeSourceReport} from "./sourceReports";
import {REPORT_PREVIEW_CHARACTERS} from "./importEvidence";
import * as api from "./api";
import {useTranslate} from "./i18n";
import {downloadLocalJson} from "./localJsonDownload";
import "./ImportEvidencePanel.css";

export default function SourceReportPanel(props:{tree:ProjectTree}) {
  const t=useTranslate();
  const [catalogue,setCatalogue]=useState<SourceReportCatalogue|null>(null);
  const [selected,setSelected]=useState("");
  const [evidence,setEvidence]=useState<SourceReportEvidence|null>(null);
  const [error,setError]=useState<string|null>(null);
  const [busy,setBusy]=useState(false);
  const epoch=useRef(0);const pending=useRef(false);const mounted=useRef(false);
  useEffect(()=>{
    mounted.current=true;epoch.current+=1;setCatalogue(null);setSelected("");setEvidence(null);setError(null);
    return()=>{mounted.current=false;epoch.current+=1;};
  },[props.tree.server_incarnation,props.tree.project_incarnation,props.tree.snapshot_revision]);
  function checkBinding(value:{server_incarnation:string;project_incarnation:number;snapshot_revision:number}) {
    if(value.server_incarnation!==props.tree.server_incarnation || value.project_incarnation!==props.tree.project_incarnation ||
       value.snapshot_revision!==props.tree.snapshot_revision) throw new Error(t("importEvidence.changed"));
  }
  async function request(kind:"catalogue"|"report") {
    if(pending.current)return;pending.current=true;setBusy(true);setError(null);
    const started=epoch.current;
    try {
      if(kind==="catalogue") {
        const result=await api.getSourceReportCatalogue();if(started!==epoch.current)return;
        checkBinding(result);setCatalogue(result);setSelected(result.records[0]?.id??"");setEvidence(null);
      } else {
        const result=await api.getSourceReport(selected);if(started!==epoch.current)return;
        checkBinding(result);if(result.id!==selected)throw new Error(t("importEvidence.changed"));setEvidence(result);
      }
    } catch(failure) {
      if(started===epoch.current)setError(api.errorMessage(failure));
    } finally {pending.current=false;if(mounted.current)setBusy(false);}
  }
  const record=catalogue?.records.find(row=>row.id===selected);
  return <div className="import-report-panel source-report-panel">
    <button type="button" disabled={busy} onClick={()=>void request("catalogue")}>{t("sourceReports.show")}</button>
    {error&&<p role="alert">{error}</p>}
    {catalogue&&<div>
      <h4>{t("sourceReports.title")}</h4>
      <p>{t("sourceReports.explanation")}</p>
      {catalogue.records.length===0?<p>{t("sourceReports.empty")}</p>:<>
        <label>{t("sourceReports.select")} <select value={selected} disabled={busy} onChange={event=>{
          if(pending.current)return;setSelected(event.target.value);setEvidence(null);setError(null);
        }}>
          {catalogue.records.map(row=><option key={row.id} value={row.id}>
            {row.source_name??t("sourceReports.unnamed")} · {row.id.slice(0,12)}
          </option>)}
        </select></label>
        <p>{record?.source_name}</p>
        <p className="source-report-identity">{record?.id}</p>
        {record?.status==="recorded"&&<p>{t("sourceReports.counts",{errors:record.error_rows,warnings:record.warning_rows,bytes:record.bytes})}</p>}
        {record&&<p role="note">{t(record.status==="recorded"?"sourceReports.recorded":"sourceReports.unavailable")}</p>}
        <button type="button" disabled={busy||record?.status!=="recorded"} onClick={()=>void request("report")}>{t("sourceReports.read")}</button>
      </>}
    </div>}
    {evidence&&<div>
      <p>{t("importEvidence.privacy")}</p>
      <button type="button" onClick={()=>downloadLocalJson(serializeSourceReport(evidence),"source-import-report.json")}>{t("sourceReports.download")}</button>
      <details><summary>{t("importEvidence.preview")}</summary>
        <p>{t("importEvidence.previewLimit",{count:REPORT_PREVIEW_CHARACTERS})}</p>
        <pre>{evidence.report_json.slice(0,REPORT_PREVIEW_CHARACTERS)}</pre>
      </details>
    </div>}
  </div>;
}
