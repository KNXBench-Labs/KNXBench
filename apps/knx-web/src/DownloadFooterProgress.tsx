/** Status-bar progress of a running device download, visible from any view (issue #43). */
import { useBusConnection } from "./busConnection";
import type { DownloadProgress } from "./gatewayRefusal";
import { useTranslate } from "./i18n";

/** Percent of the planned octets written and read back; steps when a plan writes no data. */
export function downloadPercent(progress: DownloadProgress): number {
  const [done, total] = progress.totalOctets > 0
    ? [progress.writtenOctets, progress.totalOctets] : [progress.completedSteps, progress.totalSteps];
  return total > 0 ? Math.min(100, Math.floor((done * 100) / total)) : 0;
}

/**
 * Reads the shared bus-connection observation the status bar already polls;
 * it starts, stops and cancels nothing. Cancelling stays in the owning panel.
 */
export default function DownloadFooterProgress({ onOpen }: { onOpen: () => void }) {
  const t = useTranslate();
  const { view } = useBusConnection();
  if (view.phase !== "occupied" || view.holder.kind !== "deviceDownload") return null;
  const { address, progress } = view.holder;
  return <span className="download-footer-progress" data-download-address={address}>
    <span role="status" className="task-status-text">
      {progress
        ? t("deviceDownload.footer.progress", { address, done: progress.completedSteps, of: progress.totalSteps, percent: downloadPercent(progress) })
        : t("deviceDownload.footer.running", { address })}
    </span>
    {progress && <progress aria-label={t("deviceDownload.footer.bar", { address })}
      max={Math.max(progress.totalOctets > 0 ? progress.totalOctets : progress.totalSteps, 1)}
      value={progress.totalOctets > 0 ? progress.writtenOctets : progress.completedSteps} />}
    <button type="button" onClick={onOpen}>{t("deviceDownload.footer.open")}</button>
  </span>;
}
