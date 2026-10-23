/** Shared comparison table: marked raw bytes, differences-only view and toggle. */
import { Fragment, useState } from "react";
import ResizableTable from "./ResizableTable";
import type { DecodedComparisonRow } from "./deviceComparisons";
import { comparisonRowView, markRawBytes, type ComparisonChange, type MarkedByte } from "./comparisonDiff";
import { useTranslate, type MessageKey, type Translate } from "./i18n";

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

function Bytes({ bytes, t }: { bytes: MarkedByte[] | null; t: Translate }) {
  if (bytes === null) return <>{t("compare.unknown")}</>;
  return <>{bytes.map((byte, index) => <Fragment key={index}>{index > 0 && " "}<span className="comparison-byte" data-mark={byte.mark}
    title={byte.mark === "different" ? t("compare.byteDifferent") : byte.mark === "unknown" ? t("compare.byteUnknown") : undefined}>{byte.text}</span></Fragment>)}</>;
}

/** The response is never changed: the view only chooses which original rows to show. */
export default function ComparisonTable({ rows, changes = [], selection }: { rows: readonly DecodedComparisonRow[]; changes?: readonly ComparisonChange[];
  selection?: { indices: readonly number[]; disabled: boolean; toggle: (index: number) => void } }) {
  const t = useTranslate();
  const [showAll, setShowAll] = useState(false);
  const view = comparisonRowView(rows, changes, showAll);
  return <>
    <div className="comparison-filter-bar">
      <label className="comparison-filter"><input type="checkbox" checked={showAll}
        onChange={(event) => setShowAll(event.target.checked)} />{t("compare.showEqual", { count: view.equal })}</label>
      {view.hidden > 0 && <p className="comparison-hidden">{t("compare.hiddenEqual", { count: view.hidden })}</p>}
      {view.hidden > 0 && view.visible.length === 0 && <p className="comparison-no-differences">{t("compare.noDifferences")}</p>}
    </div>
    <ResizableTable scrollClassName="comparison-table-scroll" scrollLabel={t("compare.decoded")} tableId="device-comparison"
      columns={DEVICE_COMPARISON_COLUMNS} className="workspace-table comparison-decoded"><thead><tr>
        <th>{t("compare.field")}</th><th>{t("compare.projectValue")}</th><th>{t("compare.deviceValue")}</th><th>{t("compare.location")}</th>
      </tr></thead><tbody>{view.visible.map(({ row, index }) => {
        const marks = row.kind === "raw" ? markRawBytes(row, changes) : null;
        const different = marks ? marks.different > 0 : row.project !== null && row.device !== null && row.project !== row.device;
        return <tr key={index} className={marks ? "comparison-raw" : undefined}
          data-different={different || undefined} data-unknown={(marks !== null && marks.unknown > 0) || undefined}>
          <th scope="row">{selection && row.adoptable === true && row.index === index && <input type="checkbox"
            aria-label={t("compare.adoption.select", { field: row.label })} checked={selection.indices.includes(index)}
            disabled={selection.disabled} onChange={() => selection.toggle(index)} />}
            {labels[row.kind] ? t(labels[row.kind]) : row.kind}{row.label && ` · ${row.label}`}
            {row.reason && <small className="comparison-reason">{row.reason === "parameterNotDecoded" ? t("compare.parameterNotDecoded") : row.reason}</small>}
            {marks && marks.different > 0 && <small className="comparison-byte-count">{t("compare.bytesDifferent", { count: marks.different })}</small>}
            {marks && marks.unknown > 0 && <small className="comparison-byte-count">{t("compare.bytesUnknown", { count: marks.unknown })}</small>}</th>
          {marks ? <><td><Bytes bytes={marks.project} t={t} /></td><td><Bytes bytes={marks.device} t={t} /></td></>
            : <><td>{row.project ?? t("compare.unknown")}</td><td>{row.device ?? t("compare.unknown")}</td></>}
          <td className="mono">{row.address.toString(16).toUpperCase().padStart(4, "0")}h</td>
        </tr>;
      })}</tbody></ResizableTable>
  </>;
}
