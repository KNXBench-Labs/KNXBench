/** Native data tables share one width hook, header handle and local scrolling boundary. */
import { Fragment, Children, cloneElement, isValidElement, useEffect, useId, useLayoutEffect, useRef, useState, type HTMLAttributes, type ReactElement, type ReactNode } from "react";
import { useTranslate } from "./i18n";
import { columnMax, useTableColumnWidths, type TableColumnWidth } from "./useTableColumnWidths";

type WidthState = ReturnType<typeof useTableColumnWidths>;
export function ColumnResizeHandle({ column, state, labelId, helpId, renderedWidth }: {
  column: TableColumnWidth; state: WidthState; labelId: string; helpId: string; renderedWidth: number;
}) {
  const t = useTranslate();
  const drag = useRef<{ pointerId: number; x: number; width: number; next: number } | null>(null);
  const frame = useRef<number | null>(null);
  const cancelFrame = () => { if (frame.current !== null) cancelAnimationFrame(frame.current); frame.current = null; };
  const latestFinish = useRef(() => {});
  useEffect(() => () => latestFinish.current(), []);
  function finish(cancel = false) {
    const current = drag.current;
    if (!current) return;
    cancelFrame(); drag.current = null;
    state.setWidth(column.id, cancel ? current.width : current.next, !cancel);
  }
  latestFinish.current = () => { finish(); cancelFrame(); };
  return <span className="table-column-resizer" role="separator" tabIndex={0}
    aria-orientation="vertical" aria-labelledby={labelId} aria-describedby={helpId} aria-valuetext={`${renderedWidth} px`} title={t("tableColumns.help")}
    aria-valuemin={Math.min(column.min, renderedWidth)} aria-valuemax={Math.max(columnMax(column), renderedWidth)} aria-valuenow={renderedWidth}
    onClick={event => { event.preventDefault(); event.stopPropagation(); }}
    onDoubleClick={event => { event.preventDefault(); event.stopPropagation(); state.resetColumn(column.id); }}
    onDragStart={event => { event.preventDefault(); event.stopPropagation(); }}
    onPointerDown={event => {
      event.stopPropagation();
      if (event.button !== 0) return;
      event.preventDefault(); event.currentTarget.focus();
      drag.current = { pointerId: event.pointerId, x: event.clientX, width: state.widths[column.id], next: state.widths[column.id] };
      event.currentTarget.setPointerCapture(event.pointerId);
    }}
    onPointerMove={event => {
      event.stopPropagation();
      if (drag.current?.pointerId !== event.pointerId) return;
      drag.current.next = drag.current.width + event.clientX - drag.current.x;
      if (frame.current === null) frame.current = requestAnimationFrame(() => {
        frame.current = null;
        if (drag.current) state.setWidth(column.id, drag.current.next, false);
      });
    }}
    onPointerUp={event => {
      event.stopPropagation();
      if (drag.current?.pointerId !== event.pointerId) return;
      finish();
      event.currentTarget.releasePointerCapture(event.pointerId);
    }}
    onPointerCancel={event => { event.stopPropagation(); if (drag.current?.pointerId === event.pointerId) finish(); }}
    onLostPointerCapture={event => { if (drag.current?.pointerId === event.pointerId) finish(); }}
    onKeyDown={event => {
      if (drag.current && ["ArrowLeft", "ArrowRight", "Home", "End", "Enter"].includes(event.key)) {
        event.preventDefault(); event.stopPropagation(); return;
      }
      const step = event.shiftKey ? 40 : 8;
      if (event.key === "Escape" && drag.current) {
        const pointerId = drag.current.pointerId; finish(true);
        event.currentTarget.releasePointerCapture(pointerId);
      }
      else if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
        state.setWidth(column.id, state.widths[column.id] + (event.key === "ArrowLeft" ? -step : step));
      }
      else if (event.key === "Home") state.setWidth(column.id, column.min);
      else if (event.key === "End") state.setWidth(column.id, columnMax(column));
      else if (event.key === "Enter") state.resetColumn(column.id);
      else return;
      event.preventDefault(); event.stopPropagation();
    }} />;
}

function headingText(nodes: ReactNode): string {
  return Children.toArray(nodes).map(node => typeof node === "string" || typeof node === "number" ? String(node)
    : isValidElement<{ children?: ReactNode }>(node) ? headingText(node.props.children) : "").join("");
}

/** Only decorates native thead/tr/th children; body editors and row ownership stay untouched. */
export default function ResizableTable({
  tableId, columns, children, className = "", style, scrollClassName = "", scrollLabel, fixedTrailingColumns = 0, fixedTrailingWidth = 0, ...props
}: {
  tableId: string; columns: readonly TableColumnWidth[]; scrollClassName?: string; scrollLabel?: string; fixedTrailingColumns?: number; fixedTrailingWidth?: number;
} & HTMLAttributes<HTMLTableElement>) {
  const t = useTranslate(); const state = useTableColumnWidths(tableId, columns); const id = useId();
  const viewport = useRef<HTMLDivElement>(null);
  const [overflowing, setOverflowing] = useState(false);
  const [availableWidth, setAvailableWidth] = useState(0);
  const [measuredWidths, setMeasuredWidths] = useState<number[]>([]);
  const measure = () => {
    const element = viewport.current;
    if (!element) return;
    setAvailableWidth(element.clientWidth);
    setOverflowing(element.scrollWidth > element.clientWidth || element.scrollHeight > element.clientHeight);
    const next = Array.from(element.querySelectorAll(":scope > table > thead > tr > th"), heading => Math.round(heading.getBoundingClientRect().width));
    setMeasuredWidths(previous => previous.length === next.length && previous.every((width, index) => width === next[index]) ? previous : next);
  };
  // Measure after every column edit as well as external pane/table resizes.
  useLayoutEffect(measure);
  useLayoutEffect(() => {
    const element = viewport.current;
    if (!element) return;
    const observer = new ResizeObserver(measure);
    observer.observe(element); if (element.firstElementChild) observer.observe(element.firstElementChild);
    return () => observer.disconnect();
  }, []);
  let position = 0;
  function headers(nodes: ReactNode): ReactNode {
    return Children.map(nodes, node => {
      if (!isValidElement<{ children?: ReactNode; className?: string; style?: HTMLAttributes<HTMLTableCellElement>["style"] }>(node)) return node;
      if (node.type === "caption") return null;
      if (node.type === "th") {
        const column = columns[position++];
        if (!column) return node;
        const labelId = `${id}-${column.id}`;
        return cloneElement(node as ReactElement<HTMLAttributes<HTMLTableCellElement>>, {
          className: `${node.props.className ?? ""} resizable-table-heading`,
          style: { ...node.props.style, zIndex: columns.length + fixedTrailingColumns - position + 1 },
        }, <span id={labelId} title={headingText(node.props.children)}>{node.props.children}</span>, <ColumnResizeHandle column={column} state={state} labelId={labelId} helpId={`${id}-help`} renderedWidth={measuredWidths[position - 1] || fittedWidths[position - 1]} />);
      }
      if (node.type === "thead" || node.type === "tr" || node.type === Fragment) return cloneElement(node, {}, headers(node.props.children));
      return node;
    });
  }
  const defaultWidth = columns.reduce((sum, column) => sum + column.width, 0);
  // Fit defaults to the actual pane, including navigation/inspector space.
  // User adjustments are pixel deltas on that baseline; only their surplus
  // creates overflow. Fitting is off when callers supply fixed trailing columns.
  // Two adjacent 24 px pointer targets must not cover a header's sort button.
  const headerMinimum = (column: TableColumnWidth) => column.fitMin ?? Math.min(48, column.min);
  const minimumWidth = columns.reduce((sum, column) => sum + headerMinimum(column), 0);
  const fitBudget = availableWidth > 0 && fixedTrailingColumns === 0
    ? Math.max(minimumWidth, Math.min(defaultWidth, availableWidth)) : defaultWidth;
  const scale = defaultWidth > minimumWidth ? (fitBudget - minimumWidth) / (defaultWidth - minimumWidth) : 1;
  const fittedWidths = columns.map(column => Math.max(Math.min(48, column.min),
    headerMinimum(column) + Math.floor((column.width - headerMinimum(column)) * scale)
      + state.widths[column.id] - column.width));
  const totalWidth = fittedWidths.reduce((sum, width) => sum + width, 0) + fixedTrailingColumns * fixedTrailingWidth;
  const decorated = headers(children);
  useEffect(() => {
    if (import.meta.env.DEV && position !== columns.length + fixedTrailingColumns)
      console.warn(`ResizableTable ${tableId}: heading/column count mismatch`, position, columns.length + fixedTrailingColumns);
  }, [position, columns.length, fixedTrailingColumns, tableId]);
  const caption = Children.toArray(children).find(child => isValidElement(child) && child.type === "caption");
  const captionText = isValidElement<{ children?: ReactNode }>(caption) && typeof caption.props.children === "string" ? caption.props.children : undefined;
  const tableName = scrollLabel ?? captionText ?? t(`tableColumns.name.${tableId}` as Parameters<typeof t>[0]);
  return <div className="resizable-table" data-table-id={tableId}>
    <span id={`${id}-help`} className="sr-only">{t("tableColumns.help")}</span>
    <button type="button" className="quiet-action table-column-reset" aria-label={`${t("tableColumns.reset")}: ${tableName}`} onClick={state.reset}>{t("tableColumns.reset")}</button>
    <div ref={viewport} className={`resizable-table-scroll ${scrollClassName}`} tabIndex={scrollLabel || overflowing ? 0 : undefined} role={scrollLabel || overflowing ? "region" : undefined} aria-label={scrollLabel || overflowing ? tableName : undefined}>
      <table {...props} className={`${className} resizable-data-table`} style={{ ...style, width: totalWidth }}>
        {Children.map(children, child => isValidElement(child) && child.type === "caption" ? child : null)}
        <colgroup>{columns.map((column, index) => <col key={column.id} style={{ width: fittedWidths[index] }} />)}{Array.from({ length: fixedTrailingColumns }, (_, index) => <col key={`fixed-${index}`} style={{ width: fixedTrailingWidth }} />)}</colgroup>
        {decorated}
      </table>
    </div>
  </div>;
}
