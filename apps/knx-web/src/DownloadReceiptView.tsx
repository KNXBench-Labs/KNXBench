/** Display one run's recorded preparation and settlement without granting write authority. */
import { useTranslate } from "./i18n";
import "./DownloadReceiptView.css";
import { PART_KEYS, type DownloadPreparation, type DownloadSettlement, type SettledParts } from "./downloadReceipt";

function Yes({ value }: { value: boolean }) {
  const t = useTranslate();
  return <>{t(value ? "downloadReceipt.yes" : "downloadReceipt.no")}</>;
}

function PartsTable({ parts, label }: { parts: SettledParts; label: string }) {
  const t = useTranslate();
  return (
    <div className="download-receipt-parts-scroll" role="region" aria-label={label} tabIndex={0}>
    <table className="download-receipt-parts">
      <thead><tr>
        <th>{t("downloadReceipt.part")}</th>
        <th>{t("downloadReceipt.programmed")}</th>
        <th>{t("downloadReceipt.pending")}</th>
        <th>{t("downloadReceipt.touched")}</th>
      </tr></thead>
      <tbody>
        {PART_KEYS.map((key) => (
          <tr key={key} data-part={key}>
            <td>{t(`downloadReceipt.part.${key}`)}</td>
            <td><Yes value={parts.programmed[key]} /></td>
            <td><Yes value={parts.pending[key]} /></td>
            <td><Yes value={parts.touched[key]} /></td>
          </tr>
        ))}
      </tbody>
    </table>
    </div>
  );
}

function Preparation({ value }: { value: DownloadPreparation }) {
  const t = useTranslate();
  return (
    <div className="download-receipt-preparation" data-state={value.state}>
      <h5>{t("downloadReceipt.preparation")}</h5>
      <dl>
        <dt>{t("downloadReceipt.scope")}</dt><dd>{t(`downloadReceipt.scope.${value.scope}`)}</dd>
        {value.state === "unavailable" ? <>
          <dt>{t("downloadReceipt.plan")}</dt><dd>{t("downloadReceipt.plan.unavailable")}</dd>
        </> : <>
          <dt>{t("downloadReceipt.program")}</dt>
          <dd>{value.program.state === "unavailable" ? t("downloadReceipt.program.unavailable") : <>
            <code>{value.program.id}</code>
          </>}</dd>
          {value.program.state === "identified" && <>
            <dt>{t("downloadReceipt.version")}</dt>
            <dd>{value.program.applicationVersion === null ? t("downloadReceipt.version.none") : <code>{value.program.applicationVersion}</code>}</dd>
            <dt>{t("downloadReceipt.sha")}</dt><dd><code>{value.program.sourceSha256}</code></dd>
          </>}
          <dt>{t("downloadReceipt.plannedMask")}</dt><dd><code>{value.plannedMask}</code></dd>
          <dt>{t("downloadReceipt.plannedManufacturer")}</dt><dd><code>{value.plannedManufacturer}</code></dd>
          <dt>{t("downloadReceipt.steps")}</dt><dd>{value.steps}</dd>
          <dt>{t("downloadReceipt.dataOctets")}</dt><dd>{value.dataOctets}</dd>
        </>}
      </dl>
    </div>
  );
}

function Settlement({ value, language }: { value: DownloadSettlement; language: string }) {
  const t = useTranslate();
  const parts = value.project === "persisted" || value.project === "sessionOnly" ? value.applied
    : value.project === "persistenceFailed" ? value.computed : null;
  const caption = t(value.project === "persistenceFailed" ? "downloadReceipt.partsComputed" : "downloadReceipt.partsApplied");
  return (
    <div className="download-receipt-settlement" data-project={value.project}>
      <h5>{t("downloadReceipt.settlement")}</h5>
      <p className={value.project === "persistenceFailed" ? "form-warning" : undefined} role={value.project === "persistenceFailed" ? "status" : undefined}>
        {t(`downloadReceipt.project.${value.project}`)}
      </p>
      {"source" in value && <>
        <p>{t(`downloadReceipt.source.${value.source}`)}</p>
        <p>{t(`downloadReceipt.request.${value.currentRequest}`)}</p>
      </>}
      {parts !== null && <>
        <p>{caption}</p>
        <PartsTable parts={parts} label={caption} />
      </>}
      {"settledAt" in value && <p>
        {t("downloadReceipt.settledAt")}: <time dateTime={value.settledAt}>{new Intl.DateTimeFormat(language, { dateStyle: "short", timeStyle: "medium" }).format(new Date(value.settledAt))}</time>
      </p>}
    </div>
  );
}

/** Absent is "no receipt recorded by this history", never a state of the run. */
export default function DownloadReceiptView({ preparation, settlement, language }: {
  preparation?: DownloadPreparation; settlement?: DownloadSettlement; language: string;
}) {
  const t = useTranslate();
  return (
    <section className="download-receipt" data-preparation={preparation?.state ?? "absent"} data-settlement={settlement?.project ?? "absent"}>
      <h4>{t("downloadReceipt.title")}</h4>
      <p>{t("downloadReceipt.boundary")}</p>
      {preparation === undefined && settlement === undefined && <p>{t("downloadReceipt.absent")}</p>}
      {preparation !== undefined && <Preparation value={preparation} />}
      {preparation !== undefined && settlement === undefined && <p>{t("downloadReceipt.settlement.absent")}</p>}
      {settlement !== undefined && <Settlement value={settlement} language={language} />}
    </section>
  );
}
