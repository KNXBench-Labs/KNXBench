/** Reviewed read-only acquisition, cooperative stop and local verified ZIP. */
import {useEffect,useRef,useState} from "react";
import {createPortal} from "react-dom";
import Overlay from "./Overlay";
import {useTranslate} from "./i18n";
import {collectDevices,projectContextKey} from "./treeUtils";
import {loadPreferredGateway} from "./gatewayPreference";
import {splitGatewayEndpoint,validateGatewayFields} from "./gatewayEndpoint";
import type {ProjectTree} from "./bindings/ProjectTree";
import * as api from "./diagnosticApi";
import {useFileMenuItemProps} from "./FileMenu";
import "./universalSupport.css";
import "./diagnosticExport.css";

export default function DiagnosticExportButton({project,deviceId=null}:{project:ProjectTree|null;deviceId?:number|null}){
 const t=useTranslate();const entry=useFileMenuItemProps();const [open,setOpen]=useState(false);
 return <><button type="button" {...entry} data-diagnostics-open disabled={!project} onClick={()=>setOpen(true)}>{t("diagnostics.title")}</button>{open&&createPortal(<DiagnosticExportDialog project={project!} initialDevice={deviceId} close={()=>setOpen(false)}/>,document.body)}</>;
}
/** The reviewed export dialog itself; the Diagnostics tab opens it for its device too. */
export function DiagnosticExportDialog({project,initialDevice,close}:{project:ProjectTree;initialDevice:number|null;close:()=>void}){
 const t=useTranslate();const devices=[...collectDevices(project).values()];
 const [device,setDevice]=useState<number|null>(initialDevice);const [gateway,setGateway]=useState(loadPreferredGateway);
 const [view,setView]=useState<api.DiagnosticView|null>(null);const [readConsent,setReadConsent]=useState(false);const [shareConsent,setShareConsent]=useState(false);
 const [busy,setBusy]=useState(false);const [error,setError]=useState(false);const [saved,setSaved]=useState(false);
 const epoch=useRef(0);const running=useRef<string|null>(null);const mounted=useRef(true);
 // Own response tokens synchronously: unmount may precede the queued view's React commit.
 function updateView(next:api.DiagnosticView|null){running.current=next?.phase==="running"?next.token:null;setView(next);}
 const contextKey=`${projectContextKey(project)}/${project.snapshot_revision??"unbound"}`;
 const [reviewContext,setReviewContext]=useState<string|null>(null);
 const stale=reviewContext!==null&&reviewContext!==contextKey;
 useEffect(()=>{mounted.current=true;return()=>{mounted.current=false;++epoch.current;const token=running.current;running.current=null;if(token)void api.cancelDiagnostics(token).catch(()=>{});};},[]);
 useEffect(()=>{
  if(view?.phase!=="running")return;
  let active=true;let timer:ReturnType<typeof setTimeout>;const token=view.token;
  const poll=async()=>{try{const next=await api.pollDiagnostics(token);if(active){updateView(next);if(next.phase==="complete")return;}}catch{if(active)setError(true);}if(active)timer=setTimeout(()=>void poll(),250);};
  timer=setTimeout(()=>void poll(),250);return()=>{active=false;clearTimeout(timer);};
 },[view?.token,view?.phase]);
 const acquiring=view?.phase==="running";
 function invalidate(){++epoch.current;updateView(null);setReadConsent(false);setShareConsent(false);setError(false);setSaved(false);setReviewContext(null);}
 async function perform(kind:"preview"|"start"|"cancel"|"save"){
  if(busy)return;const version=++epoch.current;setBusy(true);setError(false);
  try{
   if(kind==="preview"){
    if(device===null||project.server_incarnation===undefined||project.project_incarnation===undefined||project.snapshot_revision===undefined)throw new Error("unbound project");
    const checked=validateGatewayFields(splitGatewayEndpoint(gateway));if(!checked.ok)throw new Error("invalid gateway");
    const next=await api.previewDiagnostics(device,checked.endpoint,{serverIncarnation:project.server_incarnation,projectIncarnation:project.project_incarnation,revision:project.snapshot_revision});
    if(mounted.current&&epoch.current===version){updateView(next);setReviewContext(contextKey);setReadConsent(false);setShareConsent(false);setSaved(false);}
   }else if(kind==="start"){
    if(!view||!readConsent||stale)return;const next=await api.startDiagnostics(view.token);
    if(mounted.current&&epoch.current===version){updateView(next);setReadConsent(false);setShareConsent(false);}
    // Cleanup may run before Start returns, while the displayed view is still a preview.
    else if(next.phase==="running")void api.cancelDiagnostics(next.token).catch(()=>{});
   }else if(kind==="cancel"){
    if(!view)return;const next=await api.cancelDiagnostics(view.token);if(mounted.current&&epoch.current===version)updateView(next);
   }else{
    if(!view||!shareConsent||view.phase!=="complete")return;await api.saveDiagnostics(view.token);if(mounted.current&&epoch.current===version)setSaved(true);
   }
  }catch{if(mounted.current&&epoch.current===version)setError(true);}finally{if(mounted.current&&epoch.current===version)setBusy(false);}
 }
 const requestClose=()=>{if(acquiring){void perform("cancel");return;}close();};
 return <Overlay labelledBy="diagnostic-export-title" className="universal-support diagnostic-export" resizable={{width:840,height:740}} onClose={requestClose}>
  <div><header><h2 id="diagnostic-export-title">{t("diagnostics.title")}</h2><button type="button" disabled={acquiring||busy} onClick={close}>{t("support.close")}</button></header>
   <p>{t("diagnostics.intro")}</p><p className="form-warning">{t("diagnostics.privacy")}</p>
   <fieldset disabled={acquiring||busy}>
    <label>{t("support.device")}<select data-diagnostics-device value={device??""} onChange={e=>{invalidate();setDevice(e.target.value?Number(e.target.value):null);}}><option value="">{t("diagnostics.chooseDevice")}</option>{devices.map(d=><option key={d.id} value={d.id}>{d.address??"—"} · {d.name}</option>)}</select></label>
    <label>{t("support.gateway")}<input data-diagnostics-gateway value={gateway} onChange={e=>{invalidate();setGateway(e.target.value);}} placeholder="IPv4:port"/></label>
    <button type="button" data-diagnostics-preview disabled={device===null||!gateway} onClick={()=>void perform("preview")}>{t("diagnostics.preview")}</button>
   </fieldset>
   {error&&<p role="alert">{t("diagnostics.failed")}</p>}
   {view&&<>
    <section data-diagnostics-review><h3>{t("diagnostics.preview")}</h3><p><strong>{view.target}</strong> · {view.gateway}</p>
     <p>{t("diagnostics.readOnly")}</p><p>{t("diagnostics.limits")}</p><p>{t("diagnostics.memory")}</p>
     <p>{t("diagnostics.identity")}</p>
     <ul>{view.report.items.map(item=><li key={item.query}>{item.query} · {item.propertyId===null?"DeviceDescriptorRead (0)":`PropertyRead object ${item.objectIndex}, PID ${item.propertyId}, index ${item.startIndex}, elements ${item.requestedElements}`} · {item.expectedBytes} bytes</li>)}</ul>
     {view.report.simulated&&<strong>{t("diagnostics.simulated")}</strong>}
     {stale&&<p role="alert">{t("diagnostics.stale")}</p>}
    </section>
    {view.phase==="preview"&&<><label className="support-check"><input type="checkbox" data-diagnostics-read-consent checked={readConsent} disabled={busy||stale} onChange={e=>setReadConsent(e.target.checked)}/>{t("diagnostics.readConsent")}</label>
     <button type="button" data-diagnostics-start disabled={busy||!readConsent||stale} onClick={()=>void perform("start")}>{t("diagnostics.read")}</button></>}
    {acquiring&&<><p role="status">{t("diagnostics.progress",{count:view.report.items.filter(i=>i.completedAt!==null).length})}</p><button type="button" data-diagnostics-cancel disabled={busy||view.cancelRequested} onClick={()=>void perform("cancel")}>{t(view.cancelRequested?"diagnostics.stopping":"diagnostics.cancel")}</button></>}
    {view.phase==="complete"&&<section data-diagnostics-result>
     <h3>{t("diagnostics.result")}</h3>{view.report.stopReason&&<p role="status">{view.report.stopReason}</p>}
     <ul>{view.report.items.map(i=><li key={i.query}>{i.query}: {t(`diagnostics.status.${i.status}`)} · {i.receivedBytes}/{i.expectedBytes} bytes</li>)}</ul>
     <details><summary>{t("diagnostics.sharePreview")}</summary><pre aria-label="report.json">{JSON.stringify(view.report,null,2)}</pre></details>
     <label className="support-check"><input type="checkbox" data-diagnostics-share-consent checked={shareConsent} disabled={busy} onChange={e=>setShareConsent(e.target.checked)}/>{t("diagnostics.shareConsent")}</label>
     <button type="button" data-diagnostics-save disabled={busy||!shareConsent} onClick={()=>void perform("save")}>{t("diagnostics.save")}</button>
     {saved&&<p role="status">{t("support.saved")}</p>}<p>{t("diagnostics.handoff")}</p>
    </section>}
   </>}
  </div>
 </Overlay>;
}
