/** Shared decoded comparison table, including visible, lossless raw-memory rows. */
import type { DecodedComparisonRow } from "./deviceComparisons";
import { useTranslate, type MessageKey } from "./i18n";

const labels: Record<string, MessageKey> = {
  addressCount: "compare.kind.addressCount", individualAddress: "compare.kind.individualAddress",
  groupAddress: "compare.kind.groupAddress", associationCount: "compare.kind.associationCount",
  association: "compare.kind.association", raw: "compare.kind.raw", parameter: "compare.kind.parameter",
};
export default function ComparisonTable({ rows }: { rows: readonly DecodedComparisonRow[] }) {
  const t = useTranslate();
  return <div className="comparison-table-scroll" role="region" aria-label={t("compare.decoded")} tabIndex={0}>
    <table className="workspace-table comparison-decoded"><thead><tr>
      <th>{t("compare.field")}</th><th>{t("compare.projectValue")}</th><th>{t("compare.deviceValue")}</th><th>{t("compare.location")}</th>
    </tr></thead><tbody>{rows.map((row, index) => <tr key={index} className={row.kind === "raw" ? "comparison-raw" : undefined}
      data-different={row.project !== null && row.device !== null && row.project !== row.device || undefined}>
      <th scope="row">{labels[row.kind] ? t(labels[row.kind]) : row.kind}{row.label && ` · ${row.label}`}
        {row.reason && <small className="comparison-reason">{row.reason}</small>}</th>
      <td>{row.project ?? t("compare.unknown")}</td><td>{row.device ?? t("compare.unknown")}</td>
      <td className="mono">{row.address.toString(16).toUpperCase().padStart(4, "0")}h</td>
    </tr>)}</tbody></table>
  </div>;
}
