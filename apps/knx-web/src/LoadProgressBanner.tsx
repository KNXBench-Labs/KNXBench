/** The banner saying what a running project load is doing, or was doing when it failed. */
// Renders exactly what the server reported and nothing more (ADR-0023).
// The bar is determinate only while a real completed/total pair is on the
// wire — two phases of seventeen — and indeterminate the rest of the time,
// which is deliberately *not* the same as "zero percent": an empty bar
// that never moves is a worse lie than an honest shuttle.

import type { LoadProgressSnapshot } from "./api";
import { useTranslate } from "./i18n";
import { loadFraction, phaseMessageKey } from "./loadProgress";

export interface LoadProgressBannerProps {
  /** The file name the load names itself by, known from the moment the
   * user picked it — so the banner has something true to say before the
   * first poll has answered. */
  source: string;
  /** `null` until the first poll returns, and after that whatever the
   * server last reported. */
  snapshot: LoadProgressSnapshot | null;
}

export default function LoadProgressBanner({ source, snapshot }: LoadProgressBannerProps) {
  const t = useTranslate();
  const failed = snapshot?.status === "failed";
  const fraction = failed ? null : loadFraction(snapshot);
  const phaseKey = snapshot ? phaseMessageKey(snapshot.phase) : null;
  // No snapshot yet means the poll has not answered, not that nothing is
  // happening — and an unknown phase name shows through as itself rather
  // than being smoothed into a comfortable generic label.
  const phase = snapshot ? (phaseKey ? t(phaseKey) : snapshot.phase) : t("loadProgress.phase.starting");
  const heading = failed
    ? t("loadProgress.failed", { source })
    : snapshot?.kind === "open"
      ? t("loadProgress.opening", { source })
      : t("loadProgress.importing", { source });

  return (
    <section className="load-progress" role="status" aria-live="polite" data-failed={failed ? "true" : undefined}>
      <p className="load-progress-heading">
        <strong>{heading}</strong>
        <span className="load-progress-phase">{failed ? t("loadProgress.failedDuring", { phase }) : phase}</span>
      </p>
      {!failed && (
        <div
          className="load-progress-track"
          role="progressbar"
          aria-label={t("loadProgress.barLabel")}
          aria-valuemin={0}
          aria-valuemax={100}
          // Omitted, not zeroed, when there is no measurement: that is
          // exactly what ARIA defines an indeterminate progressbar as.
          aria-valuenow={fraction ? fraction.percent : undefined}
          aria-valuetext={
            fraction ? t("loadProgress.counted", { completed: fraction.completed, total: fraction.total }) : undefined
          }
        >
          <span
            className="load-progress-bar"
            data-indeterminate={fraction ? undefined : "true"}
            style={fraction ? { width: `${fraction.percent}%` } : undefined}
          />
        </div>
      )}
      {fraction && (
        <p className="load-progress-count">
          {t("loadProgress.counted", { completed: fraction.completed, total: fraction.total })}
        </p>
      )}
      {failed && snapshot?.error && <p className="load-progress-error">{snapshot.error}</p>}
    </section>
  );
}
