/** Shared decoded comparison table, including visible, lossless raw-memory rows. */
import ResizableTable from "./ResizableTable";
import type { DecodedComparisonRow } from "./deviceComparisons";
import { useTranslate, type MessageKey } from "./i18n";

const DEVICE_COMPARISON_COLUMNS = [
  { id: "field", width: 240, min: 120 },
  { id: "project", width: 200, min: 100 },
  { id: "device", width: 200, min: 100 },
  { id: "location", width: 120, min: 80 }
];

const labels: Record<string, MessageKey> = {
  addressCount: "compare.kind.addressCount", individualAddress: "compare.kind.individualAddress",
  groupAddress: "compare.kind.groupAddress", associationCount: "compare.kind.associationCount",
  association: "compare.kind.association", raw: "compare.kind.raw", parameter: "compare.kind.parameter",
};
export default function ComparisonTable({ rows }: { rows: readonly DecodedComparisonRow[] }) {
  const t = useTranslate();
  return <ResizableTable scrollClassName="comparison-table-scroll" scrollLabel={t("compare.decoded")} tableId="device-comparison"
    columns={DEVICE_COMPARISON_COLUMNS} className="workspace-table comparison-decoded"><thead><tr>
      <th>{t("compare.field")}</th><th>{t("compare.projectValue")}</th><th>{t("compare.deviceValue")}</th><th>{t("compare.location")}</th>
    </tr></thead><tbody>{rows.map((row, index) => <tr key={index} className={row.kind === "raw" ? "comparison-raw" : undefined}
      data-different={row.project !== null && row.device !== null && row.project !== row.device || undefined}>
      <th scope="row">{labels[row.kind] ? t(labels[row.kind]) : row.kind}{row.label && ` · ${row.label}`}
        {row.reason && <small className="comparison-reason">{row.reason === "parameterNotDecoded" ? t("compare.parameterNotDecoded") : row.reason}</small>}</th>
      <td>{row.project ?? t("compare.unknown")}</td><td>{row.device ?? t("compare.unknown")}</td>
      <td className="mono">{row.address.toString(16).toUpperCase().padStart(4, "0")}h</td>
    </tr>)}</tbody></ResizableTable>;
}
