/** File → Exchange entry for the explicitly reduced offline monitoring bundle. */
import {useEffect,useRef,useState} from "react";
import {createPortal} from "react-dom";
import type {ProjectTree} from "./bindings/ProjectTree";
import {previewMonitorExport,downloadMonitorExport} from "./api";
import {exportCountKinds,monitorExportRequest,type MonitorExportPreview} from "./monitorExport";
import {downloadLocalBlob} from "./localJsonDownload";
import {useFileMenuItemProps} from "./FileMenu";
import {useTranslate} from "./i18n";
import {useUiLanguage} from "./uiLanguage";
import Overlay from "./Overlay";
import "./MonitoringExportButton.css";

export default function MonitoringExportButton({tree}: {tree: ProjectTree | null}) {
  const t=useTranslate();const entry=useFileMenuItemProps();const [open,setOpen]=useState(false);
  const button=useRef<HTMLButtonElement|null>(null);const wasOpen=useRef(false);
  useEffect(()=>{
    if(wasOpen.current&&!open) button.current?.closest(".file-menu")?.querySelector<HTMLElement>("summary")?.focus();
    wasOpen.current=open;
  },[open]);
  const available=tree?.installations.some(row=>monitorExportRequest(tree,row.id,"")!==null)??false;
  return <>
    <button {...entry} ref={button} disabled={!available} onClick={()=>setOpen(true)}>{t("monitorExport.button")}</button>
    {open&&<MonitoringExportDialog tree={tree} onClose={()=>setOpen(false)}/>}
  </>;
}

function MonitoringExportDialog({tree,onClose}: {tree: ProjectTree|null;onClose:()=>void}) {
  const t=useTranslate();const [uiLanguage]=useUiLanguage();const cancel=useRef<HTMLButtonElement|null>(null);
  const [installation,setInstallation]=useState<number|null>(()=>tree?.installations.length===1?tree.installations[0].id:null);
  const [language,setLanguage]=useState("");const [preview,setPreview]=useState<{value:MonitorExportPreview;context:string}|null>(null);
  const [busy,setBusy]=useState<"preview"|"download"|null>(null);const [error,setError]=useState<string|null>(null);
  const [delivered,setDelivered]=useState(false);const [refused,setRefused]=useState(false);
  const request=monitorExportRequest(tree,installation,language);
  const context=JSON.stringify([request,uiLanguage]);const latest=useRef(context);latest.current=context;
  const generation=useRef(0);const locked=useRef(false);const mounted=useRef(false);const previousContext=useRef(context);
  useEffect(()=>{mounted.current=true;return()=>{mounted.current=false;generation.current++;};},[]);
  useEffect(()=>{
    generation.current++;locked.current=false;setBusy(null);setError(null);setDelivered(false);
    if(previousContext.current!==context){setPreview(null);setRefused(true);}
    previousContext.current=context;
  },[context]);
  const changed=preview!==null&&preview.context!==context;
  const downloadable=!!request&&!!preview&&!changed&&!refused&&!busy&&!delivered;
  async function run(kind:"preview"|"download") {
    if(!request||locked.current||(kind==="download"&&!downloadable))return;
    const captured=request;const token=++generation.current;locked.current=true;setBusy(kind);setError(null);setDelivered(false);
    const current=()=>mounted.current&&token===generation.current&&latest.current===context;
    try {
      if(kind==="preview") {
        const value=await previewMonitorExport(captured);
        if(current()){setPreview({value,context});setRefused(false);}
      } else if(preview) {
        const blob=await downloadMonitorExport(captured,preview.value.bundleSha256);
        if(current()){downloadLocalBlob(blob,"monitoring-export.zip");setDelivered(true);}
      }
    } catch(e) {
      if(current()){setError(e instanceof Error?e.message:String(e));setRefused(true);}
    } finally {if(current()){locked.current=false;setBusy(null);cancel.current?.focus();}}
  }
  const report=preview?.value.report;
  // A closed File submenu hides its descendants; the modal belongs at body level.
  return createPortal(<Overlay labelledBy="monitoring-export-title" className="monitoring-export-dialog" initialFocusRef={cancel} onClose={onClose}>
    <h2 id="monitoring-export-title">{t("monitorExport.title")}</h2>
    <div className="monitoring-export-body">
      <p>{t("monitorExport.scope")}</p>
      <p className="monitoring-export-privacy">{t("monitorExport.privacy")}</p>
      <p>{t("monitorExport.receiver")}</p>
      <p>{t("monitorExport.unknown")}</p>
      <div className="monitoring-export-options">
        <label>{t("monitorExport.installation")}<select value={installation??""} onChange={e=>setInstallation(e.target.value===""?null:Number(e.target.value))}>
          <option value="">{t("monitorExport.choose")}</option>
          {tree?.installations.map(row=><option key={row.id} value={row.id}>{row.name||`#${row.id}`}</option>)}
        </select></label>
        <label>{t("monitorExport.language")}<select value={language} onChange={e=>setLanguage(e.target.value)}>
          <option value="">{t("monitorExport.defaultLanguage")}</option><option value="en">English (en)</option><option value="de">Deutsch (de)</option>
        </select></label>
      </div>
      {!request&&<p role="alert">{t("monitorExport.unavailable")}</p>}
      {(changed||refused)&&<p role="alert">{t("monitorExport.changed")}</p>}
      {error&&<p role="alert">{t("monitorExport.failed",{error})}</p>}
      {busy&&<p role="status">{t(busy==="preview"?"monitorExport.previewing":"monitorExport.downloading")}</p>}
      {report&&<section aria-label={t("monitorExport.report")} aria-disabled={changed||refused} className={changed||refused?"monitoring-export-outdated":undefined}>
        <h3>{t("monitorExport.report")}</h3><p><code>{report.profile}</code> · {report.producer} {report.producer_version}</p>
        <div className="monitoring-export-counts" tabIndex={0} role="region" aria-label={t("monitorExport.report")}><table><thead><tr><th>{t("monitorExport.entity")}</th><th>{t("monitorExport.selected")}</th><th>{t("monitorExport.exported")}</th><th>{t("monitorExport.omitted")}</th></tr></thead>
          <tbody>{exportCountKinds.map(kind=><tr key={kind}><th scope="row">{t(`monitorExport.count.${kind}`)}</th><td>{report.counts[kind].selected}</td><td>{report.counts[kind].exported}</td><td>{report.counts[kind].omitted}</td></tr>)}</tbody>
        </table></div>
        <h4>{t("monitorExport.diagnostics",{count:report.issues.length})}</h4>
        <p>{t("monitorExport.showing",{shown:Math.min(50,report.issues.length),total:report.issues.length})}</p>
        <ul className="monitoring-export-issues">{report.issues.slice(0,50).map((issue,i)=><li key={i}><code>{issue.code} · {issue.entity} · {issue.field}</code><br/>{issue.detail}</li>)}</ul>
        <h4>{t("monitorExport.excluded")}</h4>
        <p>{t("monitorExport.showing",{shown:Math.min(20,report.excluded_data.length),total:report.excluded_data.length})}</p>
        <ul>{report.excluded_data.slice(0,20).map((value,i)=><li key={i}>{value}</li>)}</ul>
        <p>{t("monitorExport.complete")}</p>
      </section>}
      {delivered&&<p role="status">{t("monitorExport.handoff")}</p>}
    </div>
    <footer><button type="button" ref={cancel} onClick={onClose}>{t("monitorExport.cancel")}</button>
      <button type="button" disabled={!request||!!busy} onClick={()=>void run("preview")}>{t("monitorExport.preview")}</button>
      <button type="button" className="primary-action" disabled={!downloadable} onClick={()=>void run("download")}>{t("monitorExport.download")}</button></footer>
  </Overlay>,document.body);
}
