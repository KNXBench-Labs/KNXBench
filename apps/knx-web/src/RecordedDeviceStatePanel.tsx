/** Read-only existing model projection; never queries or programs a device. */
import {useEffect,useRef,useState} from "react";
import type {ProjectTree} from "./bindings/ProjectTree";
import type {RecordedDeviceStateView} from "./recordedDeviceState";
import {useTranslate} from "./i18n";
import * as api from "./api";
import "./RecordedDeviceStatePanel.css";
export default function RecordedDeviceStatePanel(props:{tree:ProjectTree;deviceId:number}) {
 const t=useTranslate();const [view,setView]=useState<RecordedDeviceStateView|null>(null);const [error,setError]=useState<string|null>(null);const [busy,setBusy]=useState(false);
 const epoch=useRef(0);const pending=useRef(false);const mounted=useRef(false);
 useEffect(()=>{mounted.current=true;epoch.current++;pending.current=false;setBusy(false);setView(null);setError(null);return()=>{mounted.current=false;epoch.current++;};},[props.deviceId,props.tree.server_incarnation,props.tree.project_incarnation,props.tree.snapshot_revision]);
 async function read(){
  if(pending.current)return;pending.current=true;setBusy(true);setView(null);setError(null);const started=epoch.current;
  try{const value=await api.getRecordedDeviceState(props.deviceId);if(started!==epoch.current)return;
   if(value.state.device_id!==props.deviceId||value.server_incarnation!==props.tree.server_incarnation||value.project_incarnation!==props.tree.project_incarnation||value.snapshot_revision!==props.tree.snapshot_revision)throw new Error(t("recordedDevice.changed"));
   setView(value);
  }catch(failure){if(started===epoch.current)setError(api.errorMessage(failure));}
  finally{if(started===epoch.current){pending.current=false;if(mounted.current)setBusy(false);}}
 }
 const state=view?.state;
 return <section className="recorded-device-state-panel">
  <button type="button" disabled={busy} onClick={()=>void read()}>{t("recordedDevice.show")}</button>
  {error&&<p role="alert">{error}</p>}
  {state&&<div><h3>{t("recordedDevice.title")}</h3><p>{t("recordedDevice.explanation")}</p><p>{t("recordedDevice.dates")}</p><p>{t("recordedDevice.privacy")}</p>
   <dl><div><dt>{t("recordedDevice.name")}</dt><dd><bdi dir="auto">{state.name}</bdi></dd></div>
    <div><dt>{t("recordedDevice.completion")}</dt><dd>{t(`projectMetadata.completion.${state.completion}`)}</dd></div>
    <div><dt>{t("recordedDevice.broken")}</dt><dd>{t(state.broken?"recordedDevice.yes":"recordedDevice.no")}</dd></div>
    <div><dt>{t("recordedDevice.modified")}</dt><dd>{state.last_modified??t("projectMetadata.absent")}</dd></div>
    <div><dt>{t("recordedDevice.download")}</dt><dd>{state.last_download??t("projectMetadata.absent")}</dd></div>
   </dl>
   <ul>{state.parts.map(part=><li key={part.part}><h4>{t(`recordedDevice.part.${part.part}`)}</h4><dl>
    <div><dt>{t("recordedDevice.belief")}</dt><dd>{t(part.intended_loaded?"recordedDevice.loaded":"recordedDevice.notLoaded")}</dd></div>
    <div><dt>{t("recordedDevice.report")}</dt><dd>{part.reported===null?t("recordedDevice.absent"):<>{t(`recordedDevice.report.${part.reported as 0|1|2|3|4|5}`)} <span className="mono">({part.reported})</span></>}</dd></div>
   </dl></li>)}</ul>
  </div>}
 </section>;
}
