/** Renders one failed download's diagnostic as separate facts: cause, place, restart, read-back. */
import { useTranslate } from "./i18n";
import { admitDownloadFailure, type DownloadFailure } from "./downloadFailure";

function placeLine(t: ReturnType<typeof useTranslate>, failure: DownloadFailure) {
  const { step, phase } = failure;
  if (step === null) {
    return phase === "beforeSteps" ? t("downloadFailure.phase.beforeSteps") : t("downloadFailure.phase.unknownStep");
  }
  const values = { number: step.number, of: step.of, summary: step.summary };
  switch (phase) {
    case "inStep": return t("downloadFailure.phase.inStep", values);
    case "backupBeforeStep": return t("downloadFailure.phase.backupBeforeStep", values);
    case "stoppedBeforeStep": return t("downloadFailure.phase.stoppedBeforeStep", values);
    // Admission refuses a step before the first step.
    case "beforeSteps": return t("downloadFailure.phase.beforeSteps");
  }
}

interface Props {
  /** The raw server field. */
  value?: unknown;
  /** An older record or server that kept no cause: not recorded, never success. */
  absent?: boolean;
  /** The server's own text: local to this computer, never in a report. */
  error?: string | null;
  /** The run's write classification; the partial-write caveat needs "partially". */
  written: "yes" | "no" | "partially" | null;
  /** `true` for the live result; a stored history row is not announced. */
  live?: boolean;
}

export default function DownloadFailureView({ value, absent = false, error = null, written, live = true }: Props) {
  const t = useTranslate();
  const failure = absent ? null : admitDownloadFailure(value);
  const role = (kind: "alert" | "status") => (live ? kind : undefined);
  const technical = error && <details className="download-failure-technical">
    <summary>{t("downloadFailure.technical")}</summary>
    <p>{error}</p>
  </details>;
  if (failure === null) {
    return <section className="download-failure" data-failure={absent ? "notRecorded" : "unreadable"} role={role("alert")}>
      <h4>{t("downloadFailure.title.failed")}</h4>
      <p>{t(absent ? "downloadFailure.notRecorded" : "downloadFailure.unreadable")}</p>
      {technical}
    </section>;
  }
  const stopped = failure.category === "stopped";
  return (
    <section className={`download-failure${stopped ? " download-failure--stopped" : ""}`}
      data-failure={failure.category} data-failure-code={failure.code} role={role(stopped ? "status" : "alert")}>
      <h4>{t(stopped ? "downloadFailure.title.stopped" : "downloadFailure.title.failed")}</h4>
      <p>
        {t("downloadFailure.cause", { cause: t(`downloadFailure.category.${failure.category}`) })}{" "}
        <code title={t("downloadFailure.code")}>{failure.code}</code>
      </p>
      <p data-failure-phase={failure.phase}>{placeLine(t, failure)}</p>
      <p data-failure-restart={failure.restart}>{t(`downloadFailure.restart.${failure.restart}`)}</p>
      <p>{t("downloadFailure.readBack", { count: failure.readBackOctets })}
        {written === "partially" && <>{" "}{t("downloadFailure.partialCaveat")}</>}</p>
      {technical}
    </section>
  );
}
