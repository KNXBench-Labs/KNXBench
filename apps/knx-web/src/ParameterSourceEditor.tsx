/** Parameter row host for the source-checked editor: read, verify, write, refetch. */
// Mounted only on explicit request for Time/Number/Colour/Picture fields
// (`ParameterPanel.tsx`). The source answer is verified against this exact
// device, reference, panel program and snapshot before it becomes evidence;
// anything else gives the leaf no source, which it shows as a fallback.
// Writes go only through the existing `setParameterValue`, and the host
// publishes the returned panel and tree itself inside `onApply`, so a write
// that finishes after the editor closed, or after another revision arrived,
// is still published (contract item 6). The leaf's `onApplied` is then only a
// notification.
import { useCallback, useEffect, useRef, useState } from "react";
import * as api from "./api";
import type { ModuleScope, ParameterField, ParameterPanel } from "./api";
import type { ProjectTree } from "./bindings/ProjectTree";
import { ParameterEditor } from "./features/parameter-editors/ParameterEditor";
import type { ParameterEditorField } from "./features/parameter-editors/plan";
import { useTranslate } from "./i18n";
import { parameterRevision, parameterSelectionKey, projectParameterSource, type ProjectedSource } from "./parameterSourceHost";
import type { SnapshotContext } from "./snapshotIdentity";
import { isRevisionConflict } from "./revisionConflict";
import { useUiLanguage } from "./uiLanguage";

export interface ParameterSourceEditorProps {
  deviceId: number;
  field: ParameterField;
  scope: ModuleScope | null;
  /** The authoritative parameter panel's program. */
  programId: string;
  snapshot: SnapshotContext;
  /** Snapshot the shown panel was read under; binds the write (Goal A2/B1). */
  writeBinding: SnapshotContext;
  /** Reloads tree and panel after a stale-write refusal; never writes. */
  onConflict: () => Promise<void>;
  /** Product language of the parameter panel, passed through to the write. */
  language: string | null;
  label: string;
  onUpdated: (panel: ParameterPanel) => void;
  onValueApplied: (tree: ProjectTree) => void;
  onClose: () => void;
}

type SourceState =
  | { status: "loading" }
  | { status: "ready"; projected: ProjectedSource }
  | { status: "failed"; message: string };

/** The leaf's field view: the panel's own values, unchanged. */
function editorField(field: ParameterField): ParameterEditorField {
  return { etsId: field.etsId, kind: field.kind, value: field.value, min: field.min, max: field.max,
    editable: field.editable, writeEtsId: field.writeEtsId, access: field.access };
}

export default function ParameterSourceEditor(props: ParameterSourceEditorProps) {
  const { deviceId, field, scope, programId, snapshot, writeBinding, onConflict, language, onUpdated, onValueApplied } = props;
  const t = useTranslate();
  const [uiLanguage] = useUiLanguage();
  const [state, setState] = useState<SourceState>({ status: "loading" });
  const generation = useRef(0);
  const revision = parameterRevision(snapshot);

  useEffect(() => {
    const current = ++generation.current;
    const request = { deviceId, etsId: field.etsId, programId, snapshot };
    // A refetch keeps the previous answer on screen, so a draft is not torn
    // down while the new revision's evidence is read.
    api.parameterEditorSource(deviceId, field.etsId).then(
      (body) => { if (current === generation.current) setState({ status: "ready", projected: projectParameterSource(body, request) }); },
      (error) => { if (current === generation.current) setState({ status: "failed", message: api.errorMessage(error) }); },
    );
    return () => { if (current === generation.current) generation.current++; };
    // `snapshot` is covered by `revision`, its exact string form.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [deviceId, field.etsId, programId, revision]);

  const expectedValue = field.value;
  const onApply = useCallback(async (writeEtsId: string, raw: string) => {
    let panel: ParameterPanel;
    try {
      panel = await api.setParameterValue(deviceId, writeEtsId, raw, language, { ...writeBinding, expectedValue });
    } catch (error) {
      // The leaf keeps its draft and shows this; the reload makes it stale,
      // so only a new explicit Apply on the new value can write again.
      if (!isRevisionConflict(error, "parameterWriteConflict")) throw error;
      try {
        await onConflict();
      } catch (reload) {
        throw new Error(t("parameters.conflictReloadFailed", { message: api.errorMessage(reload) }));
      }
      throw new Error(t("parameters.conflict"));
    }
    onUpdated(panel);
    // The server attaches its rebuilt tree to every successful write; a
    // missing one is a broken contract, not "nothing to publish".
    if (!panel.tree) throw new Error("setParameterValue response carried no tree");
    onValueApplied(panel.tree);
    return panel;
  }, [deviceId, language, writeBinding, expectedValue, onConflict, onUpdated, onValueApplied, t]);
  const onApplied = useCallback(() => {}, []);

  const projected = state.status === "ready" ? state.projected : null;
  const source = projected?.kind === "declared" ? projected.source : null;
  const note = projected === null ? null
    : projected.kind === "unavailable" ? t("parameters.checked.unavailable", { reason: projected.reason })
    : projected.kind === "mismatch" || projected.kind === "malformed" ? t("parameters.checked.mismatch") : null;

  return <div className="parameter-checked-editor">
    {state.status === "loading" && <p className="inspector-description" role="status"><span className="task-status-text">{t("parameters.checked.loading")}</span></p>}
    {state.status === "failed" && <p className="field-error" role="alert">{t("parameters.checked.readFailed", { message: state.message })}</p>}
    {note !== null && <p className="inspector-description" role="status">{note}</p>}
    {projected !== null && <ParameterEditor field={editorField(field)} source={source}
      context={{ programId, sourceSha256: source?.sourceSha256 ?? "", moduleScoped: scope !== null }}
      selectionKey={parameterSelectionKey(snapshot, deviceId, scope)} revision={revision} label={props.label}
      suffix={field.suffixText} locale={uiLanguage === "de" ? "de" : "en"} onApply={onApply} onApplied={onApplied} />}
    <button type="button" className="parameter-checked-close" onClick={props.onClose}>{t("parameters.checked.close")}</button>
  </div>;
}
