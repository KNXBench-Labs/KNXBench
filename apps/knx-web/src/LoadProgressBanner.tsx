/** The banner saying what a running project load is doing, or was doing when it failed. */
// Renders exactly what the server reported and nothing more (ADR-0023).
// The bar is determinate only while a real completed/total pair is on the
// wire — two phases of seventeen — and indeterminate the rest of the time,
// which is deliberately *not* the same as "zero percent": an empty bar
// that never moves is a worse lie than an honest shuttle.

import { useEffect, useState } from "react";
import type { LoadProgressSnapshot } from "./api";
import { useTranslate } from "./i18n";
import { FLAVOUR_INTERVAL_MS, flavourRotates, shuffleFlavourKeys } from "./loadFlavour";
import { loadFraction, phaseMessageKey } from "./loadProgress";

export interface LoadProgressBannerProps {
  /** The file name the load names itself by, known from the moment the
   * user picked it — so the banner has something true to say before the
   * first poll has answered. */
  source: string;
  /** `null` until the first poll returns, and after that whatever the
   * server last reported. */
  snapshot: LoadProgressSnapshot | null;
  /** Where the flavour line's shuffle gets its dice. Injected so a test
   * can pin the sequence; production passes nothing and gets
   * `Math.random`. It decides the order of a joke and nothing else — no
   * phase, no count, no bar. */
  random?: () => number;
}

export default function LoadProgressBanner({ source, snapshot, random = Math.random }: LoadProgressBannerProps) {
  const t = useTranslate();
  const failed = snapshot?.status === "failed";
  // Shuffled once, on mount. `App` remounts the banner per load, so two
  // loads in a row do not open on the same line.
  const [flavourOrder] = useState(() => shuffleFlavourKeys(random));
  const [flavourIndex, setFlavourIndex] = useState(0);
  // The one timer in this component, and it advances an index into a list
  // of jokes. It never touches the fraction, the counts or
  // `data-indeterminate`: a timer that changes text is not a timer that
  // fabricates progress, and the moment it were allowed near any of those
  // it would become one. It is cleared on unmount (the load succeeding
  // unmounts the banner) and never armed at all once the load has failed
  // — a banner apologising for a broken archive has no business being
  // funny about star constellations.
  useEffect(() => {
    if (failed) return;
    if (!flavourRotates(window, document.documentElement)) return;
    const id = setInterval(() => setFlavourIndex((i) => i + 1), FLAVOUR_INTERVAL_MS);
    return () => clearInterval(id);
  }, [failed]);
  const flavour = failed ? null : flavourOrder[flavourIndex % flavourOrder.length];
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
          // Outside the live region: the counted phases step through
          // every value (38 entries, 24 files), and a screen reader that
          // announced each one would read a phase change out of a queue
          // of dozens of numbers. The phase is the news; the count is
          // there for whoever asks the bar directly.
          aria-live="off"
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
        <p className="load-progress-count" aria-live="off">
          {t("loadProgress.counted", { completed: fraction.completed, total: fraction.total })}
        </p>
      )}
      {/* Subordinate on purpose, in both senses. Visually it is the
          quietest line in the banner, below the phase and the count; to a
          screen reader it does not exist at all. `aria-hidden` keeps it
          out of the accessibility tree and `aria-live="off"` keeps it out
          of this section's polite region even where a reader is pointed
          straight at it — round 1 of the banner already shipped one bug
          that announced itself 38 times, and a line that changes every
          1 800 ms would be a far louder version of the same mistake. */}
      {flavour && (
        <p className="load-progress-flavour" aria-hidden="true" aria-live="off">
          {t(flavour)}
        </p>
      )}
      {failed && snapshot?.error && <p className="load-progress-error">{snapshot.error}</p>}
    </section>
  );
}
