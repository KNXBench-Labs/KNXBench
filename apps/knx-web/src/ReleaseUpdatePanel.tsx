/** Durable update information after the startup toast expires; never installs software. */
import { useTranslate } from "./i18n";
import type { ReleaseUpdateControls } from "./useReleaseUpdates";
export default function ReleaseUpdatePanel({ updates }: { updates: ReleaseUpdateControls }) {
  const t = useTranslate();
  return <section className="release-update-panel" aria-label={t("updates.section")}>
    <p role="status"><span className={updates.status === "checking" ? "task-status-text" : undefined}>{updates.status === "available" && updates.update
      ? t("updates.available", { version: updates.update.version }) : t(`updates.${updates.status === "available" ? "idle" : updates.status}`)}</span></p>
    {updates.update && <>
      <a href={updates.update.url} target="_blank" rel="noopener noreferrer">{t("updates.releaseDetails")}</a>
      {updates.update.prerelease && <p>{t("updates.preview")}</p>}
    </>}
    <p><button type="button" disabled={updates.status === "checking"} onClick={() => { void updates.check(); }}>{t("updates.check")}</button></p>
  </section>;
}
