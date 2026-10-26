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

/**
 * `value` is the raw server field. `absent` is an older record or server
 * that kept no cause: shown as not recorded, never as success. `error` is
 * the server's own text, local to this computer and never in a report.
 */
export default function DownloadFailureView({ value, absent = false, error = null }: { value?: unknown; absent?: boolean; error?: string | null }) {
  const t = useTranslate();
  const failure = absent ? null : admitDownloadFailure(value);
  const technical = error && <details className="download-failure-technical">
    <summary>{t("downloadFailure.technical")}</summary>
    <p>{error}</p>
  </details>;
  if (failure === null) {
    return <section className="download-failure" data-failure={absent ? "notRecorded" : "unreadable"} role="alert">
      <h4>{t("downloadFailure.title.failed")}</h4>
      <p>{t(absent ? "downloadFailure.notRecorded" : "downloadFailure.unreadable")}</p>
      {technical}
    </section>;
  }
  const stopped = failure.category === "stopped";
  return (
    <section className={`download-failure${stopped ? " download-failure--stopped" : ""}`}
      data-failure={failure.category} data-failure-code={failure.code} role={stopped ? "status" : "alert"}>
      <h4>{t(stopped ? "downloadFailure.title.stopped" : "downloadFailure.title.failed")}</h4>
      <p>
        {t("downloadFailure.cause", { cause: t(`downloadFailure.category.${failure.category}`) })}{" "}
        <code title={t("downloadFailure.code")}>{failure.code}</code>
      </p>
      <p data-failure-phase={failure.phase}>{placeLine(t, failure)}</p>
      <p data-failure-restart={failure.restart}>{t(`downloadFailure.restart.${failure.restart}`)}</p>
      <p>{t("downloadFailure.readBack", { count: failure.readBackOctets })}</p>
      {technical}
    </section>
  );
}
