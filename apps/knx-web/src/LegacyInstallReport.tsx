/** ADR-0094 (L3): what importing a legacy ETS3 product database did. */
import type { LegacyInstallReport } from "./api";
import { useTranslate } from "./i18n";

export function LegacyInstallReportView(props: { report: LegacyInstallReport }) {
  const t = useTranslate();
  const { report } = props;
  return (
    <section className="catalog-report legacy-report" aria-live="polite" aria-label={t("catalog.installReport.heading")}>
      <p>
        {t(report.skipped ? "legacyReport.already" : "legacyReport.imported", {
          programs: t("legacyReport.programs", { count: report.programs.length }),
        })}
      </p>
      <ul>
        <li>{t("legacyReport.catalogItems", { count: report.catalogItems })}</li>
        <li>{t("legacyReport.parameters", { count: report.parameters, refs: report.parameterRefs })}</li>
        <li>{t("legacyReport.comObjects", { count: report.comObjectRefs })}</li>
        <li>{t("legacyReport.translations", { count: report.translations })}</li>
        <li>{t("legacyReport.namespace", { namespace: report.namespace })}</li>
      </ul>
      <p>
        {report.password === "remembered" ? t("legacyReport.passwordRemembered")
          : report.password === "given" ? t("legacyReport.passwordGiven")
          : t("legacyReport.passwordNone")}
        {report.remembered && <> {t("legacyReport.nowRemembered")}</>}
      </p>
      {report.rememberProblem && (
        <p className="field-error">{t("legacyReport.rememberProblem", { problem: report.rememberProblem })}</p>
      )}
      {report.diagnostics.length > 0 && (
        <>
          <h4>{t("legacyReport.diagnosticsHeading", { count: report.diagnostics.length })}</h4>
          <ul>
            {report.diagnostics.map((diagnostic, index) => (
              <li key={index}><code>{diagnostic.kind}</code> {diagnostic.detail}</li>
            ))}
          </ul>
        </>
      )}
      <p className="catalog-report catalog-report-caution">{t("legacyReport.offlineOnly")}</p>
    </section>
  );
}
