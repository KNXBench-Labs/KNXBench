/** Reviewed read-only acquisition and cooperative stop; the report goes through the wizard. */
import {useEffect,useRef,useState} from "react";
import {createPortal} from "react-dom";
import Overlay from "./Overlay";
import {useTranslate} from "./i18n";
import {collectDevices,projectContextKey} from "./treeUtils";
import {loadPreferredGateway} from "./gatewayPreference";
import {splitGatewayEndpoint,validateGatewayFields} from "./gatewayEndpoint";
import type {ProjectTree} from "./bindings/ProjectTree";
import * as api from "./diagnosticApi";
import {diagnosticProgress,hexWord,memoryPlan,refusalMessage,stopReasonMessage} from "./diagnosticMemory";
import type {MessageKey} from "./messages/en";
import {useFileMenuItemProps} from "./FileMenu";
import {useReportProblem} from "./ReportProblemEntry";
import "./universalSupport.css";
import "./diagnosticExport.css";

export default function DiagnosticExportButton({project,deviceId=null}:{project:ProjectTree|null;deviceId?:number|null}){
 const t=useTranslate();const entry=useFileMenuItemProps();const [open,setOpen]=useState(false);
 return <><button type="button" {...entry} data-diagnostics-open disabled={!project} onClick={()=>setOpen(true)}>{t("diagnostics.title")}</button>{open&&createPortal(<DiagnosticExportDialog project={project!} initialDevice={deviceId} close={()=>setOpen(false)}/>,document.body)}</>;
}
/** The reviewed read dialog itself; the Diagnostics tab opens it for its device too.
 * The profile choice sits before Preview; changing it, the device or the gateway drops the review.
 * A finished read is reported through the one Report problem wizard (DIAG-03), which finds
 * this exact retained result by its token, even when no recording is available. */
export function DiagnosticExportDialog({project,initialDevice,close}:{project:ProjectTree;initialDevice:number|null;close:()=>void}){
 const t=useTranslate();const devices=[...collectDevices(project).values()];const report=useReportProblem();
 const [device,setDevice]=useState<number|null>(initialDevice);const [gateway,setGateway]=useState(loadPreferredGateway);
 const [view,setView]=useState<api.DiagnosticView|null>(null);const [readConsent,setReadConsent]=useState(false);
 const [profile,setProfile]=useState<api.DiagnosticProfile>(api.IDENTITY_PROFILE);
 const [busy,setBusy]=useState(false);const [error,setError]=useState<false|{key:MessageKey|null;detail:string|null}>(false);
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
  const poll=async()=>{try{const next=await api.pollDiagnostics(token);if(active){updateView(next);if(next.phase==="complete")return;}}catch{if(active)setError({key:null,detail:null});}if(active)timer=setTimeout(()=>void poll(),250);};
  timer=setTimeout(()=>void poll(),250);return()=>{active=false;clearTimeout(timer);};
 },[view?.token,view?.phase]);
 const acquiring=view?.phase==="running";const memory=view?.memory??null;const progress=view?diagnosticProgress(view):null;
 function invalidate(){++epoch.current;updateView(null);setReadConsent(false);setError(false);setReviewContext(null);}
 async function perform(kind:"preview"|"start"|"cancel"){
  if(busy)return;const version=++epoch.current;setBusy(true);setError(false);
  try{
   if(kind==="preview"){
    if(device===null||project.server_incarnation===undefined||project.project_incarnation===undefined||project.snapshot_revision===undefined)throw new Error("unbound project");
    const checked=validateGatewayFields(splitGatewayEndpoint(gateway));if(!checked.ok)throw new Error("invalid gateway");
    const next=await api.previewDiagnostics(device,checked.endpoint,profile,{serverIncarnation:project.server_incarnation,projectIncarnation:project.project_incarnation,revision:project.snapshot_revision});
    if(mounted.current&&epoch.current===version){updateView(next);setReviewContext(contextKey);setReadConsent(false);}
   }else if(kind==="start"){
    if(!view||!readConsent||stale)return;const next=await api.startDiagnostics(view.token);
    if(mounted.current&&epoch.current===version){updateView(next);setReadConsent(false);}
    // Cleanup may run before Start returns, while the displayed view is still a preview.
    else if(next.phase==="running")void api.cancelDiagnostics(next.token).catch(()=>{});
   }else{
    if(!view)return;const next=await api.cancelDiagnostics(view.token);if(mounted.current&&epoch.current===version)updateView(next);
   }
  }catch(e){
   // Only a preview refusal names why: its code is the server's fixed token (product data, identity, target).
   // The translated text leads; the server's raw detail follows as secondary evidence.
   if(mounted.current&&epoch.current===version)setError(kind==="preview"&&e instanceof api.DiagnosticRefusal?{key:refusalMessage(e.code),detail:e.detail}:{key:null,detail:null});
  }finally{if(mounted.current&&epoch.current===version)setBusy(false);}
 }
 const requestClose=()=>{if(acquiring){void perform("cancel");return;}close();};
 return <Overlay labelledBy="diagnostic-export-title" className="universal-support diagnostic-export" resizable={{width:840,height:740}} onClose={requestClose}>
  <div><header><h2 id="diagnostic-export-title">{t("diagnostics.title")}</h2><button type="button" disabled={acquiring||busy} onClick={close}>{t("support.close")}</button></header>
   <p>{t("diagnostics.intro")}</p><p className="form-warning">{t("diagnostics.privacy")}</p>
   <fieldset disabled={acquiring||busy}>
    <label>{t("support.device")}<select data-diagnostics-device value={device??""} onChange={e=>{invalidate();setDevice(e.target.value?Number(e.target.value):null);}}><option value="">{t("diagnostics.chooseDevice")}</option>{devices.map(d=><option key={d.id} value={d.id}>{d.address??"—"} · {d.name}</option>)}</select></label>
    <label>{t("support.gateway")}<input data-diagnostics-gateway value={gateway} onChange={e=>{invalidate();setGateway(e.target.value);}} placeholder="IPv4:port"/></label>
    <label>{t("diagnostics.profile")}<select data-diagnostics-profile value={profile} onChange={e=>{invalidate();setProfile(e.target.value===api.MEMORY_PROFILE?api.MEMORY_PROFILE:api.IDENTITY_PROFILE);}}>
     <option value={api.IDENTITY_PROFILE}>{t("diagnostics.profile.identity")}</option><option value={api.MEMORY_PROFILE}>{t("diagnostics.profile.memory")}</option></select></label>
    <p data-diagnostics-profile-hint className={profile===api.MEMORY_PROFILE?"form-warning":undefined}>{t(profile===api.MEMORY_PROFILE?"diagnostics.profileHint.memory":"diagnostics.profileHint.identity")}</p>
    <button type="button" data-diagnostics-preview disabled={device===null||!gateway} onClick={()=>void perform("preview")}>{t("diagnostics.preview")}</button>
   </fieldset>
   {error&&<div role="alert" data-diagnostics-error><p>{t(error.key??"diagnostics.failed")}</p>{error.detail&&<p data-diagnostics-error-detail>{t("diagnostics.serverDetail")} <code>{error.detail}</code></p>}</div>}
   {view&&<>
    <section data-diagnostics-review><h3>{t("diagnostics.preview")}</h3><p><strong>{view.target}</strong> · {view.gateway}</p>
     <p>{t("diagnostics.readOnly")}</p>{memory?<MemoryPreview view={view} memory={memory}/>:<><p>{t("diagnostics.limits")}</p><p>{t("diagnostics.memory")}</p></>}
     <p>{t("diagnostics.identity")}</p>
     <ul>{view.report.items.map(item=><li key={item.query}>{item.query} · {item.propertyId===null?"DeviceDescriptorRead (0)":`PropertyRead object ${item.objectIndex}, PID ${item.propertyId}, index ${item.startIndex}, elements ${item.requestedElements}`} · {item.expectedBytes} bytes</li>)}</ul>
     {view.report.simulated&&<strong>{t("diagnostics.simulated")}</strong>}
     {stale&&<p role="alert">{t("diagnostics.stale")}</p>}
    </section>
    {view.phase==="preview"&&<><label className="support-check"><input type="checkbox" data-diagnostics-read-consent checked={readConsent} disabled={busy||stale} onChange={e=>setReadConsent(e.target.checked)}/>{t("diagnostics.readConsent")}</label>
     <button type="button" data-diagnostics-start disabled={busy||!readConsent||stale} onClick={()=>void perform("start")}>{t("diagnostics.read")}</button></>}
    {acquiring&&<><p role="status">{progress?.memory?t("diagnostics.progressMemory",{identity:progress.identity,...progress.memory}):t("diagnostics.progress",{count:progress?.identity??0})}</p><button type="button" data-diagnostics-cancel disabled={busy||view.cancelRequested} onClick={()=>void perform("cancel")}>{t(view.cancelRequested?"diagnostics.stopping":"diagnostics.cancel")}</button></>}
    {view.phase==="complete"&&<section data-diagnostics-result>
     <h3>{t("diagnostics.result")}</h3>{view.report.stopReason&&<p role="status" data-diagnostics-stop-reason>{t("diagnostics.stopReason",{label:t(stopReasonMessage(view.report.stopReason))})} <code>{view.report.stopReason}</code></p>}
     <ul>{view.report.items.map(i=><li key={i.query}>{i.query}: {t(`diagnostics.status.${i.status}`)} · {i.receivedBytes}/{i.expectedBytes} bytes</li>)}</ul>
     {memory&&<><h4>{t("diagnostics.coverageHeading")}</h4><ul data-diagnostics-coverage>{memory.ranges.map(r=><li key={r.order}>{t("diagnostics.coverage",{address:hexWord(r.address),length:r.length,read:r.coverage.read,refused:r.coverage.refusedAmbiguous,partial:r.coverage.partial,error:r.coverage.error,notRead:r.coverage.notRead})}</li>)}</ul></>}
     <details><summary>{t(memory?"diagnostics.sharePreviewMemory":"diagnostics.sharePreview")}</summary><pre aria-label="report.json">{JSON.stringify(view.report,null,2)}</pre>{memory&&<pre aria-label="memory.json">{JSON.stringify(memory,null,2)}</pre>}</details>
     {report&&<><p data-diagnostics-handoff>{t("diagnostics.handoff")}</p>
      <button type="button" data-report-problem-open="diagnostics" disabled={busy}
       onClick={()=>{close();report({problemType:"deviceReading",deviceId:device,operation:{relatedKey:"diagnosticsToken",relatedValue:view.token}});}}>{t("reportProblem.button")}</button></>}
    </section>}
   </>}
  </div>
 </Overlay>;
}
/** The memory profile's review: application, required identity, declared ranges and their total. */
function MemoryPreview({view,memory}:{view:api.DiagnosticView;memory:api.MemoryEvidence}){
 const t=useTranslate();const plan=memoryPlan(memory);const c=memory.hardwareTypeComparison;
 return <div data-diagnostics-memory-plan>
  <p>{t("diagnostics.memoryLimits")}</p>
  <p>{t("diagnostics.memoryPreview.application",{program:memory.programId,mask:view.report.assertedMask===null?"—":hexWord(view.report.assertedMask),manufacturer:view.report.assertedManufacturer===null?"—":hexWord(view.report.assertedManufacturer),hardware:c.inlineData,position:c.procedurePosition})}</p>
  <p>{t("diagnostics.memoryPreview.ranges")}</p>
  <ul data-diagnostics-ranges>{plan.ranges.map(r=><li key={r.order}>{t("diagnostics.memoryPreview.range",{address:r.address,length:r.length,position:r.procedurePosition})}</li>)}</ul>
  <p data-diagnostics-total>{t("diagnostics.memoryPreview.total",{octets:plan.totalOctets,reads:plan.reads,chunk:plan.chunkOctets})}</p>
  <p className="form-warning">{t("diagnostics.memoryPrivacy")}</p>
 </div>;
}
