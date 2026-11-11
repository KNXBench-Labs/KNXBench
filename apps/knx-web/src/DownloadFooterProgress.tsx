/** Status-bar progress of a running device download, visible from any view (issue #43). */
import { useEffect, useRef, useState } from "react";
import { acquireBusConnectionPolling, useBusConnection } from "./busConnection";
import type { DownloadProgress } from "./gatewayRefusal";
import { useTranslate } from "./i18n";

/** Percent of the planned data octets written and read back; never overall run progress. */
export function dataPercent(progress: DownloadProgress): number | null {
  if (progress.totalOctets === 0) return null;
  return Math.min(100, Math.floor((progress.writtenOctets * 100) / progress.totalOctets));
}

/**
 * Reads the shared bus-connection observation (keeping its polling alive);
 * it starts, stops and cancels nothing. Cancelling stays in the owning panel.
 * The counts are the server's finished steps and verified octets, worded as
 * such, so they never contradict the panel's "step N running" line.
 */
export default function DownloadFooterProgress({ onOpen }: { onOpen: () => void }) {
  const t = useTranslate();
  const { view } = useBusConnection();
  useEffect(() => acquireBusConnectionPolling(), []);
  const holder = view.phase === "occupied" && view.holder.kind === "deviceDownload" ? view.holder : null;
  // Announce only start and end, not every two-second poll; the bar carries the value.
  const [announcement, setAnnouncement] = useState("");
  const running = useRef<string | null>(null);
  // A hidden window, a failed read or the download's own lock seen without its
  // session say nothing about the run: keep the last known state, announce nothing.
  const undecided = view.phase === "unknown"
    || (view.phase === "occupied" && view.holder.kind === "connecting" && view.holder.session === "deviceDownload");
  useEffect(() => {
    if (undecided) return;
    const address = holder?.address ?? null;
    if (address === running.current) return;
    setAnnouncement(address !== null
      ? t("deviceDownload.footer.started", { address })
      : t("deviceDownload.footer.ended", { address: running.current ?? "" }));
    running.current = address;
  }, [holder?.address, undecided, t]);
  const progress = holder?.progress;
  const percent = progress ? dataPercent(progress) : null;
  return <>
    <span className="sr-only" role="status" aria-live="polite" aria-atomic="true">{announcement}</span>
    {holder && <span className="download-footer-progress" data-download-address={holder.address}>
      <span className="task-status-text">
        {progress
          ? t(percent === null ? "deviceDownload.footer.steps" : "deviceDownload.footer.progress",
            { address: holder.address, done: progress.completedSteps, of: progress.totalSteps, percent: percent ?? 0 })
          : t("deviceDownload.footer.running", { address: holder.address })}
      </span>
      {progress && percent !== null && <progress aria-label={t("deviceDownload.footer.bar", { address: holder.address })}
        max={Math.max(progress.totalOctets, 1)} value={progress.writtenOctets} />}
      <button type="button" aria-label={t("deviceDownload.footer.openLabel", { address: holder.address })} onClick={onOpen}>
        {t("deviceDownload.footer.open")}</button>
    </span>}
  </>;
}
