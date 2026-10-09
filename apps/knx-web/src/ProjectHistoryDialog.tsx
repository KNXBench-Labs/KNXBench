/** Project history: durable undo disclosure, named versions and guarded time travel. */
import { useEffect, useRef, useState } from "react";
import type { ProjectTree } from "./bindings/ProjectTree";
import * as api from "./api";
import { useTranslate } from "./i18n";
import { useUiLanguage } from "./uiLanguage";
import Overlay from "./Overlay";
import "./projectHistory.css";
import { filterProjectVersions, validVersionLabel, type ProjectHistory, type ProjectVersion } from "./projectHistory";

type Confirmation = { kind: "restore" | "delete" | "clear"; view: ProjectHistory; version?: ProjectVersion };
const mib = (bytes: number) => (bytes / (1024 * 1024)).toFixed(1);

export default function ProjectHistoryDialog(props: {
  tree: ProjectTree; onTreeUpdate: (tree: ProjectTree, restored: boolean) => void; onClose: () => void;
}) {
  const t = useTranslate();
  const [language] = useUiLanguage();
  const [view, setView] = useState<ProjectHistory | null>(null);
  const viewRef = useRef<ProjectHistory | null>(null);
  const [label, setLabel] = useState("");
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [confirmation, setConfirmation] = useState<Confirmation | null>(null);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState(false);
  const alive = useRef(false);
  const sequence = useRef(0);
  const mutationPending = useRef(false);
  const cancelRef = useRef<HTMLButtonElement | null>(null);
  const treeRef = useRef(props.tree);
  treeRef.current = props.tree;

  function accept(next: ProjectHistory) {
    viewRef.current = next;
    setView(next);
    setSelectedId((previous) => next.versions.some((v) => v.id === previous) ? previous : next.versions[0]?.id ?? null);
  }
  async function refresh() {
    const request = ++sequence.current;
    setLoading(true); setError(null); setConfirmation(null);
    try {
      const next = await api.projectHistory();
      if (!alive.current || request !== sequence.current) return;
      if (treeRef.current.server_incarnation && next.serverIncarnation !== treeRef.current.server_incarnation) {
        throw new Error("Project history belongs to another server lifetime; reopen the project.");
      }
      accept(next);
      if (next.snapshotRevision > (treeRef.current.snapshot_revision ?? 0)) props.onTreeUpdate(next.project, false);
    } catch (failure) {
      if (alive.current && request === sequence.current) setError(String(failure instanceof Error ? failure.message : failure));
    } finally {
      if (alive.current && request === sequence.current) setLoading(false);
    }
  }
  useEffect(() => {
    alive.current = true;
    void refresh();
    return () => { alive.current = false; ++sequence.current; };
  }, []);
  useEffect(() => {
    const current = viewRef.current;
    if (current && (props.tree.server_incarnation !== current.serverIncarnation
      || (props.tree.snapshot_revision ?? 0) > current.snapshotRevision)) {
      setNotice(true); setConfirmation(null);
      if (!mutationPending.current) void refresh();
    }
  }, [props.tree.server_incarnation, props.tree.snapshot_revision]);
  useEffect(() => { if (confirmation) cancelRef.current?.focus(); }, [confirmation]);

  async function action(kind: "create" | "restore" | "delete" | "clear", reviewed: ProjectHistory, id?: number) {
    const request = ++sequence.current;
    mutationPending.current = true;
    let succeeded = false;
    setBusy(true); setError(null);
    try {
      const next = kind === "create" ? await api.createProjectVersion(reviewed, label)
        : kind === "restore" ? await api.restoreProjectVersion(reviewed, id!, true)
        : kind === "delete" ? await api.deleteProjectVersion(reviewed, id!, true)
        : await api.clearProjectUndo(reviewed, true);
      if (!alive.current || request !== sequence.current) return;
      if (treeRef.current.server_incarnation && next.serverIncarnation !== treeRef.current.server_incarnation) {
        throw new Error("Project history belongs to another server lifetime; reopen the project.");
      }
      accept(next); setConfirmation(null); setNotice(false);
      if (kind === "create") setLabel("");
      props.onTreeUpdate(next.project, kind === "restore");
      succeeded = true;
    } catch (failure) {
      if (alive.current && request === sequence.current) {
        setConfirmation(null);
        setError(String(failure instanceof Error ? failure.message : failure));
      }
    } finally {
      mutationPending.current = false;
      if (alive.current) {
        setBusy(false);
        if (succeeded && kind !== "restore" && viewRef.current
          && (treeRef.current.snapshot_revision ?? 0) > viewRef.current.snapshotRevision) void refresh();
      }
    }
  }

  const selected = view?.versions.find((v) => v.id === selectedId);
  const disabled = loading || busy || error !== null;
  const displayLabel = (version: ProjectVersion) => version.reason === "named" ? version.label : t(`projectHistory.reason.${version.reason}`);
  const filtered = view ? filterProjectVersions(view.versions.map((v) => ({ ...v, label: displayLabel(v) })), query) : [];
  const nearingLimit = view && (view.totalBytes >= view.limits.maxHistoryBytes * 0.9
    || view.versions.length >= view.limits.maxVersions * 0.9 || view.undoSteps + view.redoSteps >= view.limits.maxStackStates * 0.9);

  return <Overlay className="project-history-panel" labelledBy="project-history-title" onClose={busy ? () => {} : props.onClose}>
    <header className="project-history-heading"><div><h2 id="project-history-title">{t("projectHistory.title")}</h2>
      <p>{t("projectHistory.subtitle")}</p></div><button type="button" onClick={props.onClose} disabled={busy}>{t("projectHistory.close")}</button></header>
    {notice && <p className="project-history-notice" role="status">{t("projectHistory.stale")}</p>}
    {error !== null && <div className="project-history-error" role="alert"><strong>{t("projectHistory.failed")}</strong>
      <p>{error}</p><small>{t("toast.error.messageIsEnglish")}</small></div>}
    {loading && <p role="status">{t("projectHistory.loading")}</p>}
    {busy && <p role="status" data-project-history-busy>{t("projectHistory.busy")}</p>}
    {confirmation ? <section className="project-history-confirmation" aria-labelledby="project-history-confirm-title">
      <h3 id="project-history-confirm-title">{t(confirmation.kind === "restore" ? "projectHistory.restoreTitle" : confirmation.kind === "delete" ? "projectHistory.deleteTitle" : "projectHistory.clearTitle")}</h3>
      <p>{t(confirmation.kind === "restore" ? "projectHistory.restoreWarning" : confirmation.kind === "delete" ? "projectHistory.deleteWarning" : "projectHistory.clearWarning", { name: confirmation.version ? displayLabel(confirmation.version) : "" })}</p>
      <div className="project-history-actions"><button ref={cancelRef} type="button" disabled={busy} onClick={() => setConfirmation(null)}>{t("projectHistory.cancel")}</button>
        <button type="button" className="project-history-danger" disabled={disabled} onClick={() => void action(confirmation.kind, confirmation.view, confirmation.version?.id)}>
          {t(confirmation.kind === "restore" ? "projectHistory.confirmRestore" : confirmation.kind === "delete" ? "projectHistory.confirmDelete" : "projectHistory.confirmClear")}</button></div>
    </section> : view && <>
      <p className={`project-history-persistence ${view.persistence === "native" ? "native" : "session"}`}>{t(view.persistence === "native" ? "projectHistory.native" : "projectHistory.session")}</p>
      <dl className="project-history-stats">
        <div data-history-stat="undo"><dt>{t("projectHistory.undo")}</dt><dd>{view.undoSteps}</dd></div>
        <div data-history-stat="redo"><dt>{t("projectHistory.redo")}</dt><dd>{view.redoSteps}</dd></div>
        <div><dt>{t("projectHistory.versions")}</dt><dd>{view.versions.length}</dd></div>
        <div><dt>{t("projectHistory.storage")}</dt><dd>{mib(view.totalBytes)} MiB</dd></div>
      </dl>
      <form className="project-history-create" onSubmit={(event) => { event.preventDefault(); if (!disabled && view.persistence === "native" && validVersionLabel(label) && view.versions.length < view.limits.maxVersions) void action("create", view); }}>
        <label htmlFor="project-version-label">{t("projectHistory.label")}</label>
        <div><input id="project-version-label" value={label} onInput={(event) => setLabel(event.currentTarget.value)} disabled={disabled || view.persistence !== "native"} aria-describedby="project-version-label-hint" />
          <button type="submit" disabled={disabled || view.persistence !== "native" || !validVersionLabel(label) || view.versions.length >= view.limits.maxVersions}>{t("projectHistory.create")}</button></div>
        <small id="project-version-label-hint">{t("projectHistory.labelHint")}</small>
      </form>
      <label className="project-history-search">{t("projectHistory.search")}<input type="search" value={query} disabled={disabled} onInput={(event) => { setQuery(event.currentTarget.value); setSelectedId(null); }} /></label>
      <ul className="project-history-versions" aria-label={t("projectHistory.versions")}>{filtered.map((version) => <li key={version.id}>
        <button type="button" aria-pressed={selectedId === version.id} disabled={disabled} onClick={() => setSelectedId(version.id)}>
          <strong>{version.label}</strong><span>{t(`projectHistory.reason.${version.reason}`)}</span>
          <time dateTime={version.createdAt} title={version.createdAt}>{new Intl.DateTimeFormat(language === "de" ? "de" : "en", { dateStyle: "medium", timeStyle: "short" }).format(new Date(version.createdAt))}</time>
          <small>{mib(version.bytes)} MiB · #{version.id}</small></button></li>)}</ul>
      {!filtered.length && !error && <p className="project-history-empty">{t(view.versions.length ? "projectHistory.noMatches" : "projectHistory.empty")}</p>}
      {selected && <p className="project-history-selection">{t("projectHistory.selected", { name: displayLabel(selected) })}</p>}
      <div className="project-history-actions">
        <button type="button" disabled={disabled || !selected || view.persistence !== "native"} onClick={() => setConfirmation({ kind: "restore", view, version: selected })}>{t("projectHistory.restore")}</button>
        <button type="button" disabled={disabled || !selected || view.persistence !== "native"} onClick={() => setConfirmation({ kind: "delete", view, version: selected })}>{t("projectHistory.delete")}</button>
        <button type="button" disabled={disabled || !view.undoSteps && !view.redoSteps} onClick={() => setConfirmation({ kind: "clear", view })}>{t("projectHistory.clearUndo")}</button>
      </div>
      {nearingLimit && <p className="project-history-notice" role="status">{t("projectHistory.nearLimit")}</p>}
      <p className="project-history-limit-note">{t("projectHistory.limits", { steps: view.limits.maxStackStates, versions: view.limits.maxVersions,
        image: view.limits.maxImageBytes / (1024 * 1024), total: view.limits.maxHistoryBytes / (1024 * 1024) })}</p>
      <p className="project-history-caveat">{t("projectHistory.backupCaveat")}</p>
    </>}
    <footer className="project-history-footer"><button type="button" disabled={busy || loading} onClick={() => void refresh()}>{t("projectHistory.refresh")}</button></footer>
  </Overlay>;
}
