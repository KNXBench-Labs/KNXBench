/** Renders the read-only check after a download as separate facts; it never offers a resend. */
import { useTranslate } from "./i18n";
import { admitPostDownloadCheck, checkLines, machineReadings, type MachineReading } from "./postDownloadCheck";

function observedKey(reading: MachineReading) {
  return reading.observed.result === "observed"
    ? `postDownloadCheck.state.${reading.observed.loadState}` as const
    : `postDownloadCheck.observed.${reading.observed.result}` as const;
}

/** `value` is the raw server field: absent (an older server) reads as not checked. */
export default function PostDownloadCheckView({ value }: { value: unknown }) {
  const t = useTranslate();
  const check = admitPostDownloadCheck(value);
  if (check === null) {
    return <section className="post-download-check" data-check="unreadable">
      <h4>{t("postDownloadCheck.title")}</h4>
      <p className="form-warning" role="status">{t("postDownloadCheck.unreadable")}</p>
    </section>;
  }
  const readings = machineReadings(check);
  const reach = check.observation.status === "attempted" ? check.observation.reach.result : "none";
  return (
    <section className="post-download-check" data-check={check.observation.status} data-reach={reach}>
      <h4>{t("postDownloadCheck.title")}</h4>
      <p>{t("postDownloadCheck.scope")}</p>
      {checkLines(check).map((line) => (
        <p key={line.key} className={line.warning ? "form-warning" : undefined} role={line.warning ? "status" : undefined}>
          {t(line.key, line.values)}
        </p>
      ))}
      {readings.length > 0 && (
        <table className="post-download-check-machines">
          <thead><tr><th>{t("postDownloadCheck.machine")}</th><th>{t("postDownloadCheck.expected")}</th><th>{t("postDownloadCheck.observed")}</th></tr></thead>
          <tbody>
            {readings.map((reading) => (
              <tr key={reading.machine} data-machine={reading.machine}>
                <td>{t(`postDownloadCheck.machine.${reading.machine}`)}</td>
                <td>{reading.expected === null ? t("postDownloadCheck.expected.unresolved") : t(`postDownloadCheck.state.${reading.expected}`)}</td>
                <td>{t(observedKey(reading))}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
      {readings.length > 0 && <p>{t("postDownloadCheck.exemplar")}</p>}
      <p>{t("postDownloadCheck.runtimeUnknown")}</p>
    </section>
  );
}
