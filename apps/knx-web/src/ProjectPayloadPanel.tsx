/** Preserved project payloads are explicit inert local downloads, never execution. */
import {useEffect,useRef,useState} from "react";
import type {ProjectTree} from "./bindings/ProjectTree";
import type {ProjectPayloadCatalogue} from "./projectPayloads";
import * as api from "./api";
import {downloadLocalBlob} from "./localJsonDownload";
import {useTranslate} from "./i18n";
import "./ImportEvidencePanel.css";
import type {MessageKey} from "./messages/en";
const ROLE_LABELS:Record<string,MessageKey>={
 "user-file-candidate":"payloads.role.user-file-candidate",
 "add-in-state-candidate":"payloads.role.add-in-state-candidate",
 "ets-app-named-candidate":"payloads.role.ets-app-named-candidate",
 "manufacturer-baggage":"payloads.role.manufacturer-baggage",
};

export default function ProjectPayloadPanel(props:{tree:ProjectTree}) {
 const t=useTranslate();const [catalogue,setCatalogue]=useState<ProjectPayloadCatalogue|null>(null);
 const [id,setId]=useState<number|null>(null);const [error,setError]=useState<string|null>(null);const [busy,setBusy]=useState(false);
 const epoch=useRef(0);const pending=useRef(false);const mounted=useRef(false);
 useEffect(()=>{mounted.current=true;epoch.current+=1;setCatalogue(null);setId(null);setError(null);
  return()=>{mounted.current=false;epoch.current+=1;};
 },[props.tree.server_incarnation,props.tree.project_incarnation,props.tree.snapshot_revision]);
 async function action(kind:"list"|"download") {
  if(pending.current)return;pending.current=true;setBusy(true);setError(null);const started=epoch.current;
  try {
   if(kind==="list") {
    const value=await api.getProjectPayloads();if(started!==epoch.current)return;
    if(value.server_incarnation!==props.tree.server_incarnation||value.project_incarnation!==props.tree.project_incarnation||value.snapshot_revision!==props.tree.snapshot_revision)throw new Error(t("importEvidence.changed"));
    setCatalogue(value);setId(value.records[0]?.id??null);
   } else if(catalogue&&id!==null) {
    const blob=await api.getProjectPayload(catalogue,id);if(started!==epoch.current)return;
    downloadLocalBlob(blob,"retained-payload.bin");
   }
  } catch(failure){if(started===epoch.current)setError(api.errorMessage(failure));}
  finally{pending.current=false;if(mounted.current)setBusy(false);}
 }
 const selected=catalogue?.records.find(row=>row.id===id);
 return <div className="import-report-panel project-payload-panel">
  <button type="button" disabled={busy} onClick={()=>void action("list")}>{t("payloads.show")}</button>
  {error&&<p role="alert">{error}</p>}
  {catalogue&&<div>
   <h4>{t("payloads.title")}</h4><p>{t("payloads.explanation")}</p><p>{t("payloads.privacy")}</p>
   {catalogue.records.length===0?<p>{t("payloads.empty")}</p>:<>
    <label>{t("payloads.select")} <select disabled={busy} value={id??""} onChange={event=>{
      if(pending.current)return;const value=Number(event.target.value);if(catalogue.records.some(row=>row.id===value))setId(value);
    }}>{catalogue.records.map(row=><option key={row.id} value={row.id}>{row.source_path}</option>)}</select></label>
    {selected&&<>
     <p className="payload-path">{selected.source_path}</p>
     <dl><dt>{t("payloads.role")}</dt><dd>{t(ROLE_LABELS[selected.role])}</dd>
      <dt>{t("payloads.kind")}</dt><dd>{selected.kind}</dd><dt>{t("payloads.content")}</dt><dd>{selected.content_class}</dd>
      <dt>{t("payloads.bytes")}</dt><dd>{selected.bytes}</dd><dt>{t("payloads.hash")}</dt><dd className="payload-path">{selected.sha256}</dd></dl>
     {!selected.downloadable&&<p role="note">{t("payloads.large",{limit:catalogue.max_download_bytes})}</p>}
     <button type="button" disabled={busy||!selected.downloadable} onClick={()=>void action("download")}>{t("payloads.download")}</button>
    </>}
   </>}
  </div>}
 </div>;
}
