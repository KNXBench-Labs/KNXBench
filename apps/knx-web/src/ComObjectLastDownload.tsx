/** Object-table marks, captions and summary for links and flags since the last download. */
// ADR-0174 §5: the server compares; this file only presents its verdict.
import type { DownloadComparisonState } from "./bindings/DownloadComparisonState";
import type { DownloadObjectDifference } from "./bindings/DownloadObjectDifference";
import type { DownloadObjectFlags } from "./bindings/DownloadObjectFlags";
import type { ObjectLastDownload } from "./bindings/ObjectLastDownload";
import { rawGroupAddressText } from "./gaNotation";
import { useTranslate, type MessageKey, type Translate } from "./i18n";
import { formatLogTime } from "./logRows";
import { differenceKind, flagText, linkChanges } from "./objectLastDownload";
import { useUiLanguage } from "./uiLanguage";

const ROW_KEYS: Record<"links" | "flags" | "both", MessageKey> = {
  links: "comTable.lastDownload.row.links", flags: "comTable.lastDownload.row.flags", both: "comTable.lastDownload.row.both" };

/** The row caption: what differs, always visible, and the summary's description. */
export function rowNoteText(t: Translate, difference: DownloadObjectDifference): string {
  return t(ROW_KEYS[differenceKind(difference)]);
}

type Unusable = Exclude<DownloadComparisonState, "usable">;

function sectionSentence(t: Translate, section: "links" | "flags", state: DownloadComparisonState, time: string): string {
  return state === "usable" ? t(`comTable.lastDownload.${section}.usable` as const, { time })
    : t(`comTable.lastDownload.${section}.${state as Unusable}` as const);
}

/**
 * Static line above the table. `hidden` counts selected rows the current
 * filters or collapsed groups hide; the server counts rows it could not select.
 */
export function ObjectLastDownloadSummary({ lastDownload, hidden }: { lastDownload: ObjectLastDownload; hidden: number }) {
  const t = useTranslate();
  const [uiLanguage] = useUiLanguage();
  const time = (at: string | undefined) => at === undefined ? "" : formatLogTime(Date.parse(at), uiLanguage, new Date()) ?? at;
  const { links, flags } = lastDownload;
  const compared = links.state === "usable" || flags.state === "usable";
  const count = compared ? lastDownload.differing_objects : 0;
  const parts: string[] = [];
  if (compared) {
    parts.push(count === 0 ? t("comTable.lastDownload.noneDiffer")
      : t(count === 1 ? "comTable.lastDownload.differs.one" : "comTable.lastDownload.differs.other", { count }));
    if (hidden > 0) parts.push(t(hidden === 1 ? "comTable.lastDownload.hidden.one" : "comTable.lastDownload.hidden.other", { count: hidden }));
    const noRow = lastDownload.no_selected_row;
    if (noRow > 0) parts.push(t(noRow === 1 ? "comTable.lastDownload.noRow.one" : "comTable.lastDownload.noRow.other", { count: noRow }));
  }
  parts.push(sectionSentence(t, "links", links.state, time(links.at)), sectionSentence(t, "flags", flags.state, time(flags.at)));
  const differs = count > 0;
  return <p className="com-last-download" data-links={links.state} data-flags={flags.state} data-differs={differs ? "true" : undefined}>
    {differs && <span className="com-last-download-dot" aria-hidden="true" />}
    <span>{parts.join(" ")}</span>
  </p>;
}

function Flags({ flags }: { flags: DownloadObjectFlags }) {
  const t = useTranslate();
  const text = flagText(flags, t);
  if (!text.letters) return <>{t("comTable.lastDownload.noFlags")}</>;
  return <span title={text.names}><span aria-hidden="true">{text.letters}</span><span className="sr-only">{text.names}</span></span>;
}

/** The row's differences inside its editor row: added, removed (struck through), redirected, flags. */
export function RowDownloadDetails({ difference, addressStyle }: { difference: DownloadObjectDifference; addressStyle: string }) {
  const t = useTranslate();
  const direction = (sending: boolean) => t(sending ? "inspector.direction.send" : "inspector.direction.receive");
  const flags = difference.flags;
  return <section className="com-object-download-diff" aria-label={t("comTable.lastDownload.heading")}>
    <h4>{t("comTable.lastDownload.heading")}</h4>
    <ul>
      {linkChanges(difference).map(change => {
        const address = rawGroupAddressText(change.address, addressStyle);
        if (change.kind === "added") return <li key={`a${change.address}`} data-link-change="added">
          {t("comTable.lastDownload.linkAdded")} <span className="mono">{address}</span> ({direction(change.sending)})</li>;
        if (change.kind === "removed") return <li key={`r${change.address}`} data-link-change="removed">
          {t("comTable.lastDownload.linkRemoved")} <del className="mono">{address}</del> ({direction(change.sending)})</li>;
        return <li key={`d${change.address}`} data-link-change="redirected">{t("comTable.lastDownload.linkRedirected",
          { address, downloaded: direction(change.downloaded), current: direction(change.current) })}</li>;
      })}
      {flags !== null && flags.downloaded === null && <li data-flag-change="notActiveThen">{t("comTable.lastDownload.notActiveThen")}</li>}
      {flags !== null && flags.downloaded !== null && flags.current === null && <li data-flag-change="notActiveNow">
        {t("comTable.lastDownload.notActiveNow")} {t("comTable.lastDownload.flagsAt")} <Flags flags={flags.downloaded} /></li>}
      {flags !== null && flags.downloaded !== null && flags.current !== null && <li data-flag-change="changed">
        {t("comTable.lastDownload.flagsAt")} <Flags flags={flags.downloaded} /></li>}
    </ul>
  </section>;
}
