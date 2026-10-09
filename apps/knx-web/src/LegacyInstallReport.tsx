/** ADR-0094 (L3): what importing a legacy ETS3 product database did. */
import type { LegacyInstallReport } from "./api";
import { useTranslate } from "./i18n";

/**
 * Every diagnostic, grouped by kind in first-seen order. The measured `.vd5`
 * yields about 1,800 (ADR-0094, VD5), so each kind folds; none is dropped.
 */
export function groupByKind(diagnostics: LegacyInstallReport["diagnostics"]): [string, string[]][] {
  const groups = new Map<string, string[]>();
  for (const { kind, detail } of diagnostics) {
    const group = groups.get(kind);
    if (group) group.push(detail);
    else groups.set(kind, [detail]);
  }
  return [...groups.entries()];
}

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
          {groupByKind(report.diagnostics).map(([kind, details]) => (
            <details key={kind} className="legacy-report-kind">
              <summary><code>{kind}</code> ({details.length})</summary>
              <ul>
                {details.map((detail, index) => <li key={index}>{detail}</li>)}
              </ul>
            </details>
          ))}
        </>
      )}
      <p className="catalog-report catalog-report-caution">{t("legacyReport.offlineOnly")}</p>
    </section>
  );
}
