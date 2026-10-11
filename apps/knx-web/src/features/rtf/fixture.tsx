/** Synthetic host for isolated instruction editor acceptance. */
import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import "../../styles.css";
import { InstructionEditor } from "./InstructionEditor";
import { createInstructionClient, type Context } from "./client";
const params = new URLSearchParams(location.search);
const deviceId = Number(params.get("device") ?? "1");
const language = params.get("lang") === "de" ? "de" : "en";
document.documentElement.lang = language;
document.documentElement.dataset.theme = params.get("theme") === "porcelain" ? "porcelain" : "graphite";
const client = createInstructionClient();
function Host() {
  const [context, setContext] = useState<Context | null>(null); const [error, setError] = useState("");
  async function refresh() { const view = await client.read(deviceId); setContext({serverIncarnation:view.serverIncarnation,projectIncarnation:view.projectIncarnation,snapshotRevision:view.snapshotRevision}); }
  useEffect(() => { void refresh().catch(reason => setError(String(reason))); }, []);
  return <main style={{maxWidth:880,margin:"32px auto",padding:20,background:"var(--knx-bg)"}}><p style={{color:"var(--knx-muted)"}}>Synthetic component fixture with real HTTP/native storage. Not a workbench integration.</p>{error && <p role="alert">{error}</p>}{context && <InstructionEditor deviceId={deviceId} context={context} language={language} client={client} onApplied={() => void refresh().catch(reason => setError(String(reason)))}/>}</main>;
}
createRoot(document.getElementById("root")!).render(<Host/>);
