/** Group-address views' last-download summary line (server comparison, ADR-0174). */
import type { LastDownloadCoverage } from "./groupAddressLastDownloadCoverage";
import type { GroupAddressPresentationRow } from "./groupAddressPresentation";
import { useTranslate } from "./i18n";
import { useUiLanguage } from "./uiLanguage";
import { formatLogTime } from "./logRows";
import type { DownloadComparisonState } from "./bindings/DownloadComparisonState";

const NOT_COMPARED = ["loading", "failed", "absent", "otherProgram", "productUnavailable", "currentUnreadable"] as const;

/**
 * Static line above the views. `differingRows` holds the marked row keys of the
 * whole index; `hidden` counts marked rows the current query, filter or scope
 * removed. `coverage` says which devices the comparison covers: only when every
 * relevant device is compared may it say that nothing differs.
 */
export function GroupAddressLastDownloadSummary({ differingRows, rows, coverage }: {
  differingRows: ReadonlySet<string>;
  rows: GroupAddressPresentationRow[];
  coverage: LastDownloadCoverage | null;
}) {
  const t = useTranslate();
  const [uiLanguage] = useUiLanguage();
  if (coverage === null || coverage.total === 0) return null;
  const { total, compared, counts } = coverage;
  const complete = compared === total;
  const shown = new Set(rows.map(row => row.key));
  let hidden = 0;
  for (const key of differingRows) if (!shown.has(key)) hidden += 1;
  const missing = NOT_COMPARED.filter(state => counts[state] > 0);
  const uniform = missing.length === 1 && compared === 0;
  const state: DownloadComparisonState | "mixed" = complete ? "usable" : uniform
    && missing[0] !== "loading" && missing[0] !== "failed" ? missing[0]! : "mixed";
  const time = coverage.latestAt === undefined ? "" : formatLogTime(Date.parse(coverage.latestAt), uiLanguage, new Date()) ?? coverage.latestAt;
  const count = differingRows.size;
  const parts: string[] = [];
  if (!complete && state === "mixed") {
    parts.push(t(total === 1 ? "gaWorkspace.lastDownload.coverage.one" : "gaWorkspace.lastDownload.coverage.other", { compared, total }));
    parts.push(t("gaWorkspace.lastDownload.notCompared", { list: missing.map(reason => t(`gaWorkspace.lastDownload.reason.${reason}`, { count: counts[reason] })).join(", ") }));
  }
  if (compared > 0) {
    parts.push(count === 0 ? t(complete ? "gaWorkspace.lastDownload.noneDiffer" : "gaWorkspace.lastDownload.noneDifferCompared")
      : t(count === 1 ? "gaWorkspace.lastDownload.differs.one" : "gaWorkspace.lastDownload.differs.other", { count }));
    if (hidden > 0) parts.push(t(hidden === 1 ? "gaWorkspace.lastDownload.hidden.one" : "gaWorkspace.lastDownload.hidden.other", { count: hidden }));
    parts.push(t("comTable.lastDownload.links.usable", { time }));
  } else if (state !== "mixed") {
    parts.push(t(`comTable.lastDownload.links.${state}` as const));
  }
  const differs = compared > 0 && count > 0;
  return <p className="ga-last-download" data-links={state} data-coverage={complete ? "complete" : "partial"}
    data-compared={compared} data-total={total} data-differs={differs ? "true" : undefined}>
    {differs && <span className="ga-last-download-dot" aria-hidden="true" />}
    <span>{parts.join(" ")}</span>
  </p>;
}
