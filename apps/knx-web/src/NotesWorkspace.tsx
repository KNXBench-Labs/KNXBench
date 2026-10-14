/** Notes workspace: mounts the project notes editor against the open project. */
// Host side of docs/contracts/project-note-editor-host.md. The editor owns
// draft, preview and apply; this host hands it the tree's real snapshot
// identity (or nothing at all), the service's target catalogue with
// translated kind words, the UI language, and a refresh through the
// workbench's own tree publication after a confirmed Apply. It stays mounted
// while hidden, so moving to another workbench view never drops a draft.
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import * as api from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import { createNoteTargetsReader, type NotesClient, type NoteTargetOption, type NoteTargetsReader } from "./features/notes/client";
import { ProjectNotesEditor } from "./features/notes/ProjectNotesEditor";
import { useTranslate } from "./i18n";
import { catalogueForContext, noteTargetChoices, notesContextFromTree, type NoteTargetKindLabels } from "./notesHost";
import { useUiLanguage } from "./uiLanguage";

const defaultTargets = createNoteTargetsReader();

export interface NotesWorkspaceProps {
  tree: ProjectTree;
  active: boolean;
  /** The workbench's own snapshot publication (it refuses stale trees). */
  onTreeUpdate: (tree: ProjectTree) => void | Promise<void>;
  onDirtyChange: (dirty: boolean) => void;
  /** Test seam; production uses the editor's authenticated HTTP client. */
  client?: NotesClient;
  /** Test seam; production reads `GET /api/project/notes/targets`. */
  targets?: NoteTargetsReader;
}

/**
 * `settled` turns true at the first answer (or failure) and stays true, so a
 * reload for a newer revision keeps the editor and the previous choices.
 */
interface TargetState { options: readonly NoteTargetOption[]; settled: boolean; failed: boolean }

export default function NotesWorkspace({ tree, active, onTreeUpdate, onDirtyChange, client, targets = defaultTargets }: NotesWorkspaceProps) {
  const t = useTranslate();
  const [uiLanguage] = useUiLanguage();
  const context = notesContextFromTree(tree);
  const contextRef = useRef(context);
  contextRef.current = context;
  const contextKey = context ? `${context.serverIncarnation}\u0000${context.projectIncarnation}\u0000${context.snapshotRevision}` : null;
  const [targetState, setTargetState] = useState<TargetState>({ options: [], settled: false, failed: false });
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    const current = contextRef.current;
    if (contextKey === null || !current) return;
    const controller = new AbortController();
    targets(controller.signal).then((view) => {
      if (controller.signal.aborted) return;
      const options = catalogueForContext(view, current);
      setTargetState((previous) => options
        ? { options, settled: true, failed: false }
        : { ...previous, settled: true, failed: true });
    }, () => {
      if (controller.signal.aborted) return;
      setTargetState((previous) => ({ ...previous, settled: true, failed: true }));
    });
    return () => controller.abort();
  }, [contextKey, attempt, targets]);

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
  const choices = useMemo(() => noteTargetChoices(targetState.options, labels), [targetState.options, labels]);
  // The draft dies with this instance (project change, close), so the host's
  // draft flag must not outlive it.
  const dirtyReport = useRef(onDirtyChange);
  dirtyReport.current = onDirtyChange;
  useEffect(() => () => dirtyReport.current(false), []);
  // A rejected refresh propagates to the editor, which reports "confirmed,
  // refresh failed" instead of a failed change.
  const refresh = useCallback(async () => {
    await onTreeUpdate(await api.currentProject());
  }, [onTreeUpdate]);

  // The editor waits for the first catalogue answer, so a stored target is
  // never shown as "not in the catalogue" merely because it is still loading.
  let body;
  if (!context) body = <p role="status">{t("notes.identityUnavailable")}</p>;
  else if (!targetState.settled) body = <p role="status"><span className="task-status-text">{t("notes.targetsLoading")}</span></p>;
  else body = <>
    {targetState.failed && <div className="notes-targets-failed" role="alert">
      <p className="field-error">{t("notes.targetsFailed")}</p>
      <button type="button" onClick={() => setAttempt((count) => count + 1)}>{t("notes.targetsRetry")}</button>
    </div>}
    <ProjectNotesEditor context={context} language={uiLanguage === "de" ? "de" : "en"}
      targetChoices={choices} client={client} onApplied={refresh} onDirtyChange={onDirtyChange} />
  </>;
  return (
    <section className="notes-workspace" hidden={!active} aria-label={t("workbench.notes")}>
      {body}
    </section>
  );
}
