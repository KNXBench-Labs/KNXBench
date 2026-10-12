/** Presents explicit parameter drafts without selecting codecs or issuing implicit writes. */
import { useEffect, useId, useRef, useState } from "react";
import { chooseParameterEditor, parameterDraftIssue, type ParameterEditorContext, type ParameterEditorField, type ParameterEditorSource } from "./plan";
import { parameterEditorCopy } from "./copy";
import styles from "./editor.module.css";

export interface ParameterEditorProps<T> {
  field: ParameterEditorField;
  source: ParameterEditorSource | null;
  context: ParameterEditorContext;
  /** Exact project/device/module identity; never a human label. */
  selectionKey: string;
  /** Host-owned server/project/snapshot identity, not an optimistic counter. */
  revision: string;
  label: string;
  locale?: "en" | "de";
  onApply: (writeEtsId: string, value: string) => Promise<T>;
  /** Publish the service's actual panel/tree, not a local reconstruction. */
  onApplied: (result: T) => void;
}

export function ParameterEditor<T>(props: ParameterEditorProps<T>) {
  const identity = JSON.stringify([props.selectionKey, props.field.etsId, props.context.programId, props.context.sourceSha256, props.context.moduleScoped]);
  return <ParameterDraft key={identity} {...props} />;
}

function ParameterDraft<T>(props: ParameterEditorProps<T>) {
  const { field, revision, onApply, onApplied } = props;
  const copy = parameterEditorCopy[props.locale ?? "en"];
  const plan = chooseParameterEditor(field, props.source, props.context);
  const [draft, setDraft] = useState(field.value ?? "");
  const [baseline, setBaseline] = useState(field.value);
  const [loadedRevision, setLoadedRevision] = useState(revision);
  const [pending, setPending] = useState(false);
  const [accepted, setAccepted] = useState(false);
  const [failure, setFailure] = useState<string | null>(null);
  const mounted = useRef(true);
  const latestRevision = useRef(revision);
  const inFlight = useRef(false);
  const id = useId();
  // Saving the project bumps the revision without touching this field. A
  // revision change with an identical field and no write in flight therefore
  // moves the draft onto the new revision; any field change still blocks Apply.
  const fieldSignature = JSON.stringify([field.kind, field.value, field.min, field.max, field.editable, field.writeEtsId, field.access]);
  // When the authoritative value equals the draft (typically the refresh after
  // our own Apply), nothing can be lost: the editor settles onto it. A clean
  // editor follows the project. A revision that arrived while a write was in
  // flight is never adopted silently.
  const loadedSignature = useRef(fieldSignature);
  const blockedRevision = useRef<string | null>(null);
  const [settled, setSettled] = useState(false);
  useEffect(() => {
    latestRevision.current = revision;
    if (inFlight.current) { if (revision !== loadedRevision) blockedRevision.current = revision; return; }
    if (revision === blockedRevision.current) return;
    if (field.value !== null && draft === field.value && (baseline !== field.value || revision !== loadedRevision)) {
      setBaseline(field.value); setLoadedRevision(revision); loadedSignature.current = fieldSignature;
      if (accepted) { setAccepted(false); setSettled(true); }
      return;
    }
    // Without a draft there is nothing to protect: follow the project (Undo/Redo, another client).
    if (!accepted && draft === (baseline ?? "") && (baseline !== field.value || revision !== loadedRevision)) {
      setDraft(field.value ?? ""); setBaseline(field.value); setLoadedRevision(revision); loadedSignature.current = fieldSignature;
      setSettled(false); setFailure(null);
      return;
    }
    if (revision !== loadedRevision && !accepted && fieldSignature === loadedSignature.current) setLoadedRevision(revision);
  }, [revision, loadedRevision, accepted, fieldSignature, draft, baseline, field.value]);
  useEffect(() => { mounted.current = true; return () => { mounted.current = false; }; }, []);
  const stale = loadedRevision !== revision || baseline !== field.value;
  const restricted = !field.editable || field.writeEtsId === null || field.access === "Read" || field.access === "None";
  const issue = parameterDraftIssue(plan, draft);
  const unchanged = draft === (baseline ?? "");

  async function apply() {
    if (inFlight.current || stale || accepted || restricted || unchanged || issue !== null || field.writeEtsId === null) return;
    inFlight.current = true; setPending(true); setFailure(null);
    const requestRevision = revision;
    try {
      const result = await onApply(field.writeEtsId, draft);
      if (!mounted.current || latestRevision.current !== requestRevision) return;
      setAccepted(true);
      onApplied(result);
    } catch (error) {
      if (mounted.current && latestRevision.current === requestRevision) setFailure(error instanceof Error ? error.message : copy.failed);
    } finally {
      inFlight.current = false;
      if (mounted.current) setPending(false);
    }
  }

  if (plan.kind === "fallback") {
    return <section className={styles.panel} aria-label={props.label}>
      <h4>{props.label}</h4><p role="status" className={styles.notice}>{copy.fallback[plan.reason]}</p>
      <div className={styles.raw} aria-label={copy.original}>{field.value === null ? copy.absent : field.value}</div>
    </section>;
  }
  return <section className={styles.panel} aria-label={props.label}>
    <label htmlFor={id} className={styles.label}>{props.label}</label>
    {plan.kind === "time-count" && <p id={`${id}-count`} className={styles.caption}>{copy.count} <span>{plan.unit ?? copy.unspecified}</span></p>}
    <input id={id} type="text" inputMode="numeric" className={styles.input} value={draft}
      disabled={restricted || pending || accepted} aria-invalid={!unchanged && issue !== null}
      aria-describedby={`${id}-status`} onChange={event => { setDraft(event.target.value); setFailure(null); setSettled(false); }} />
    {plan.kind === "integer-slider" && <input type="range" className={styles.slider} aria-label={`${props.label}: ${copy.slider}`}
      min={plan.min} max={plan.max} step={plan.step} value={issue === null ? Number(draft) : Number(field.value)}
      disabled={restricted || pending || accepted || stale || issue !== null}
      onChange={event => { setDraft(event.target.value); setFailure(null); setSettled(false); }} />}
    <div id={`${id}-status`} className={styles.messages}>
      {restricted && <p className={styles.notice}>{copy.restricted}</p>}
      {stale && <p role="status" className={styles.notice}>{copy.stale}</p>}
      {!unchanged && issue !== null && <p role="alert" className={styles.error}>{copy[issue]}</p>}
      {failure !== null && <p role="alert" className={styles.error}>{failure}</p>}
      {accepted && <p role="status">{copy.applied}</p>}
      {settled && <p role="status">{copy.settled}</p>}
    </div>
    <div className={styles.actions}>
      <button type="button" disabled={pending || accepted || stale || restricted || unchanged || issue !== null} onClick={() => { void apply(); }}>{pending ? copy.busy : copy.apply}</button>
      <button type="button" disabled={pending} onClick={() => { setDraft(field.value ?? ""); setBaseline(field.value); setLoadedRevision(revision); loadedSignature.current = fieldSignature; blockedRevision.current = null; setFailure(null); setAccepted(false); setSettled(false); }}>{copy.reset}</button>
    </div>
  </section>;
}
