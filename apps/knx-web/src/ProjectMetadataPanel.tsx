/** Explicit read-only view of project metadata already held by the normalized model. */
import {useEffect,useRef,useState} from "react";
import type {ProjectTree} from "./bindings/ProjectTree";
import type {ProjectMetadataView,ProjectMetadata} from "./projectMetadata";
import type {MessageKey} from "./messages/en";
import {useTranslate} from "./i18n";
import * as api from "./api";
import "./ProjectMetadataPanel.css";
const COMPLETION:Record<ProjectMetadata["completion"],MessageKey>={
 Undefined:"projectMetadata.completion.Undefined",Editing:"projectMetadata.completion.Editing",FinishedDesign:"projectMetadata.completion.FinishedDesign",FinishedCommissioning:"projectMetadata.completion.FinishedCommissioning",
 Tested:"projectMetadata.completion.Tested",Locked:"projectMetadata.completion.Locked",Accepted:"projectMetadata.completion.Accepted",
};
const STYLE:Record<ProjectMetadata["group_address_style"],MessageKey>={Free:"projectMetadata.style.Free",TwoLevel:"projectMetadata.style.TwoLevel",ThreeLevel:"projectMetadata.style.ThreeLevel"};
export default function ProjectMetadataPanel(props:{tree:ProjectTree}) {
 const t=useTranslate();const [view,setView]=useState<ProjectMetadataView|null>(null);const [error,setError]=useState<string|null>(null);const [busy,setBusy]=useState(false);
 const epoch=useRef(0);const pending=useRef(false);const mounted=useRef(false);
 useEffect(()=>{mounted.current=true;epoch.current+=1;setView(null);setError(null);return()=>{mounted.current=false;epoch.current+=1;};},[props.tree.server_incarnation,props.tree.project_incarnation,props.tree.snapshot_revision]);
 async function read() {
  if(pending.current)return;pending.current=true;setBusy(true);setError(null);const started=epoch.current;
  try {const value=await api.getProjectMetadata();if(started!==epoch.current)return;
   if(value.server_incarnation!==props.tree.server_incarnation||value.project_incarnation!==props.tree.project_incarnation||value.snapshot_revision!==props.tree.snapshot_revision)throw new Error(t("projectMetadata.changed"));
   setView(value);
  }catch(failure){if(started===epoch.current)setError(api.errorMessage(failure));}
  finally{pending.current=false;if(mounted.current)setBusy(false);}
 }
 const info=view?.metadata;
 function text(value:string|null){return value===null?t("projectMetadata.absent"):value===""?t("projectMetadata.empty"):value;}
 const rows:readonly [MessageKey,string|number][] = info?[
  ["projectMetadata.projectId",text(info.project_id)],["projectMetadata.name",text(info.name)],["projectMetadata.number",text(info.project_number)],
  ["projectMetadata.style",t(STYLE[info.group_address_style])],["projectMetadata.completion",t(COMPLETION[info.completion])],
  ["projectMetadata.modified",text(info.last_modified)],["projectMetadata.start",text(info.project_start)],
  ["projectMetadata.modelVersion",info.model_schema_version],["projectMetadata.etsVersion",info.ets_schema_version],["projectMetadata.unlifted",info.unlifted_group_address_dpt_declarations],
 ]:[];
 return <section className="project-metadata-panel">
  <button type="button" disabled={busy} onClick={()=>void read()}>{t("projectMetadata.show")}</button>
  {error&&<p role="alert">{error}</p>}
  {info&&<div><h3>{t("projectMetadata.title")}</h3><p>{t("projectMetadata.explanation")}</p><p>{t("projectMetadata.dates")}</p><p>{t("projectMetadata.privacy")}</p>
   <dl>{rows.map(([label,value])=><div key={label}><dt>{t(label)}</dt><dd><bdi dir="auto">{value}</bdi></dd></div>)}</dl>
  </div>}
 </section>;
}
