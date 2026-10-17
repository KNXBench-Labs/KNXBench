/** Lets the user decide each stranded note before a delete runs (ADR-0138). */
import { useEffect, useId, useMemo, useRef, useState, type ReactNode } from "react";
import { createPortal } from "react-dom";
import Overlay from "./Overlay";
import * as api from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import { createNoteTargetsReader, noteTargetKey, type NoteTarget, type NoteTargetsReader } from "./features/notes/client";
import { useTranslate } from "./i18n";
import {
  dispositionRequired, initialChoices, isDispositionStale, moveOptions, wireChoices,
  type Deletion, type DispositionRequired, type NoteChoice,
} from "./noteDisposition";
import { noteTargetChoices, type NoteTargetKindLabels } from "./notesHost";
import "./noteDisposition.css";

const defaultTargets = createNoteTargetsReader();

type Failure = { kind: "stale" } | { kind: "refused"; reason: string };

export interface NoteDispositionDialogProps {
  deletion: Deletion;
  required: DispositionRequired;
  onDeleted: (tree: ProjectTree) => void;
  onClose: () => void;
  /** Test seam; production reads `GET /api/project/notes/targets`. */
  targets?: NoteTargetsReader;
}

export default function NoteDispositionDialog({ deletion, required, onDeleted, onClose, targets = defaultTargets }: NoteDispositionDialogProps) {
  const t = useTranslate();
  const base = useId();
  const [choices, setChoices] = useState(() => initialChoices(required.notes));
  const [catalogue, setCatalogue] = useState<"loading" | "failed" | ReturnType<typeof noteTargetChoices>>("loading");
  const [busy, setBusy] = useState(false);
  const [failure, setFailure] = useState<Failure | null>(null);
  const alertRef = useRef<HTMLParagraphElement>(null);
  const firstChoiceRef = useRef<HTMLInputElement>(null);

  const labels = useMemo<NoteTargetKindLabels>(() => ({
    installation: t("notes.target.installation"),
    area: t("notes.target.area"),
    line: t("notes.target.line"),
    buildingPart: t("notes.target.buildingPart"),
    device: t("notes.target.device"),
    comObject: t("notes.target.comObject"),
    groupRange: t("notes.target.groupRange"),
    groupAddress: t("notes.target.groupAddress"),
  }), [t]);

  useEffect(() => {
    const controller = new AbortController();
    targets(controller.signal).then((view) => {
      if (controller.signal.aborted) return;
      // A catalogue from another server or project is not this delete's.
      const same = view.serverIncarnation === required.context.serverIncarnation
        && view.projectIncarnation === required.context.projectIncarnation;
      setCatalogue(same ? noteTargetChoices(view.targets, labels) : "failed");
    }, () => { if (!controller.signal.aborted) setCatalogue("failed"); });
    return () => controller.abort();
  }, [targets, labels, required.context.serverIncarnation, required.context.projectIncarnation]);

  useEffect(() => { if (failure) alertRef.current?.focus(); }, [failure]);

  const known = new Map(Array.isArray(catalogue) ? catalogue.map((choice) => [noteTargetKey(choice.target), choice.label]) : []);
  const targetLabel = (target: NoteTarget) => target.kind === "project"
    ? t("noteDisposition.projectTarget")
    : known.get(noteTargetKey(target)) ?? t("noteDisposition.unknownTarget", { kind: labels[target.kind], id: target.id });
  const options = Array.isArray(catalogue) ? moveOptions(catalogue, required.removedTargets) : [];
  const wire = wireChoices(required.notes, choices);
  const undecidedMove = [...choices.values()].some((choice) => choice.kind === "move" && !choice.target);

  function choose(note: number, choice: NoteChoice) {
    setFailure(null);
    setChoices((previous) => new Map(previous).set(note, choice));
  }

  async function confirm() {
    if (!wire || busy || failure?.kind === "stale") return;
    setBusy(true); setFailure(null);
    try {
      onDeleted(await api.deleteWithNotes(required.context, deletion, wire));
    } catch (e) {
      if (isDispositionStale(e)) setFailure({ kind: "stale" });
      else setFailure({ kind: "refused", reason: api.errorMessage(e) });
    } finally {
      setBusy(false);
    }
  }

  function close() { if (!busy) onClose(); }

  return <Overlay labelledBy={`${base}-title`} className="note-disposition-panel" initialFocusRef={firstChoiceRef} onClose={close}>
    <h2 id={`${base}-title`}>{t("noteDisposition.title")}</h2>
    <p>{t("noteDisposition.intro", { count: required.notes.length })}</p>
    <ul className="note-disposition-notes" data-note-disposition>
      {required.notes.map((note, index) => {
        const choice = choices.get(note.id) ?? { kind: "project" };
        const name = `${base}-note-${note.id}`;
        return <li key={note.id} data-note-disposition-note={note.id}>
          <fieldset disabled={busy}>
            <legend><strong className="note-disposition-title">{note.title}</strong>{" "}
              <span className="note-disposition-target">{t("noteDisposition.on", { target: targetLabel(note.target) })}</span></legend>
            <label><input ref={index === 0 ? firstChoiceRef : undefined} type="radio" name={name} value="project"
              checked={choice.kind === "project"} onChange={() => choose(note.id, { kind: "project" })} />{t("noteDisposition.keep")}</label>
            <div className="note-disposition-move">
              <label><input type="radio" name={name} value="move" checked={choice.kind === "move"}
                onChange={() => choose(note.id, { kind: "move", target: null })} />{t("noteDisposition.move")}</label>
              <select aria-label={t("noteDisposition.moveTarget", { title: note.title })} disabled={busy || options.length === 0}
                value={choice.kind === "move" && choice.target ? noteTargetKey(choice.target) : ""}
                onChange={(event) => {
                  const picked = options.find((option) => noteTargetKey(option.target) === event.target.value);
                  choose(note.id, { kind: "move", target: picked?.target ?? null });
                }}>
                <option value="">{t("noteDisposition.moveNone")}</option>
                {options.map((option) => <option key={noteTargetKey(option.target)} value={noteTargetKey(option.target)}>{option.label}</option>)}
              </select>
            </div>
            <label><input type="radio" name={name} value="delete" checked={choice.kind === "delete"}
              onChange={() => choose(note.id, { kind: "delete" })} />{t("noteDisposition.delete")}</label>
          </fieldset>
        </li>;
      })}
    </ul>
    {catalogue === "loading" && <p role="status"><span className="task-status-text">{t("noteDisposition.targetsLoading")}</span></p>}
    {catalogue === "failed" && <p className="note-disposition-muted">{t("noteDisposition.targetsFailed")}</p>}
    {undecidedMove && <p className="note-disposition-muted" data-note-disposition-incomplete>{t("noteDisposition.incomplete")}</p>}
    <p className="note-disposition-muted">{t("noteDisposition.undo")}</p>
    {busy && <p role="status"><span className="task-status-text">{t("noteDisposition.busy")}</span></p>}
    {failure && <p ref={alertRef} tabIndex={-1} role="alert">
      {failure.kind === "stale" ? t("noteDisposition.stale") : t("noteDisposition.refused", { reason: failure.reason })}</p>}
    <div className="note-disposition-actions">
      <button type="button" data-note-disposition-confirm disabled={!wire || busy || failure?.kind === "stale"} onClick={() => void confirm()}>
        {t("noteDisposition.confirm")}</button>
      <button type="button" disabled={busy} onClick={close}>{t("noteDisposition.cancel")}</button>
    </div>
  </Overlay>;
}

/**
 * Runs a delete; when the server answers that notes would be stranded, opens
 * the decision dialog instead of showing an error. `onDeleted` receives the
 * new tree from either path; other failures go to `onError` unchanged.
 */
export function useNoteAwareDelete(onDeleted: (tree: ProjectTree) => void, targets?: NoteTargetsReader): {
  run: (deletion: Deletion, action: () => Promise<ProjectTree>, onError: (message: string) => void) => Promise<void>;
  dialog: ReactNode;
} {
  const [pending, setPending] = useState<{ deletion: Deletion; required: DispositionRequired } | null>(null);
  async function run(deletion: Deletion, action: () => Promise<ProjectTree>, onError: (message: string) => void) {
    try {
      onDeleted(await action());
    } catch (e) {
      const required = dispositionRequired(e);
      if (required) setPending({ deletion, required });
      else onError(api.errorMessage(e));
    }
  }
  const dialog = pending ? createPortal(<NoteDispositionDialog deletion={pending.deletion} required={pending.required} targets={targets}
    onDeleted={(tree) => { setPending(null); onDeleted(tree); }} onClose={() => setPending(null)} />, document.body) : null;
  return { run, dialog };
}
