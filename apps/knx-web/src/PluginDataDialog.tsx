/** Adopts one device's plug-in bytes from a source archive: list, preview, confirm. */
import { useEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import Overlay from "./Overlay";
import { pickOpenPath } from "./filePicker";
import * as api from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import { useTranslate } from "./i18n";
import {
  addressDifference, classifyAdoptionError, pairKey, refusalMessage, visiblePairs,
  type AdoptionApplyResponse, type AdoptionCandidate, type AdoptionDevice, type AdoptionError,
  type AdoptionListing, type AdoptionPreviewResponse,
} from "./pluginData";
import "./pluginData.css";

/** Label of the version the server keeps before an adoption (ADR-0136 amendment). */
const ROLLBACK_LABEL = "Before plug-in data adoption";

type Props = {
  tree: ProjectTree;
  /** The device panel's device: only its pairs are listed until the user asks for all. */
  deviceId: number | null;
  onTreeUpdate: (tree: ProjectTree) => void;
  onClose: () => void;
  onOpenHistory?: () => void;
};

type Mode = "list" | "preview" | "apply" | "reload";

function bindingOf(tree: ProjectTree) {
  const { server_incarnation: serverIncarnation, project_incarnation: projectIncarnation, snapshot_revision: snapshotRevision } = tree;
  if (serverIncarnation === undefined || projectIncarnation === undefined || snapshotRevision === undefined) return null;
  return { serverIncarnation, projectIncarnation, snapshotRevision };
}
const bindingKey = (tree: ProjectTree) => `${tree.server_incarnation}:${tree.project_incarnation}:${tree.snapshot_revision}`;

export function PluginDataButton(props: { tree: ProjectTree | null; deviceId: number; onTreeUpdate: (tree: ProjectTree) => void; onOpenHistory?: () => void }) {
  const t = useTranslate();
  const [open, setOpen] = useState(false);
  const trigger = useRef<HTMLButtonElement>(null);
  return <>
    <button ref={trigger} type="button" data-plugin-data-open disabled={!props.tree} onClick={() => setOpen(true)}>{t("pluginData.button")}</button>
    {open && props.tree && createPortal(<PluginDataDialog tree={props.tree} deviceId={props.deviceId} onTreeUpdate={props.onTreeUpdate}
      onOpenHistory={props.onOpenHistory} onClose={() => { setOpen(false); trigger.current?.focus(); }} />, document.body)}
  </>;
}

export default function PluginDataDialog({ tree, deviceId, onTreeUpdate, onClose, onOpenHistory }: Props) {
  const t = useTranslate();
  const [password, setPassword] = useState("");
  const [path, setPath] = useState("");
  const [listing, setListing] = useState<AdoptionListing | null>(null);
  const [showAll, setShowAll] = useState(false);
  const [selected, setSelected] = useState<string | null>(null);
  const [shown, setShown] = useState<AdoptionPreviewResponse | null>(null);
  const [consent, setConsent] = useState(false);
  const [mode, setMode] = useState<Mode | null>(null);
  const [error, setError] = useState<AdoptionError | null>(null);
  const [uncertain, setUncertain] = useState(false);
  const [done, setDone] = useState<AdoptionApplyResponse | null>(null);
  const epoch = useRef(0);
  /** The server holds the last previewed plan; whatever ends a preview here releases it. */
  const shownToken = useRef<string | null>(null);
  const acknowledged = useRef<string | null>(null);
  const pairsRef = useRef<HTMLFieldSetElement>(null);
  const previewRef = useRef<HTMLHeadingElement>(null);
  const doneRef = useRef<HTMLParagraphElement>(null);
  const alertRef = useRef<HTMLParagraphElement>(null);
  const previousMode = useRef<Mode | null>(null);
  // A step's button is disabled while it runs, which drops focus out of the
  // dialog (and out of its Escape/Tab handling). Its result takes focus.
  useEffect(() => {
    const finished = previousMode.current !== null && mode === null;
    previousMode.current = mode;
    if (!finished) return;
    (alertRef.current ?? doneRef.current ?? previewRef.current ?? pairsRef.current)?.focus();
  }, [mode]);

  function dropShown() {
    const token = shownToken.current;
    shownToken.current = null;
    if (token) api.cancelPluginData(token).catch(() => { /* The plan is inert once stale. */ });
    setShown(null); setConsent(false);
  }
  useEffect(() => () => { ++epoch.current; dropShown(); }, []);
  const revision = bindingKey(tree);
  useEffect(() => {
    if (acknowledged.current === revision) { acknowledged.current = null; return; }
    // A change elsewhere makes a shown preview stale; never apply it. A listing
    // or preview still in flight was read against the old state: discard it.
    if (mode === "list" || mode === "preview") { ++epoch.current; setMode(null); }
    dropShown();
  }, [revision]);

  const binding = bindingOf(tree);
  const { shown: pairs, hidden } = listing ? visiblePairs(listing.candidates, deviceId, showAll) : { shown: [], hidden: 0 };
  const pair = pairs.find(c => pairKey(c) === selected && !c.refusalDetail) ?? null;
  const native = listing?.persistence === "native";
  const canPreview = Boolean(pair && native && binding && !mode);

  function fail(e: unknown) { setError(classifyAdoptionError(e)); setUncertain(false); }

  async function chooseSource() {
    if (mode) return;
    const id = ++epoch.current;
    setMode("list"); setError(null); setUncertain(false); setDone(null); dropShown();
    try {
      const chosen = await pickOpenPath([{ name: "KNX project", extensions: ["knxproj"] }]);
      if (!chosen || epoch.current !== id) return;
      const result = await api.listPluginDataCandidates(chosen, password || undefined);
      if (epoch.current !== id) return;
      setPath(chosen); setListing(result); setSelected(null); setShowAll(false);
    } catch (e) { if (epoch.current === id) fail(e); }
    finally { if (epoch.current === id) setMode(null); }
  }

  async function preview() {
    if (!canPreview || !pair || !listing || !binding) return;
    const id = ++epoch.current;
    setMode("preview"); setError(null); setUncertain(false); setDone(null); dropShown();
    try {
      const result = await api.previewPluginData({ path, ...(password ? { password } : {}), sourceHash: listing.sourceHash,
        request: { sourceDevice: pair.sourceDevice.id, targetDevice: pair.targetDevice.id }, ...binding });
      if (epoch.current !== id) { api.cancelPluginData(result.confirmationToken).catch(() => undefined); return; }
      shownToken.current = result.confirmationToken; setShown(result);
    } catch (e) { if (epoch.current === id) fail(e); }
    finally { if (epoch.current === id) setMode(null); }
  }

  async function apply() {
    if (!shown || !consent || mode) return;
    const id = ++epoch.current;
    setMode("apply"); setError(null); setUncertain(false);
    const token = shown.confirmationToken;
    try {
      const result = await api.applyPluginData(token);
      shownToken.current = null;
      acknowledged.current = bindingKey(result.project);
      onTreeUpdate(result.project);
      setDone(result); setShown(null); setConsent(false); setPassword("");
    } catch (e) {
      const failure = classifyAdoptionError(e);
      const answered = typeof (e as { status?: unknown } | null)?.status === "number";
      // The server drops a plan it refuses. Without an answer, cancelling is
      // harmless either way and stops a delayed apply from landing later.
      if (!answered) dropShown(); else { shownToken.current = null; setShown(null); setConsent(false); }
      setError(failure); setUncertain(!answered);
      if (!answered) {
        // A lost acknowledgement is not proof of rollback: observe, never replay.
        try { onTreeUpdate(await api.currentProject()); } catch { /* The uncertainty stays visible. */ }
      }
    } finally { if (epoch.current === id) setMode(null); }
  }

  async function reload() {
    if (mode) return;
    setMode("reload");
    try { onTreeUpdate(await api.currentProject()); } catch (e) { fail(e); }
    finally { setMode(null); }
  }

  function close() { if (mode === "apply") return; dropShown(); onClose(); }

  function errorText(failure: AdoptionError): string {
    switch (failure.kind) {
      case "refused": return refusalMessage(t, failure.refusal);
      case "stale": return t("pluginData.error.stale");
      case "sourceChanged": return t("pluginData.error.sourceChanged");
      case "nativeRequired": return t("pluginData.sessionOnly");
      case "rollbackUnavailable": return t("pluginData.error.rollbackUnavailable", { detail: failure.message });
      case "historyStale": return t("pluginData.error.historyStale");
      case "source": return t("pluginData.error.source", { detail: failure.message });
      case "target": return t("pluginData.error.target");
      case "other": return failure.message;
    }
  }
  const deviceLabel = (d: AdoptionDevice) => d.address ? `${d.name} (${d.address})` : d.name;
  const pairStatus = (c: AdoptionCandidate) => c.refusalDetail
    ? t("pluginData.pair.cannot", { reason: refusalMessage(t, c.refusalDetail) })
    : t("pluginData.pair.can", { count: c.replacements });
  const address = shown ? addressDifference(shown.preview) : null;
  const none = t("pluginData.note.noAddress");

  return <Overlay labelledBy="plugin-data-title" className="plugin-data-panel" onClose={close}>
    <div className="plugin-data-heading"><h2 id="plugin-data-title">{t("pluginData.title")}</h2>
      <button type="button" disabled={mode === "apply"} onClick={close}>{t("pluginData.close")}</button></div>
    <p>{t("pluginData.intro")}</p>
    <label className="settings-field"><span>{t("pluginData.password")}</span>
      <input type="password" autoComplete="off" value={password} disabled={Boolean(mode)}
        onChange={e => { dropShown(); setListing(null); setSelected(null); setPassword(e.target.value); }} /></label>
    <button type="button" disabled={Boolean(mode)} onClick={() => void chooseSource()}>{t("pluginData.choose")}</button>
    {path && listing && <p className="plugin-data-source">{t("pluginData.source", { name: path.split(/[\\/]/).pop() ?? path })}</p>}
    {listing && !native && <p className="plugin-data-warning" data-plugin-session>{t("pluginData.sessionOnly")}</p>}
    {listing && <fieldset ref={pairsRef} tabIndex={-1} data-plugin-pairs className="plugin-data-pairs" disabled={Boolean(mode)}>
      <legend>{t("pluginData.pairs.legend")}</legend>
      {listing.candidates.length === 0 ? <p>{t("pluginData.pairs.none")}</p>
        : pairs.length === 0 ? <p>{t("pluginData.pairs.noneForDevice")}</p> : null}
      <ul>{pairs.map(c => {
        const key = pairKey(c);
        const label = t("pluginData.pair.label", { source: deviceLabel(c.sourceDevice), target: deviceLabel(c.targetDevice) });
        return <li key={key} className={c.refusalDetail ? "plugin-data-pair plugin-data-pair-refused" : "plugin-data-pair"}>
          {c.refusalDetail
            ? <span><strong>{label}</strong><br /><span className="plugin-data-reason">{pairStatus(c)}</span></span>
            : <label><input type="radio" name="plugin-data-pair" data-plugin-pair={key} checked={selected === key}
                onChange={() => { dropShown(); setError(null); setSelected(key); }} />
              <span><strong>{label}</strong><br /><span>{pairStatus(c)}</span></span></label>}
        </li>;
      })}</ul>
      {hidden > 0 && <p>{t("pluginData.pairs.hidden", { count: hidden })}</p>}
      {deviceId !== null && listing.candidates.length > 0 && <label className="plugin-data-choice">
        <input type="checkbox" data-plugin-show-all checked={showAll} onChange={e => setShowAll(e.target.checked)} />{t("pluginData.pairs.showAll")}</label>}
    </fieldset>}
    {mode && <p role="status"><span className="task-status-text">{t("pluginData.busy")}</span></p>}
    {error && <p ref={alertRef} tabIndex={-1} role="alert">{errorText(error)}{uncertain ? ` — ${t("pluginData.uncertain")}` : ""}</p>}
    {error?.kind === "stale" && <button type="button" disabled={Boolean(mode)} onClick={() => void reload()}>{t("pluginData.reload")}</button>}
    {listing && <div className="plugin-data-actions"><button type="button" disabled={!canPreview} onClick={() => void preview()}>{t("pluginData.preview")}</button></div>}
    {shown && <section className="plugin-data-preview" data-plugin-preview aria-labelledby="plugin-data-preview-title">
      <h3 ref={previewRef} tabIndex={-1} id="plugin-data-preview-title">{t("pluginData.previewHeading")}</h3>
      <p>{t("pluginData.previewDevices", { source: deviceLabel(shown.preview.sourceDevice), target: deviceLabel(shown.preview.targetDevice) })}</p>
      <ul className="plugin-data-replacements">{shown.preview.replacements.map(r => <li key={r.path}>
        <strong>{t(`pluginData.channel.${r.channel}`)}</strong>
        <dl>
          <dt>{t("pluginData.entry.path")}</dt><dd className="mono">{r.path}</dd>
          <dt>{t("pluginData.entry.size")}</dt><dd>{t("pluginData.entry.sizeChange", { before: r.bytesBefore, after: r.bytesAfter })}</dd>
          <dt>{t("pluginData.entry.before")}</dt><dd className="mono">{r.sha256Before}</dd>
          <dt>{t("pluginData.entry.after")}</dt><dd className="mono">{r.sha256After}</dd>
        </dl></li>)}</ul>
      {shown.preview.unchangedPaths.length > 0 && <details><summary>{t("pluginData.unchanged", { count: shown.preview.unchangedPaths.length })}</summary>
        <ul>{shown.preview.unchangedPaths.map(p => <li key={p} className="mono">{p}</li>)}</ul></details>}
      <ul className="plugin-data-notes">
        <li>{t("pluginData.note.verbatim")}</li>
        <li>{t("pluginData.note.kept")}</li>
        <li><strong>{t("pluginData.note.noUndo", { label: ROLLBACK_LABEL })}</strong></li>
        {address && <li>{t("pluginData.note.address", { source: address.source ?? none, target: address.target ?? none })}</li>}
      </ul>
      {shown.preview.notes.length > 0 && <details><summary>{t("pluginData.serverNotes")}</summary>
        <ul>{shown.preview.notes.map((note, index) => <li key={index}>{note}</li>)}</ul></details>}
      <label className="plugin-data-choice"><input type="checkbox" data-plugin-consent checked={consent} disabled={Boolean(mode)}
        onChange={e => setConsent(e.target.checked)} />{t("pluginData.consent")}</label>
      <div className="plugin-data-actions">
        <button type="button" className="primary-action" disabled={!consent || Boolean(mode)} onClick={() => void apply()}>{t("pluginData.apply")}</button>
        <button type="button" disabled={Boolean(mode)} onClick={() => dropShown()}>{t("pluginData.cancelPreview")}</button>
      </div>
    </section>}
    {done && <div className="plugin-data-done">
      <p ref={doneRef} tabIndex={-1} role="status" data-plugin-done>{t("pluginData.done", { count: done.preview.replacements.length, label: done.rollbackVersion.label })}</p>
      {onOpenHistory && <button type="button" onClick={() => { close(); onOpenHistory(); }}>{t("pluginData.openHistory")}</button>}
    </div>}
  </Overlay>;
}
