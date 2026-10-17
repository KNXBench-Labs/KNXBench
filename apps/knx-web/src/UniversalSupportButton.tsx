/** Universal device evidence intake with separate acquisition and disclosure consent. */
import {useEffect,useRef,useState} from "react";
import {createPortal} from "react-dom";
import Overlay from "./Overlay";
import {useTranslate} from "./i18n";
import type {ProjectTree} from "./bindings/ProjectTree";
import {collectDevices,projectContextKey} from "./treeUtils";
import {previewSupport,diagnoseSupport,saveSupport,type SupportPreview,type SupportBinding} from "./supportApi";
import "./universalSupport.css";
import DiagnosticExportButton from "./DiagnosticExportButton";

export default function UniversalSupportButton({project,deviceId=null,downloadId}:{project:ProjectTree|null;deviceId?:number|null;downloadId?:number}){
 const t=useTranslate();const [open,setOpen]=useState(false);
 // Equal refreshes keep the draft; a changed server/project/revision aborts it.
 const projectBinding=project?`${projectContextKey(project)}/${project.snapshot_revision??"unbound"}`:"no-project";
 useEffect(()=>setOpen(false),[projectBinding,deviceId,downloadId]);
 return <><button type="button" data-support-open onClick={()=>setOpen(true)}>{t("support.title")}</button><DiagnosticExportButton project={project} deviceId={deviceId}/>{open&&createPortal(<SupportDialog project={project} initialDevice={deviceId} downloadId={downloadId} close={()=>setOpen(false)}/>,document.body)}</>;
}
function SupportDialog({project,initialDevice,downloadId,close}:{project:ProjectTree|null;initialDevice:number|null;downloadId?:number;close:()=>void}){
 const t=useTranslate();const [device,setDevice]=useState<number|null>(initialDevice);
 const [localAddress,setLocalAddress]=useState("");
 const [program,setProgram]=useState("");const [firmware,setFirmware]=useState("");const [mask,setMask]=useState("");
 const [functional,setFunctional]=useState<"not-checked"|"works-as-configured"|"does-not-work-as-configured">("not-checked");
 const [context,setContext]=useState(false);const [rights,setRights]=useState(false);
 const [gateway,setGateway]=useState("");const [liveConsent,setLiveConsent]=useState(false);const [exportConsent,setExportConsent]=useState(false);
 const [preview,setPreview]=useState<SupportPreview|null>(null);const [busy,setBusy]=useState(false);const [error,setError]=useState(false);const [saved,setSaved]=useState(false);
 const epoch=useRef(0);const abort=useRef<AbortController|null>(null);
 useEffect(()=>()=>{++epoch.current;abort.current?.abort();},[]);
 const devices=project?[...collectDevices(project).values()]:[];
 function invalidate(){++epoch.current;abort.current?.abort();setBusy(false);setPreview(null);setLiveConsent(false);setExportConsent(false);setSaved(false);setError(false);}
 const binding:SupportBinding|undefined=project?.server_incarnation!==undefined&&project.project_incarnation!==undefined&&project.snapshot_revision!==undefined?{serverIncarnation:project.server_incarnation,projectIncarnation:project.project_incarnation,revision:project.snapshot_revision}:undefined;
 const valid=(!mask||/^[a-fA-F0-9]{4}$/.test(mask))&&(!context||rights);
 async function run(kind:"preview"|"read"|"save"){
  if(busy||!valid||(kind==="read"&&(!preview||!liveConsent))||(kind==="save"&&(!preview||!exportConsent)))return;
  const version=++epoch.current;const controller=new AbortController();abort.current=controller;setBusy(true);setError(false);setSaved(false);setExportConsent(false);setLiveConsent(false);
  try{
   if(kind==="save"){await saveSupport(preview!,controller.signal);if(epoch.current===version)setSaved(true);}
   else {const result=kind==="read"?await diagnoseSupport(preview!.token,gateway,controller.signal):await previewSupport({programId:program||undefined,firmware:firmware||undefined,mask:mask?parseInt(mask,16):undefined,functionalCheck:functional,includeContext:context,contextRights:rights},device,binding,device===initialDevice?downloadId:undefined,device===null?(localAddress||undefined):undefined,controller.signal);if(epoch.current===version)setPreview(result);}
  }catch {if(epoch.current===version&&!controller.signal.aborted)setError(true);}
  finally{if(epoch.current===version)setBusy(false);}
 }
 return <Overlay labelledBy="universal-support-title" className="universal-support" resizable={{width:850,height:760}} onClose={close}>
  <div data-universal-support><header><h2 id="universal-support-title">{t("support.title")}</h2><button onClick={close}>{t("support.close")}</button></header>
   <p>{t("support.intro")}</p><p className="form-warning">{t("support.privacy")}</p>
   <fieldset disabled={busy}>
    <label>{t("support.device")}<select data-support-device value={device??""} onChange={e=>{invalidate();setDevice(e.target.value===""?null:Number(e.target.value));}}><option value="">{t("support.manual")}</option>{devices.map(d=><option key={d.id} value={d.id}>{d.address??"—"} · {d.name}</option>)}</select></label>
    {device===null&&<label>{t("support.target")}<input data-support-local-address value={localAddress} maxLength={10} onChange={e=>{invalidate();setLocalAddress(e.target.value);}} placeholder="1.1.67"/></label>}
    <label>{t("support.program")}<input data-support-program value={program} maxLength={64} onChange={e=>{invalidate();setProgram(e.target.value);}} placeholder="M-0001_A-9205-04-9270"/></label>
    <label>{t("support.mask")}<input data-support-mask value={mask} maxLength={4} onChange={e=>{invalidate();setMask(e.target.value);}} placeholder="0025" aria-invalid={mask!==""&&!/^[a-fA-F0-9]{4}$/.test(mask)}/></label>
    <label>{t("support.firmware")}<input data-support-firmware value={firmware} maxLength={48} onChange={e=>{invalidate();setFirmware(e.target.value);}}/></label>
    <label>{t("support.functional")}<select data-support-functional value={functional} onChange={e=>{invalidate();setFunctional(e.target.value as typeof functional);}}><option value="not-checked">{t("support.notChecked")}</option><option value="works-as-configured">{t("support.works")}</option><option value="does-not-work-as-configured">{t("support.doesNotWork")}</option></select></label>
    <label className="support-check"><input type="checkbox" data-support-context checked={context} onChange={e=>{invalidate();setContext(e.target.checked);setRights(false);}}/>{t("support.context")}</label>
    {context&&<label className="support-check"><input type="checkbox" data-support-rights checked={rights} onChange={e=>{invalidate();setRights(e.target.checked);}}/>{t("support.rights")}</label>}
    <button type="button" data-support-preview disabled={!valid} onClick={()=>void run("preview")}>{t("support.preview")}</button>
   </fieldset>
   {busy&&<p role="status" className="task-status-text">{t("support.busy")}</p>}{error&&<p role="alert">{t("support.failed")}</p>}
   {preview&&<>
    <section data-support-preview-content><h3>{t("support.preview")}</h3><pre>{preview.summary}</pre><pre aria-label="report.json">{JSON.stringify(preview.report,null,2)}</pre></section>
    <details><summary>{t("support.live")}</summary><p>{t("support.liveWarning")}</p><p data-support-local-target>{preview.localAddress??"—"} · {preview.profile}</p><ul>{preview.reads.map((r,i)=><li key={i}>{r}</li>)}</ul>
     <label>{t("support.gateway")}<input data-support-gateway value={gateway} disabled={busy} onChange={e=>{setGateway(e.target.value);setLiveConsent(false);}} placeholder="192.0.2.10:3671"/></label>
     <label className="support-check"><input data-support-live-consent type="checkbox" checked={liveConsent} disabled={busy||!preview.localAddress} onChange={e=>setLiveConsent(e.target.checked)}/>{t("support.liveConsent")}</label>
     <button data-support-read disabled={busy||!liveConsent||!preview.localAddress||!gateway} onClick={()=>void run("read")}>{t("support.read")}</button>
    </details>
    <label className="support-check"><input data-support-export-consent type="checkbox" checked={exportConsent} disabled={busy} onChange={e=>setExportConsent(e.target.checked)}/>{t("support.exportConsent")}</label>
    <button data-support-save disabled={busy||!exportConsent} onClick={()=>void run("save")}>{t("support.save")}</button>
    {saved&&<p role="status">{t("support.saved")}</p>}
    <p>{t("support.handoff")}</p><div className="support-actions"><a href="https://github.com/KNXBench-Labs/KNXBench/issues/new?template=analysis.yml" target="_blank" rel="noopener noreferrer">{t("deviceDownload.help.issue")}</a><a href="mailto:issue@knxbench.com?subject=KNXBench%20device%20support%20report">{t("deviceDownload.help.mail")}</a></div>
   </>}
  </div>
 </Overlay>;
}
