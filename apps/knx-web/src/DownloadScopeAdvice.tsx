/** Shows the flag-based scope advice: resolved scope, unmet address/medium needs, its basis. */
import type { DeviceDownloadScopeAdvice, DeviceDownloadScopeCode } from "./api";
import { uncoveredParts, unmetNeeds } from "./downloadScopeAdvice";
import { useTranslate } from "./i18n";

interface Props {
  advice: DeviceDownloadScopeAdvice;
  /** The scope a `changes` request resolved to; absent for an explicit scope or a no-op. */
  resolved?: DeviceDownloadScopeCode;
  /** The scope the shown plan writes; pending parts outside it are named (#39). */
  scope?: DeviceDownloadScopeCode;
}

export default function DownloadScopeAdvice({ advice, resolved, scope }: Props) {
  const t = useTranslate();
  const uncovered = scope === undefined ? [] : uncoveredParts(scope, advice);
  return (
    <section className="device-download-scope-advice" data-scope-advice={advice.write}>
      <h4>{t("deviceDownload.scopeAdvice.title")}</h4>
      {resolved !== undefined
        ? <p data-scope-resolved>{t(`deviceDownload.scopeAdvice.resolved.${resolved}` as const)}</p>
        : <p data-scope-advised>{advice.write === "nothing" ? t("deviceDownload.scopeAdvice.advisedNothing")
          : t("deviceDownload.scopeAdvice.advised", { scope: t(`deviceDownload.scopeAdvice.write.${advice.write}` as const) })}</p>}
      {uncovered.length > 0 && (
        <div className="device-download-scope-uncovered" role="note" data-scope-uncovered={uncovered.join(" ")}>
          <p><strong>{t(uncovered.includes("application") ? "deviceDownload.scopeAdvice.uncovered.titleApplication"
            : "deviceDownload.scopeAdvice.uncovered.title")}</strong></p>
          <ul>{uncovered.map((part) => (
            <li key={part} data-scope-uncovered-part={part}>{t(`deviceDownload.scopeAdvice.uncovered.${part}` as const)}</li>
          ))}</ul>
          <p>{t("deviceDownload.scopeAdvice.uncovered.after")}</p>
        </div>
      )}
      {unmetNeeds(advice).map((need) => (
        <p key={need} className="form-warning" data-scope-unmet={need}>
          {t(need === "medium" ? "deviceDownload.scopeAdvice.medium"
            : advice.address === "missing" ? "deviceDownload.scopeAdvice.address.missing"
              : "deviceDownload.scopeAdvice.address.pending")}
        </p>
      ))}
      {advice.fullyProgrammed && <p data-scope-fully-programmed>{t("deviceDownload.scopeAdvice.fullyProgrammed")}</p>}
      <p className="device-download-plan-hint">{t("deviceDownload.scopeAdvice.basis")}</p>
    </section>
  );
}
