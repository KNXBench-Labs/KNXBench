/** Catalogue backups in the catalogue view: list, restore and delete, each after asking. */
// Installing (with preview) lives in ProductInstallControl/CatalogPackageUpdate
// since Goal B5; this panel keeps what the server backed up. The catalogue view
// is mounted for every session, so backups are read only once their list is
// opened, and re-read after an install, restore or deletion from then on.
// Restore and delete both ask first, with keeping as the default. A deletion
// names the file by the identity the listing gave it, so a file replaced under
// the same name since then is kept and the list is read again.
import { useCallback, useEffect, useRef, useState } from "react";
import * as api from "./api";
import type { CatalogueBackup } from "./api";
import { formatBytes } from "./catalogueUpdate";
import { useTranslate } from "./i18n";
import Overlay from "./Overlay";
import { useUiLanguage } from "./uiLanguage";

type Asking = { action: "restore"; backup: CatalogueBackup } | { action: "delete"; backup: CatalogueBackup };

export default function CatalogUpdatePanel(props: {
  onInstalled: () => void | Promise<void>;
  /** Changes after every install elsewhere in the view, so an open list is read again. */
  installRevision: number;
}) {
  const { onInstalled, installRevision } = props;
  const t = useTranslate();
  const [uiLanguage] = useUiLanguage();
  const [backups, setBackups] = useState<CatalogueBackup[] | null>(null);
  const [backupError, setBackupError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [asking, setAsking] = useState<Asking | null>(null);
  const [working, setWorking] = useState(false);
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

  // An install elsewhere adds a backup; an open list shows it.
  const seenRevision = useRef(installRevision);
  useEffect(() => {
    if (seenRevision.current === installRevision) return;
    seenRevision.current = installRevision;
    void refreshBackups();
  }, [installRevision, refreshBackups]);

  async function restore(name: string) {
    setAsking(null);
    setWorking(true);
    setNotice(null);
    setBackupError(null);
    try {
      await api.restoreCatalogBackup(name);
      setNotice(t("catalogUpdate.restore.done", { name }));
      await onInstalled();
      await refreshBackups();
    } catch (e) {
      setBackupError(t("catalogUpdate.restore.failed", { message: api.errorMessage(e) }));
    } finally {
      setWorking(false);
    }
  }

  async function remove(backup: CatalogueBackup) {
    setAsking(null);
    setWorking(true);
    setNotice(null);
    setBackupError(null);
    try {
      await api.deleteCatalogBackup(backup.name, backup.identity);
      setNotice(t("catalogUpdate.delete.done", { name: backup.name }));
      await refreshBackups();
    } catch (e) {
      if (api.isCatalogueBackupChanged(e)) {
        // Read the list first: a successful read clears older messages.
        await refreshBackups();
        setBackupError(t("catalogUpdate.delete.changed"));
      } else {
        setBackupError(t("catalogUpdate.delete.failed", { message: api.errorMessage(e) }));
      }
    } finally {
      setWorking(false);
    }
  }

  return <section className="catalog-update" aria-label={t("catalogUpdate.region")}>
    <details className="catalog-backups" onToggle={(e) => {
      if (!e.currentTarget.open || backupsOpenedRef.current) return;
      backupsOpenedRef.current = true;
      void loadBackups();
    }}>
      <summary>{backups === null ? t("catalogUpdate.backups.headingUnread")
        : t("catalogUpdate.backups.heading", { count: backups.length })}</summary>
      {notice !== null && <p role="status">{notice}</p>}
      {backupError !== null && <p className="field-error" role="alert">{backupError}</p>}
      {backups !== null && backups.length === 0 && <p className="inspector-description">{t("catalogUpdate.backups.empty")}</p>}
      {backups !== null && backups.length > 0 && <ul className="catalog-backup-list">
        {backups.map((backup) => <li key={backup.name}>
          <code className="catalog-backup-name">{backup.name}</code>
          <span className="catalog-backup-meta">
            {formatBytes(backup.bytes, uiLanguage)} · {new Date(backup.modifiedUnixSeconds * 1000).toLocaleString(uiLanguage)}
          </span>
          <span className="catalog-backup-actions">
            <button type="button" disabled={working} onClick={() => setAsking({ action: "restore", backup })}>{t("catalogUpdate.backups.restore")}</button>
            <button type="button" disabled={working} onClick={() => setAsking({ action: "delete", backup })}>{t("catalogUpdate.backups.delete")}</button>
          </span>
        </li>)}
      </ul>}
    </details>

    {asking?.action === "restore" && <Overlay labelledBy="catalog-restore-title" className="quit-confirm unsaved-changes-dialog catalog-restore-confirm"
      initialFocusRef={keepRef} onClose={() => setAsking(null)}>
      <h2 id="catalog-restore-title">{t("catalogUpdate.restore.title")}</h2>
      <p>{t("catalogUpdate.restore.message", { name: asking.backup.name })}</p>
      <p>{t("catalogUpdate.restore.projects")}</p>
      <footer className="quit-confirm-footer unsaved-changes-actions">
        <button ref={keepRef} type="button" className="dialog-action dialog-action--secondary" onClick={() => setAsking(null)}>
          {t("catalogUpdate.restore.keep")}</button>
        <button type="button" className="dialog-action dialog-action--danger" onClick={() => { void restore(asking.backup.name); }}>
          {t("catalogUpdate.restore.confirm")}</button>
      </footer>
    </Overlay>}

    {asking?.action === "delete" && <Overlay labelledBy="catalog-delete-title" className="quit-confirm unsaved-changes-dialog catalog-restore-confirm"
      initialFocusRef={keepRef} onClose={() => setAsking(null)}>
      <h2 id="catalog-delete-title">{t("catalogUpdate.delete.title")}</h2>
      <p>{t("catalogUpdate.delete.message", { name: asking.backup.name })}</p>
      <footer className="quit-confirm-footer unsaved-changes-actions">
        <button ref={keepRef} type="button" className="dialog-action dialog-action--secondary" onClick={() => setAsking(null)}>
          {t("catalogUpdate.delete.keep")}</button>
        <button type="button" className="dialog-action dialog-action--danger" onClick={() => { void remove(asking.backup); }}>
          {t("catalogUpdate.delete.confirm")}</button>
      </footer>
    </Overlay>}
  </section>;
}
