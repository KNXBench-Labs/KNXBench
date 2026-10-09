/** One guarded name editor shared by device and group-address surfaces. */
import { useContext, createContext, useEffect, useRef, useState } from "react";
import type { ProjectTree } from "./bindings/ProjectTree";
import { useTranslate } from "./i18n";
import { hasRenameContext, nameProblem, renameTarget, type RenameTarget } from "./rename";
import * as api from "./api";

export const RenameScope = createContext(0);
export default function RenameNameField({ tree, target, onApplied, onDone, onPendingChange, autoFocus = false }: {
  tree: ProjectTree; target: RenameTarget; onApplied: (tree: ProjectTree) => void;
  onDone?: () => void; onPendingChange?: (pending: boolean) => void; autoFocus?: boolean;
}) {
  const t = useTranslate();
  const scope = useContext(RenameScope);
  const identity = `${scope}/${tree.server_incarnation}/${tree.project_incarnation}/${target.kind}/${target.id}`;
  const alive = useRef(true);
  const currentIdentity = useRef(identity);
  const generation = useRef(0);
  if (currentIdentity.current !== identity) { generation.current++; currentIdentity.current = identity; }
  const [draft, setDraft] = useState(target.name);
  const [base, setBase] = useState({ tree, name: target.name });
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const pending = useRef(false);
  const cancelled = useRef(false);
  const editing = useRef(false);
  useEffect(() => { alive.current = true; return () => { alive.current = false; }; }, []);
  useEffect(() => {
    editing.current = false; cancelled.current = true; pending.current = false; setBusy(false);
    setDraft(target.name); setBase({ tree, name: target.name }); setError(null);
  }, [identity]);
  useEffect(() => {
    if (!editing.current && !pending.current) {
      setDraft(target.name); setBase({ tree, name: target.name });
    }
  }, [tree, target.name]);
  const eligible = hasRenameContext(tree) && (target.kind === "device" || renameTarget(tree, target.kind, target.id) !== null);
  function cancel() {
    if (pending.current) return;
    cancelled.current = true; editing.current = false;
    setDraft(target.name); setBase({ tree, name: target.name }); setError(null); onDone?.();
  }
  async function apply() {
    if (cancelled.current || pending.current || !eligible) return;
    if (draft === base.name) { editing.current = false; setError(null); onDone?.(); return; }
    const problem = nameProblem(draft);
    if (problem) { setError(t(`rename.${problem}`)); return; }
    const ownedGeneration = generation.current;
    pending.current = true; setBusy(true); onPendingChange?.(true); setError(null);
    try {
      const updated = await api.renameEntity(target.kind, target.id, draft, base.tree, base.name);
      if (!alive.current || generation.current !== ownedGeneration) return;
      editing.current = false;
      setBase({ tree: updated, name: draft });
      onApplied(updated); onDone?.();
    } catch (reason) {
      if (alive.current && generation.current === ownedGeneration) setError(api.errorMessage(reason));
    } finally {
      if (alive.current && generation.current === ownedGeneration) { pending.current = false; setBusy(false); onPendingChange?.(false); }
    }
  }
  async function refresh() {
    if (pending.current) return;
    const ownedGeneration = generation.current;
    pending.current = true; setBusy(true); onPendingChange?.(true);
    try {
      const updated = await api.currentProject();
      if (!alive.current || generation.current !== ownedGeneration) return;
      if (updated.server_incarnation !== base.tree.server_incarnation || updated.project_incarnation !== base.tree.project_incarnation)
        throw new Error(t("rename.contextChanged"));
      let current = renameTarget(updated, target.kind, target.id);
      if (!current && target.kind === "device") {
        // Unplaced instances live in the canonical catalogue read, not placement
        // branches. Its response must match the exact refreshed project snapshot.
        const catalogue = await api.deviceCatalog();
        if (!alive.current || generation.current !== ownedGeneration) return;
        if (catalogue.serverIncarnation !== updated.server_incarnation || catalogue.snapshotRevision !== updated.snapshot_revision)
          throw new Error(t("rename.contextChanged"));
        const rows = catalogue.devices.filter((row) => row.id === target.id);
        if (rows.length === 1) current = { kind: "device", id: target.id, name: rows[0].device.name };
      }
      if (!current) throw new Error(t("rename.contextChanged"));
      // Refresh is never a write: the retained draft requires an explicit new Apply.
      setBase({ tree: updated, name: current.name }); onApplied(updated);
      setError(t("rename.refreshed"));
    } catch (reason) {
      if (alive.current && generation.current === ownedGeneration) setError(api.errorMessage(reason));
    } finally {
      if (alive.current && generation.current === ownedGeneration) { pending.current = false; setBusy(false); onPendingChange?.(false); }
    }
  }
  return <div className="rename-name-field">
    <label className="inspector-field">{t("inspector.name")}
      <input data-rename-name value={draft} disabled={!eligible || busy} autoFocus={autoFocus}
        onFocus={() => { cancelled.current = false; }}
        onPaste={(event) => {
          if (nameProblem(event.clipboardData.getData("text")) === "control") {
            event.preventDefault(); setError(t("rename.control"));
          }
        }}
        onChange={(event) => { editing.current = true; cancelled.current = false; setDraft(event.target.value); }}
        onBlur={() => { if (editing.current && !error) void apply(); }}
        onKeyDown={(event) => {
          if (event.key === "Escape") { event.preventDefault(); event.stopPropagation(); cancel(); }
          if (event.key === "Enter" && !event.nativeEvent.isComposing) {
            event.preventDefault(); event.stopPropagation(); void apply();
          }
        }} />
    </label>
    {!eligible && <p role="status">{t("rename.unavailable")}</p>}
    {error && <p className="field-error" role="alert">{error}</p>}
    {(editing.current || onDone || error) && <div className="rename-actions">
      <button type="button" disabled={busy} onPointerDown={(e) => e.preventDefault()} onClick={() => void apply()}>{t("rename.apply")}</button>
      <button type="button" disabled={busy} onPointerDown={(e) => e.preventDefault()} onClick={cancel}>{t("rename.cancel")}</button>
      {error && <button type="button" disabled={busy} onPointerDown={(e) => e.preventDefault()} onClick={() => void refresh()}>{t("rename.refresh")}</button>}
    </div>}
    {busy && <p role="status">{t("rename.saving")}</p>}
  </div>;
}
