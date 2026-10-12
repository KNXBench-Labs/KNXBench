/** The shared "unsaved changes" decision: keep editing, discard, or save first. */
// One component for every place that would otherwise replace or end a
// modified project (File › Quit, File › Open/Import). The three answers
// carry explicit visual roles instead of inheriting the application-wide
// accent fill: that inheritance is what once painted the destructive
// label in warning ink on an accent background, leaving the middle
// button visually unlabelled.
//
// The component only renders the decision. The protection itself — save
// first and act only on a clean, successful save; a withdrawn decision
// never acts later — stays with the caller, which owns the project.
import type { ReactNode } from "react";
import Overlay from "./Overlay";
import "./UnsavedChangesDialog.css";

export interface UnsavedChangesDialogProps {
  /** Unique heading id: the dialog is labelled by its own title. */
  titleId: string;
  /** Variant marker kept for callers/tests, e.g. `replace-confirm`. */
  variantClassName?: string;
  title: string;
  message: string;
  /** Muted supporting lines: the next file, or how "save first" behaves. */
  details?: ReactNode[];
  keepLabel: string;
  discardLabel: string;
  saveLabel: string;
  savingLabel: string;
  saving: boolean;
  onKeep: () => void;
  onDiscard: () => void;
  onSave: () => void;
}

export default function UnsavedChangesDialog(props: UnsavedChangesDialogProps) {
  const className = ["quit-confirm", "unsaved-changes-dialog", props.variantClassName].filter(Boolean).join(" ");
  return (
    <Overlay labelledBy={props.titleId} className={className} onClose={props.onKeep}>
      <h2 id={props.titleId}>{props.title}</h2>
      <p>{props.message}</p>
      {props.details?.map((detail, index) => (
        <p key={index} className="quit-confirm-hint">{detail}</p>
      ))}
      {/* "Keep editing" comes first so `Overlay`'s initial focus lands on
          the answer that loses nothing. It stays live during a save: it
          withdraws the pending decision, not the save itself. */}
      <footer className="quit-confirm-footer unsaved-changes-actions">
        <button type="button" className="dialog-action dialog-action--secondary" onClick={props.onKeep}>
          {props.keepLabel}
        </button>
        <button type="button" className="dialog-action dialog-action--danger quit-confirm-discard"
          disabled={props.saving} onClick={props.onDiscard}>
          {props.discardLabel}
        </button>
        <button type="button" className="dialog-action dialog-action--primary primary-action quit-confirm-save"
          disabled={props.saving} onClick={props.onSave}>
          {props.saving ? props.savingLabel : props.saveLabel}
        </button>
      </footer>
    </Overlay>
  );
}
