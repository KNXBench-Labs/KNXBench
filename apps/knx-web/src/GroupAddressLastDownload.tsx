/** Group-address views' last-download summary line (server comparison, ADR-0174). */
import type { ObjectLastDownload } from "./bindings/ObjectLastDownload";
import type { GroupAddressPresentationRow } from "./groupAddressPresentation";
import { useTranslate } from "./i18n";
import { useUiLanguage } from "./uiLanguage";
import { formatLogTime } from "./logRows";
import type { DownloadComparisonState } from "./bindings/DownloadComparisonState";

/**
 * Static line above the views. `differingRows` holds the marked row keys of the
 * whole index; `hidden` counts marked rows the current query, filter or scope
 * removed. One sentence per links state when every read device shares it.
 */
export function GroupAddressLastDownloadSummary({ differingRows, rows, lastDownloads }: {
  differingRows: ReadonlySet<string>;
  rows: GroupAddressPresentationRow[];
  lastDownloads: ReadonlyMap<number, ObjectLastDownload | undefined>;
}) {
  const t = useTranslate();
  const [uiLanguage] = useUiLanguage();
  const entries = [...lastDownloads.values()].filter((entry): entry is ObjectLastDownload => entry !== undefined);
  if (entries.length === 0) return null;
  const usable = entries.filter(entry => entry.links.state === "usable");
  const compared = usable.length > 0;
  const shown = new Set(rows.map(row => row.key));
  let hidden = 0;
  for (const key of differingRows) if (!shown.has(key)) hidden += 1;
  const states = new Set(entries.map(entry => entry.links.state));
  const state: DownloadComparisonState | "mixed" = states.size === 1 ? [...states][0]! : "mixed";
  const latest = usable.reduce<string | undefined>((best, entry) =>
    entry.links.at !== undefined && (best === undefined || entry.links.at > best) ? entry.links.at : best, undefined);
  const time = latest === undefined ? "" : formatLogTime(Date.parse(latest), uiLanguage, new Date()) ?? latest;
  const count = differingRows.size;
  const parts: string[] = [];
  if (compared) {
    parts.push(count === 0 ? t("gaWorkspace.lastDownload.noneDiffer")
      : t(count === 1 ? "gaWorkspace.lastDownload.differs.one" : "gaWorkspace.lastDownload.differs.other", { count }));
    if (hidden > 0) parts.push(t(hidden === 1 ? "gaWorkspace.lastDownload.hidden.one" : "gaWorkspace.lastDownload.hidden.other", { count: hidden }));
  }
  if (state !== "mixed") {
    parts.push(state === "usable"
      ? t("comTable.lastDownload.links.usable", { time })
      : t(`comTable.lastDownload.links.${state}` as const));
  }
  const differs = compared && count > 0;
  return <p className="ga-last-download" data-links={state} data-differs={differs ? "true" : undefined}>
    {differs && <span className="ga-last-download-dot" aria-hidden="true" />}
    <span>{parts.join(" ")}</span>
  </p>;
}
