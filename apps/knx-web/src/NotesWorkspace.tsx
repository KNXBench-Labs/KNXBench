/** Notes workspace: mounts the project notes editor against the open project. */
// Host side of docs/contracts/project-note-editor-host.md. The editor owns
// draft, preview and apply; this host hands it the tree's real snapshot
// identity (or nothing at all), the stable-ID target catalogue, the UI
// language, and a refresh through the workbench's own tree publication
// after a confirmed Apply. It stays mounted while hidden, so moving to
// another workbench view never drops a draft.
import { useCallback, useEffect, useMemo, useRef } from "react";
import * as api from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { NotesClient } from "./features/notes/client";
import { ProjectNotesEditor } from "./features/notes/ProjectNotesEditor";
import { useTranslate } from "./i18n";
import { noteTargetChoices, notesContextFromTree, type NoteTargetKindLabels } from "./notesHost";
import { useUiLanguage } from "./uiLanguage";

export interface NotesWorkspaceProps {
  tree: ProjectTree;
  active: boolean;
  /** The workbench's own snapshot publication (it refuses stale trees). */
  onTreeUpdate: (tree: ProjectTree) => void | Promise<void>;
  onDirtyChange: (dirty: boolean) => void;
  /** Test seam; production uses the editor's authenticated HTTP client. */
  client?: NotesClient;
}

export default function NotesWorkspace({ tree, active, onTreeUpdate, onDirtyChange, client }: NotesWorkspaceProps) {
  const t = useTranslate();
  const [uiLanguage] = useUiLanguage();
  const context = notesContextFromTree(tree);
  const labels = useMemo<NoteTargetKindLabels>(() => ({
    installation: t("notes.target.installation"),
    area: t("notes.target.area"),
    line: t("notes.target.line"),
    buildingPart: t("notes.target.buildingPart"),
    device: t("notes.target.device"),
    groupRange: t("notes.target.groupRange"),
    groupAddress: t("notes.target.groupAddress"),
  }), [t]);
  const choices = useMemo(() => noteTargetChoices(tree, labels), [tree, labels]);
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

  return (
    <section className="notes-workspace" hidden={!active} aria-label={t("workbench.notes")}>
      {context
        ? <ProjectNotesEditor context={context} language={uiLanguage === "de" ? "de" : "en"}
            targetChoices={choices} client={client} onApplied={refresh} onDirtyChange={onDirtyChange} />
        : <p role="status">{t("notes.identityUnavailable")}</p>}
    </section>
  );
}
