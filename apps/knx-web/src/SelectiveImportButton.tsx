/** Selects ETS source devices/lines, previews dependencies and confirms one atomic merge. */
import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import Overlay from "./Overlay";
import { pickOpenPath } from "./filePicker";
import * as api from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import { useTranslate } from "./i18n";
import "./selectiveImport.css";

type Props = { tree: ProjectTree | null; scope: string | number; onTreeUpdate: (tree: ProjectTree) => void };
export default function SelectiveImportButton(props: Props) {
  const t = useTranslate();
  const [open, setOpen] = useState(false);
  const trigger = useRef<HTMLButtonElement>(null);
  const previous = useRef(false);
  useEffect(() => {
    if (previous.current && !open) trigger.current?.closest("details")?.querySelector("summary")?.focus();
    previous.current = open;
  }, [open]);
  return <><button ref={trigger} disabled={!props.tree} onClick={() => setOpen(true)}>{t("selectiveImport.button")}</button>
    {open && props.tree && createPortal(<ImportDialog key={props.scope} {...props} tree={props.tree} onClose={() => setOpen(false)} />, document.body)}</>;
}
function ImportDialog({ tree, scope, onTreeUpdate, onClose }: Props & { tree: ProjectTree; onClose: () => void }) {
  const t = useTranslate();
  const [path, setPath] = useState("");
  const [password, setPassword] = useState("");
  const [inventory, setInventory] = useState<api.ImportSourceInventory | null>(null);
  const [sourceId, setSourceId] = useState<number | null>(null);
  const [targetId, setTargetId] = useState<number | null>(tree.installations.length === 1 ? tree.installations[0].id : null);
  const [devices, setDevices] = useState<number[]>([]);
  const [lines, setLines] = useState<number[]>([]);
  const [preview, setPreview] = useState<api.SelectionPreviewResponse | null>(null);
  const [consent, setConsent] = useState(false);
  const [mode, setMode] = useState<"inspect" | "preview" | "apply" | null>(null);
  const [error, setError] = useState("");
  const [done, setDone] = useState(false);
  const [search, setSearch] = useState("");
  const epoch = useRef(0);
  const acknowledgedRevision = useRef<string | null>(null);
  useEffect(() => () => { ++epoch.current; }, []);
  function invalidate() { ++epoch.current; setPreview(null); setConsent(false); setError(""); setDone(false); }
  useEffect(() => {
    const revision = `${tree.server_incarnation}:${tree.snapshot_revision}`;
    if (acknowledgedRevision.current === revision) { acknowledgedRevision.current = null; return; }
    invalidate(); if (mode !== "apply") setMode(null);
  }, [scope, tree.server_incarnation, tree.snapshot_revision]);
  const installationLabel = (i: { id: number; name: string }) => i.name.trim() ? i.name : t("selectiveImport.unnamedInstallation", { id: i.id });
  const source = inventory?.installations.find(i => i.id === sourceId);
  const target = tree.installations.find(i => i.id === targetId);
  const ready = Boolean(source && target && (devices.length || lines.length));
  const matches = source?.devices.filter(d => `${d.name} ${d.address ?? ""} ${d.id}`.toLocaleLowerCase().includes(search.toLocaleLowerCase())) ?? [];
  const shown = matches.slice(0, 250);
  function body(): api.SelectionRequest {
    if (!inventory || !source || !target) throw new Error(t("selectiveImport.chooseInstallation"));
    return { path, ...(password ? { password } : {}), sourceHash: inventory.sourceHash,
      selection: { sourceInstallation: source.id, targetInstallation: target.id, devices, lines } };
  }
  async function inspect() {
    if (mode) return;
    const id = ++epoch.current; setMode("inspect"); setError(""); setDone(false); setPreview(null); setConsent(false);
    try {
      const chosen = await pickOpenPath([{ name: "KNX project", extensions: ["knxproj"] }]);
      if (!chosen || epoch.current !== id) return;
      const result = await api.inspectImportSource(chosen, password || undefined);
      if (epoch.current !== id) return;
      setPath(chosen); setInventory(result); setSourceId(result.installations.length === 1 ? result.installations[0].id : null);
      setDevices([]); setLines([]); setSearch("");
    } catch (e) { if (epoch.current === id) setError(e instanceof Error ? e.message : String(e)); }
    finally { if (epoch.current === id) setMode(null); }
  }
  async function run(operation: "preview" | "apply") {
    if (mode || !ready || (operation === "apply" && (!preview || !consent))) return;
    const id = ++epoch.current; setMode(operation); setError("");
    try {
      if (operation === "preview") {
        setPreview(null); setConsent(false);
        const result = await api.previewImportSelection(body());
        if (epoch.current === id) setPreview(result);
      } else {
        const result = await api.applyImportSelection({ ...body(), confirmationToken: preview!.confirmationToken });
        if (epoch.current === id) { acknowledgedRevision.current = `${result.project.server_incarnation}:${result.project.snapshot_revision}`; onTreeUpdate(result.project); setPassword(""); setPreview(null); setConsent(false); setDone(true); }
        else { setError(t("selectiveImport.uncertain")); }
      }
    } catch (e) {
      if (epoch.current === id) {
        setError(e instanceof Error ? e.message : String(e)); setPreview(null); setConsent(false);
        if (operation === "apply") {
          // A lost acknowledgement is not proof of rollback. Observe current state,
          // never replay the mutating request automatically.
          setError(`${e instanceof Error ? e.message : String(e)} — ${t("selectiveImport.uncertain")}`);
          try { const current = await api.currentProject(); if (epoch.current === id) onTreeUpdate(current); } catch { /* The uncertainty remains visible. */ }
        }
      }
    } finally { if (epoch.current === id || operation === "apply") setMode(null); }
  }
  function toggle(values: number[], id: number, checked: boolean) { return checked ? [...values, id] : values.filter(value => value !== id); }
  const close = () => { if (mode !== "apply") onClose(); };
  const counts = preview?.preview.counts;
  return <Overlay labelledBy="selective-import-title" className="selective-import-panel" onClose={close}>
    <div className="selective-import-heading"><h2 id="selective-import-title">{t("selectiveImport.title")}</h2><button disabled={mode === "apply"} onClick={close}>{t("selectiveImport.close")}</button></div>
    <p>{t("selectiveImport.intro")}</p>
    <p className="selective-import-warning">{t("selectiveImport.privacy")}</p>
    <label className="settings-field"><span>{t("selectiveImport.password")}</span><input type="password" autoComplete="off" value={password} disabled={Boolean(mode)} onChange={e => { invalidate(); setPassword(e.target.value); setInventory(null); setDevices([]); setLines([]); }} /></label>
    <button disabled={Boolean(mode)} onClick={() => void inspect()}>{t("selectiveImport.choose")}</button>
    {path && <p className="selective-import-source">{path.split(/[\\/]/).pop()}</p>}
    {inventory && <>
      <label className="settings-field"><span>{t("selectiveImport.source")}</span><select value={sourceId ?? ""} disabled={Boolean(mode)} onChange={e => { invalidate(); setSourceId(e.target.value === "" ? null : Number(e.target.value)); setDevices([]); setLines([]); }}>
        <option value="">{t("selectiveImport.chooseInstallation")}</option>{inventory.installations.map(i => <option key={i.id} value={i.id}>{installationLabel(i)}</option>)}
      </select></label>
      <label className="settings-field"><span>{t("selectiveImport.destination")}</span><select value={targetId ?? ""} disabled={Boolean(mode)} onChange={e => { invalidate(); setTargetId(e.target.value === "" ? null : Number(e.target.value)); }}>
        <option value="">{t("selectiveImport.chooseInstallation")}</option>{tree.installations.map(i => <option key={i.id} value={i.id}>{installationLabel(i)}</option>)}
      </select></label>
      {source && <>
        <fieldset disabled={Boolean(mode)}><legend>{t("selectiveImport.lines")}</legend>{source.lines.map(l => <label className="selective-import-choice" key={l.id}>
          <input type="checkbox" data-import-line={l.id} checked={lines.includes(l.id)} onChange={e => { invalidate(); setLines(toggle(lines, l.id, e.target.checked)); }} />{l.address} · {l.name} ({l.deviceCount})
        </label>)}</fieldset>
        <label className="settings-field"><span>{t("selectiveImport.search")}</span><input type="search" value={search} onChange={e => setSearch(e.target.value)} /></label>
        <p>{t("selectiveImport.showing", { shown: shown.length, total: matches.length })}</p>
        <fieldset disabled={Boolean(mode)}><legend>{t("selectiveImport.devices")}</legend>{shown.map(d => <label className="selective-import-choice" key={d.id}>
          <input type="checkbox" data-import-device={d.id} checked={devices.includes(d.id)} onChange={e => { invalidate(); setDevices(toggle(devices, d.id, e.target.checked)); }} />{d.address ?? "—"} · {d.name} (ID {d.id})
        </label>)}</fieldset>
      </>}
      <details><summary>{t("selectiveImport.report")}</summary><pre>{JSON.stringify(inventory.sourceReport, null, 2)}</pre></details>
    </>}
    {mode && <p role="status">{t("selectiveImport.busy")}</p>}
    {error && <p role="alert">{error}</p>}
    {done && <p role="status">{t("selectiveImport.done")}</p>}
    <div className="selective-import-actions"><button disabled={!ready || Boolean(mode) || done} onClick={() => void run("preview")}>{t("selectiveImport.preview")}</button></div>
    {preview && counts && <section data-import-preview>
      <p>{t("selectiveImport.summary", { devices: counts.devices, objects: counts.communicationObjects, parameters: counts.parameters, modules: counts.modules, addresses: counts.groupAddresses, lines: counts.lines, buildings: counts.buildingParts })}</p>
      <details><summary>{t("selectiveImport.mappings")}</summary><ul>{preview.preview.mappings.map((m, index) => <li key={index}>{m.kind}: {m.source} → {m.target} ({t(m.reused ? "selectiveImport.reused" : "selectiveImport.created")})</li>)}</ul></details>
      {preview.preview.notes.map((note, index) => <p key={index}>{note}</p>)}
      <label className="selective-import-choice"><input type="checkbox" data-import-consent checked={consent} disabled={Boolean(mode)} onChange={e => setConsent(e.target.checked)} />{t("selectiveImport.consent")}</label>
      <div className="selective-import-actions"><button disabled={!consent || Boolean(mode)} onClick={() => void run("apply")}>{t("selectiveImport.apply")}</button></div>
    </section>}
  </Overlay>;
}
