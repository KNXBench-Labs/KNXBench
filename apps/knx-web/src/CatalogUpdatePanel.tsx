/** Previewed catalogue update and backup restore in the catalogue view (ADR-0118). */
// Preview first: nothing is installed until "Back up and install", which sends
// the same file again with the preview's token. The server backs up the
// product database before installing; backups can be restored by the names the
// server lists. Restore asks first, with keeping the catalogue as the default.
// The catalogue view is mounted for every session, so backups are read only
// once their list is opened, and re-read after an update or restore from then on.
import { useCallback, useRef, useState } from "react";
import * as api from "./api";
import type { CatalogueBackup, CatalogueUpdateApplied, CatalogueUpdatePreview } from "./api";
import { CatalogInstallReportView } from "./CatalogInstallReport";
import { formatBytes, installedVersionList } from "./catalogueUpdate";
import { useTranslate } from "./i18n";
import Overlay from "./Overlay";
import { useUiLanguage } from "./uiLanguage";

type Phase =
  | { kind: "idle" }
  | { kind: "previewing"; file: File }
  | { kind: "previewed"; file: File; preview: CatalogueUpdatePreview; applying: boolean; error: string | null }
  | { kind: "stale"; file: File }
  | { kind: "applied"; result: CatalogueUpdateApplied }
  | { kind: "failed"; message: string };

export default function CatalogUpdatePanel(props: { onInstalled: () => void | Promise<void> }) {
  const { onInstalled } = props;
  const t = useTranslate();
  const [uiLanguage] = useUiLanguage();
  const [phase, setPhase] = useState<Phase>({ kind: "idle" });
  const applyingRef = useRef(false);
  const [backups, setBackups] = useState<CatalogueBackup[] | null>(null);
  const [backupError, setBackupError] = useState<string | null>(null);
  const [restoreNotice, setRestoreNotice] = useState<string | null>(null);
  const [confirming, setConfirming] = useState<string | null>(null);
  const [restoring, setRestoring] = useState(false);
  const keepRef = useRef<HTMLButtonElement | null>(null);

  const backupsOpenedRef = useRef(false);
  const loadBackups = useCallback(async () => {
    try {
      setBackups((await api.catalogBackups()).backups);
      setBackupError(null);
    } catch (e) {
      setBackupError(t("catalogUpdate.backups.loadFailed", { message: api.errorMessage(e) }));
    }
  }, [t]);
  const refreshBackups = useCallback(async () => {
    if (backupsOpenedRef.current) await loadBackups();
  }, [loadBackups]);

  async function preview(file: File) {
    setPhase({ kind: "previewing", file });
    try {
      const answer = await api.previewCatalogUpdate(file);
      setPhase({ kind: "previewed", file, preview: answer.preview, applying: false, error: null });
    } catch (e) {
      setPhase({ kind: "failed", message: api.errorMessage(e) });
    }
  }

  async function apply() {
    if (phase.kind !== "previewed" || applyingRef.current) return;
    applyingRef.current = true;
    const { file, preview: shown } = phase;
    setPhase({ ...phase, applying: true, error: null });
    try {
      const result = await api.applyCatalogUpdate(file, shown.token);
      setPhase({ kind: "applied", result });
      await onInstalled();
      await refreshBackups();
    } catch (e) {
      if (api.isCatalogueUpdateStale(e)) setPhase({ kind: "stale", file });
      else setPhase({ kind: "previewed", file, preview: shown, applying: false, error: api.errorMessage(e) });
    } finally {
      applyingRef.current = false;
    }
  }

  async function restore(name: string) {
    setConfirming(null);
    setRestoring(true);
    setRestoreNotice(null);
    setBackupError(null);
    try {
      await api.restoreCatalogBackup(name);
      setRestoreNotice(t("catalogUpdate.restore.done", { name }));
      await onInstalled();
      await refreshBackups();
    } catch (e) {
      setBackupError(t("catalogUpdate.restore.failed", { message: api.errorMessage(e) }));
    } finally {
      setRestoring(false);
    }
  }

  const busy = phase.kind === "previewing" || (phase.kind === "previewed" && phase.applying);
  const unknown = t("catalogUpdate.notDeclared");

  return <section className="catalog-update" aria-label={t("catalogUpdate.region")}>
    <label className="catalog-install catalog-update-open">
      {phase.kind === "previewing" ? t("catalogUpdate.previewing") : t("catalogUpdate.open")}
      <input type="file" accept=".knxprod,application/zip" disabled={busy}
        onChange={(e) => {
          const file = e.target.files?.[0];
          if (file) void preview(file);
          e.currentTarget.value = "";
        }} />
    </label>

    {phase.kind === "failed" && <p className="field-error" role="alert">{t("catalogUpdate.error", { message: phase.message })}</p>}

    {phase.kind === "previewed" && <div className="catalog-update-preview" role="region" aria-labelledby="catalog-update-heading">
      <h2 id="catalog-update-heading">{t("catalogUpdate.heading", { name: phase.preview.sourceName })}</h2>
      <p className="catalog-update-notice" role="status">{t("catalogUpdate.nothingYet")}</p>
      <p className="catalog-update-hash">{t("catalogUpdate.package")} <code>{phase.preview.packageSha256}</code></p>
      {phase.preview.packageAlreadyInstalled && <p className="catalog-update-notice">{t("catalogUpdate.already")}</p>}
      <ul className="catalog-update-programs">
        {phase.preview.programs.map((program) => <li key={program.id}>
          <strong>{program.name ?? program.id}</strong>
          <span className={`catalog-update-status catalog-update-status--${program.status}`}>{t(`catalogUpdate.status.${program.status}`)}</span>
          <span className="catalog-update-program-detail">{t("catalogUpdate.program", {
            number: program.applicationNumber ?? unknown, version: program.applicationVersion ?? unknown })}</span>
          {program.name !== null && <code className="catalog-update-program-id">{program.id}</code>}
          {program.installedVersions.length > 0 && <span className="catalog-update-program-detail">
            {t("catalogUpdate.installedVersions", { versions: installedVersionList(program, unknown) })}</span>}
        </li>)}
      </ul>
      <p>{t("catalogUpdate.added", { products: phase.preview.productsAdded, links: phase.preview.hardware2programsAdded })}</p>
      {phase.preview.idConflicts > 0 && <p className="catalog-update-warning">{t("catalogUpdate.conflicts", { count: phase.preview.idConflicts })}</p>}
      <p className="inspector-description">{t("catalogUpdate.bindings")}</p>
      {phase.error !== null && <p className="field-error" role="alert">{t("catalogUpdate.error", { message: phase.error })}</p>}
      <div className="catalog-update-actions">
        <button type="button" disabled={phase.applying} onClick={() => { void apply(); }}>
          {phase.applying ? t("catalogUpdate.applying") : t("catalogUpdate.apply")}</button>
        <button type="button" className="catalog-update-cancel" disabled={phase.applying}
          onClick={() => setPhase({ kind: "idle" })}>{t("catalogUpdate.cancel")}</button>
      </div>
    </div>}

    {phase.kind === "stale" && <div className="catalog-update-preview" role="alert">
      <p>{t("catalogUpdate.stale")}</p>
      <button type="button" onClick={() => { void preview(phase.file); }}>{t("catalogUpdate.previewAgain")}</button>
    </div>}

    {phase.kind === "applied" && <div className="catalog-update-preview">
      <p role="status">{t("catalogUpdate.applied", { backup: phase.result.backup })}</p>
      <CatalogInstallReportView report={phase.result.report} />
    </div>}

    <details className="catalog-backups" onToggle={(e) => {
      if (!e.currentTarget.open || backupsOpenedRef.current) return;
      backupsOpenedRef.current = true;
      void loadBackups();
    }}>
      <summary>{backups === null ? t("catalogUpdate.backups.headingUnread")
        : t("catalogUpdate.backups.heading", { count: backups.length })}</summary>
      {restoreNotice !== null && <p role="status">{restoreNotice}</p>}
      {backupError !== null && <p className="field-error" role="alert">{backupError}</p>}
      {backups !== null && backups.length === 0 && <p className="inspector-description">{t("catalogUpdate.backups.empty")}</p>}
      {backups !== null && backups.length > 0 && <ul className="catalog-backup-list">
        {backups.map((backup) => <li key={backup.name}>
          <code className="catalog-backup-name">{backup.name}</code>
          <span className="catalog-backup-meta">
            {formatBytes(backup.bytes, uiLanguage)} · {new Date(backup.modifiedUnixSeconds * 1000).toLocaleString(uiLanguage)}
          </span>
          <button type="button" disabled={restoring} onClick={() => setConfirming(backup.name)}>{t("catalogUpdate.backups.restore")}</button>
        </li>)}
      </ul>}
    </details>

    {confirming !== null && <Overlay labelledBy="catalog-restore-title" className="quit-confirm unsaved-changes-dialog catalog-restore-confirm"
      initialFocusRef={keepRef} onClose={() => setConfirming(null)}>
      <h2 id="catalog-restore-title">{t("catalogUpdate.restore.title")}</h2>
      <p>{t("catalogUpdate.restore.message", { name: confirming })}</p>
      <p>{t("catalogUpdate.restore.projects")}</p>
      <footer className="quit-confirm-footer unsaved-changes-actions">
        <button ref={keepRef} type="button" className="dialog-action dialog-action--secondary" onClick={() => setConfirming(null)}>
          {t("catalogUpdate.restore.keep")}</button>
        <button type="button" className="dialog-action dialog-action--danger" onClick={() => { void restore(confirming); }}>
          {t("catalogUpdate.restore.confirm")}</button>
      </footer>
    </Overlay>}
  </section>;
}
