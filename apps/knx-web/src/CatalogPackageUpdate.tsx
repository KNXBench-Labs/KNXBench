/** One product package, previewed and installed only after confirmation (ADR-0118, Goal B5). */
// Moved here from CatalogUpdatePanel so every web install takes this path: the
// catalogue view and the device wizard alike. Nothing is installed until
// "Back up and install", which sends the same file again with the preview's
// token; the server backs the product database up first. A file the package
// route recognises as a legacy product database is handed back to the caller.
import { useEffect, useRef, useState } from "react";
import * as api from "./api";
import type { CatalogueUpdateApplied, CatalogueUpdatePreview } from "./api";
import { CatalogInstallReportView } from "./CatalogInstallReport";
import { installedVersionList } from "./catalogueUpdate";
import { useTranslate } from "./i18n";
import { isLegacyProductRefusal } from "./legacyInstall";

type Phase =
  | { kind: "previewing" }
  | { kind: "previewed"; preview: CatalogueUpdatePreview; applying: boolean; error: string | null }
  | { kind: "stale" }
  | { kind: "applied"; result: CatalogueUpdateApplied }
  | { kind: "failed"; message: string };

export default function CatalogPackageUpdate(props: {
  file: File;
  onInstalled: () => void | Promise<void>;
  /** Cancelled: the caller forgets the file. */
  onDone: () => void;
  /** Not a package but a (renamed) legacy product database. */
  onLegacy: (file: File) => void;
  /** Reading or installing: the caller's picker waits. */
  onBusyChange?: (busy: boolean) => void;
}) {
  const { file, onInstalled, onDone, onLegacy, onBusyChange } = props;
  const t = useTranslate();
  const [phase, setPhase] = useState<Phase>({ kind: "previewing" });
  const [attempt, setAttempt] = useState(0);
  const applyingRef = useRef(false);

  useEffect(() => {
    let current = true;
    setPhase({ kind: "previewing" });
    void (async () => {
      try {
        const answer = await api.previewCatalogUpdate(file);
        if (current) setPhase({ kind: "previewed", preview: answer.preview, applying: false, error: null });
      } catch (e) {
        if (!current) return;
        if (isLegacyProductRefusal(e)) onLegacy(file);
        else setPhase({ kind: "failed", message: api.errorMessage(e) });
      }
    })();
    return () => { current = false; };
    // A new file or "Preview again" starts afresh; the callbacks do not.
  }, [file, attempt]);

  const busy = phase.kind === "previewing" || (phase.kind === "previewed" && phase.applying);
  useEffect(() => { onBusyChange?.(busy); }, [busy, onBusyChange]);
  useEffect(() => () => onBusyChange?.(false), [onBusyChange]);

  async function apply() {
    if (phase.kind !== "previewed" || applyingRef.current) return;
    applyingRef.current = true;
    const shown = phase.preview;
    setPhase({ ...phase, applying: true, error: null });
    try {
      const result = await api.applyCatalogUpdate(file, shown.token);
      setPhase({ kind: "applied", result });
      await onInstalled();
    } catch (e) {
      if (api.isCatalogueUpdateStale(e)) setPhase({ kind: "stale" });
      else setPhase({ kind: "previewed", preview: shown, applying: false, error: api.errorMessage(e) });
    } finally {
      applyingRef.current = false;
    }
  }

  const unknown = t("catalogUpdate.notDeclared");
  return <>
    {phase.kind === "previewing" && <p className="catalog-update-notice" role="status">{t("catalogUpdate.previewing")}</p>}

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
          onClick={onDone}>{t("catalogUpdate.cancel")}</button>
      </div>
    </div>}

    {phase.kind === "stale" && <div className="catalog-update-preview" role="alert">
      <p>{t("catalogUpdate.stale")}</p>
      <button type="button" onClick={() => setAttempt((n) => n + 1)}>{t("catalogUpdate.previewAgain")}</button>
    </div>}

    {phase.kind === "applied" && <div className="catalog-update-preview">
      <p role="status">{t("catalogUpdate.applied", { backup: phase.result.backup })}</p>
      <CatalogInstallReportView report={phase.result.report} />
    </div>}
  </>;
}
