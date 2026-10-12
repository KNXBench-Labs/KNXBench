/** Isolated notes host with actual context and shared help; not production chrome. */
import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import "../../styles.css";
import { ProjectNotesEditor } from "./ProjectNotesEditor";
import { createNotesClient, type NotesContext } from "./client";
import HelpPanel from "../../HelpPanel";
import { HELP_TOPICS, type HelpTopicId } from "../../help";
import { useUiLanguage } from "../../uiLanguage";
const params=new URLSearchParams(location.search);
const language=params.get("lang")==="de"?"de":"en";
document.documentElement.lang=language;
document.documentElement.dataset.theme=params.get("theme")==="porcelain"?"porcelain":"graphite";
document.documentElement.dataset.motionLevel="off";
const client=createNotesClient();
function Host(){
 const [,setLanguage]=useUiLanguage();const [context,setContext]=useState<NotesContext|null>(null);
 const [error,setError]=useState("");const [help,setHelp]=useState<HelpTopicId|null>(null);
 const [dirty,setDirty]=useState(false);
 async function refresh(){const view=await client.read();setContext({serverIncarnation:view.serverIncarnation,projectIncarnation:view.projectIncarnation,snapshotRevision:view.snapshotRevision});}
 useEffect(()=>{setLanguage(language);void refresh().catch(reason=>setError(String(reason)));},[setLanguage]);
 useEffect(()=>{function open(event:Event){const topic=(event as CustomEvent<{topicId?:unknown}>).detail?.topicId;const found=HELP_TOPICS.find(entry=>entry.id===topic);if(found)setHelp(found.id);}window.addEventListener("knxbench:open-help",open);return()=>window.removeEventListener("knxbench:open-help",open);},[]);
 return <main style={{maxWidth:1100,margin:"24px auto",padding:12,background:"var(--knx-bg)"}}>
  <p style={{color:"var(--knx-muted)"}}>Isolated component fixture with HTTP/native storage. No workbench, bus or ETS claim.</p>
  <output data-fixture-dirty={dirty}>{dirty?"Local draft":"No local draft"}</output>
  {error&&<p role="alert">{error}</p>}{context&&<ProjectNotesEditor context={context} language={language} client={client} onApplied={async()=>{await refresh();}} onDirtyChange={setDirty}/>}
  {help&&<HelpPanel initialTopicId={help} onClose={()=>setHelp(null)}/>}
 </main>;
}
createRoot(document.getElementById("root")!).render(<Host/>);
