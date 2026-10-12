/** Safe, preview-first installation instruction editor. */
import { useEffect, useId, useRef, useState } from "react";
import { createInstructionClient, sameContext, type Context, type InstructionApplied, type InstructionClient, type InstructionPreview, type InstructionView } from "./client";
import { displayText, formatSelection, literal, replaceText, type Document, type Style } from "./document";
import { editorCopy } from "./copy";
import styles from "./editor.module.css";
/** `context: null` (no complete project identity) mounts a read-only view: it reads, never writes. */
export type InstructionEditorProps = Readonly<{ deviceId: number; context: Context | null; language?: "en" | "de"; client?: InstructionClient; onApplied?: (outcome: InstructionApplied) => void; onDirtyChange?: (dirty: boolean) => void }>;
const defaultClient = createInstructionClient();
function equalDocument(a: Document, b: Document): boolean { return a.runs.length === b.runs.length && a.runs.every((run, i) => { const other = b.runs[i]; return run.text === other.text && run.style.bold === other.style.bold && run.style.italic === other.style.italic && run.style.underline === other.style.underline; }); }
/** Same instruction content (source and override), ignoring identity and persistence mode. */
function sameInstructionContent(a: InstructionView, b: InstructionView): boolean {
  const content = (v: InstructionView) => JSON.stringify([v.device, v.originalXmlSha256, v.originalPresent, v.originalBytes, v.originalFormat, v.unsupported, v.previewError, v.effectiveDocument, v.overrideDocument]);
  return content(a) === content(b);
}
function Rendered({ document }: { document: Document }) {
  return <div className={styles.preview}>{document.runs.map((run, i) => <span key={i} style={{ fontWeight: run.style.bold ? 700 : 400, fontStyle: run.style.italic ? "italic" : "normal", textDecoration: run.style.underline ? "underline" : "none" }}>{run.text}</span>)}</div>;
}
export function InstructionEditor({ deviceId, context, language = "en", client = defaultClient, onApplied, onDirtyChange }: InstructionEditorProps) {
  const copy = editorCopy[language]; const id = useId(); const readOnly = context === null;
  const [view, setView] = useState<InstructionView | null>(null);
  const [draft, setDraft] = useState<Document>(literal("")); const draftRef = useRef(draft);
  const [plan, setPlan] = useState<InstructionPreview | null>(null);
  const planRef = useRef<{ device: number; token: string } | null>(null);
  const [conversion, setConversion] = useState(false); const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null); const [conflict, setConflict] = useState(false);
  const [reload, setReload] = useState(0); const dirty = useRef(false); const [rebasing, setRebasing] = useState(false);
  const generation = useRef(0); const selection = useRef<[number, number]>([0, 0]);
  const textarea = useRef<HTMLTextAreaElement>(null);
  const priorIdentity = useRef(""); const callbacks = useRef({ onApplied, onDirtyChange });
  // The last admitted view: a dirty draft is based on its content.
  const base = useRef<InstructionView | null>(null);
  callbacks.current = { onApplied, onDirtyChange };
  function markDirty(value: boolean) { dirty.current = value; callbacks.current.onDirtyChange?.(value); }
  function revoke() {
    const previous = planRef.current; planRef.current = null; setPlan(null); setConversion(false);
    if (previous) void client.cancel(previous.device, previous.token).catch(() => undefined);
  }
  useEffect(() => {
    const active = ++generation.current; const controller = new AbortController();
    const identity = `${context?.serverIncarnation ?? ""}:${context?.projectIncarnation ?? ""}:${deviceId}`;
    const keepDraft = priorIdentity.current === identity && dirty.current;
    priorIdentity.current = identity;
    // A dirty draft survives a revision change (Save bumps the revision too):
    // keep it editable while re-reading, and move it onto the new revision only
    // if these instructions are unchanged on the server; otherwise conflict.
    const rebasing = keepDraft && base.current !== null;
    setError(null); setBusy(!rebasing); setRebasing(rebasing); revoke(); setConflict(keepDraft && !rebasing);
    if (!rebasing) setView(null);
    if (!keepDraft) { base.current = null; const empty = literal(""); draftRef.current = empty; setDraft(empty); markDirty(false); }
    void client.read(deviceId, controller.signal).then(value => {
      if (generation.current !== active || controller.signal.aborted) return;
      if ((context !== null && !sameContext(value, context)) || value.device !== deviceId) { setConflict(true); return; }
      if (keepDraft && (base.current === null || !sameInstructionContent(base.current, value))) { setConflict(true); return; }
      base.current = value; setView(value);
      if (!keepDraft) { const initial = value.effectiveDocument ?? literal(""); draftRef.current = initial; setDraft(initial); }
    }).catch(reason => {
      if (generation.current === active && !controller.signal.aborted) setError(reason instanceof Error ? reason.message : copy.invalid);
    }).finally(() => { if (generation.current === active) { setBusy(false); setRebasing(false); } });
    return () => {
      controller.abort(); generation.current++;
      const previous = planRef.current; planRef.current = null;
      if (previous) void client.cancel(previous.device, previous.token).catch(() => undefined);
    };
  // Primitive identity fields prevent unrelated host rerenders from discarding drafts.
  }, [deviceId, context?.serverIncarnation, context?.projectIncarnation, context?.snapshotRevision, client, reload]);
  function change(next: Document) {
    if (readOnly) return;
    revoke(); draftRef.current = next; setDraft(next); setError(null);
    markDirty(!equalDocument(next, view?.effectiveDocument ?? literal("")));
  }
  function format(flag: keyof Style | "clear") {
    try {
      const [start, end] = selection.current; let next = draftRef.current;
      if (flag === "clear") for (const name of ["bold", "italic", "underline"] as const) next = formatSelection(next, start, end, name, false);
      else next = formatSelection(next, start, end, flag, true);
      change(next); textarea.current?.focus(); textarea.current?.setSelectionRange(start, end);
    } catch { setError(copy.invalid); }
  }
  async function makePreview(document: Document | null) {
    if (!view || conflict || busy || rebasing || readOnly) return;
    revoke(); setBusy(true); setError(null); const active = generation.current; const controller = new AbortController();
    try {
      const value = await client.preview(deviceId, { serverIncarnation: view.serverIncarnation, projectIncarnation: view.projectIncarnation, snapshotRevision: view.snapshotRevision }, document, controller.signal);
      if (generation.current !== active) { void client.cancel(deviceId, value.token).catch(() => undefined); return; }
      planRef.current = { device: deviceId, token: value.token }; setPlan(value);
    } catch (reason) { if (generation.current === active) setError(reason instanceof Error ? reason.message : copy.invalid); }
    finally { if (generation.current === active) setBusy(false); }
  }
  async function apply() {
    if (!plan || busy || rebasing || conflict || readOnly || (plan.conversionRequired && !conversion)) return;
    setBusy(true); setError(null); const active = generation.current; const controller = new AbortController();
    try {
      const outcome = await client.apply(deviceId, plan.token, conversion, controller.signal);
      if (generation.current !== active) return;
      planRef.current = null; setPlan(null); markDirty(false);
      callbacks.current.onApplied?.(outcome);
    } catch (reason) { if (generation.current === active) setError(reason instanceof Error ? reason.message : copy.invalid); }
    finally { if (generation.current === active) setBusy(false); }
  }
  return <section className={styles.panel} aria-labelledby={`${id}-title`} aria-busy={busy}>
    <header className={styles.header}><h3 id={`${id}-title`}>{copy.title}</h3>{view && <span className={styles.provenance}>{view.overrideDocument !== null ? copy.edited : copy.original}</span>}</header>
    {busy && <div role="status">{view ? copy.pending : copy.loading}</div>}
    {view && <div className={styles.provenance}>{view.persistence === "session-only" ? copy.session : copy.native}</div>}
    {view && (view.unsupported.length > 0 || view.previewError) && <aside className={styles.warning}>{view.previewError ? copy.unreadable : copy.subset}<details><summary>{view.unsupported.length ? `${view.unsupported.length} ${copy.unsupported}` : view.previewError}</summary>{view.unsupported.map((word, index) => <div key={index}>{word.slice(0, 120)}{word.length > 120 ? "…" : ""}</div>)}</details></aside>}
    {conflict && <aside className={styles.warning} role="alert">{copy.changed}<div><button type="button" onClick={() => { markDirty(false); setConflict(false); setReload(value => value + 1); }}>{copy.reload}</button></div></aside>}
    {readOnly && <div className={styles.provenance} role="note">{copy.readOnly}</div>}
    {error && <div role="alert" className={styles.error}>{error}</div>}
    {(view || conflict) && <>
      <label htmlFor={`${id}-text`}>{copy.text}</label>
      {!readOnly && <div className={styles.toolbar} role="toolbar" aria-label={copy.rendered}>{(["bold", "italic", "underline", "clear"] as const).map(flag => <button key={flag} type="button" disabled={busy || conflict} onMouseDown={event => event.preventDefault()} onClick={() => format(flag)}>{copy[flag]}</button>)}</div>}
      <textarea id={`${id}-text`} ref={textarea} className={styles.input} value={displayText(draft)} readOnly={readOnly} disabled={busy || conflict} onChange={() => undefined} onSelect={event => { selection.current = [event.currentTarget.selectionStart, event.currentTarget.selectionEnd]; }} onInput={event => { try { change(replaceText(draftRef.current, event.currentTarget.value)); } catch { setError(copy.invalid); } }}/>
      <h4>{copy.rendered}</h4><Rendered document={draft}/>
      {!readOnly && <div className={styles.actions}><button type="button" disabled={busy || rebasing || conflict || !dirty.current} onClick={() => void makePreview(draft)}>{copy.preview}</button><button type="button" disabled={busy || rebasing || conflict || view?.overrideDocument === null} onClick={() => void makePreview(null)}>{copy.reset}</button></div>}
    </>}
    {plan && <section className={styles.change} aria-label={copy.change}><h4>{copy.change}</h4>{plan.noOp && <p>{copy.noOp}</p>}{plan.replacement ? <Rendered document={plan.replacement}/> : <p>{copy.resetPreview}</p>}{plan.conversionRequired && <label className={styles.confirm}><input type="checkbox" checked={conversion} onChange={event => setConversion(event.currentTarget.checked)}/><span>{copy.conversion}</span></label>}<div className={styles.actions}><button type="button" disabled={busy || conflict || (plan.conversionRequired && !conversion)} onClick={() => void apply()}>{copy.apply}</button><button type="button" disabled={busy} onClick={revoke}>{copy.cancel}</button></div></section>}
  </section>;
}
