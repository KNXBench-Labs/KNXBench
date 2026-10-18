/** Native data tables share one width hook, header handle and local scrolling boundary. */
import { Children, cloneElement, isValidElement, useId, useRef, type HTMLAttributes, type ReactElement, type ReactNode } from "react";
import { useTranslate } from "./i18n";
import { columnMax, useTableColumnWidths, type TableColumnWidth } from "./useTableColumnWidths";

type WidthState = ReturnType<typeof useTableColumnWidths>;
export function ColumnResizeHandle({ column, state, labelId }: {
  column: TableColumnWidth; state: WidthState; labelId: string;
}) {
  const t = useTranslate();
  const drag = useRef<{ pointerId: number; x: number; width: number } | null>(null);
  return <span className="table-column-resizer" role="separator" tabIndex={0}
    aria-orientation="vertical" aria-labelledby={labelId} title={t("tableColumns.help")}
    aria-valuemin={column.min} aria-valuemax={columnMax(column)} aria-valuenow={state.widths[column.id]}
    onClick={event => { event.preventDefault(); event.stopPropagation(); }}
    onDoubleClick={event => { event.preventDefault(); event.stopPropagation(); state.resetColumn(column.id); }}
    onDragStart={event => { event.preventDefault(); event.stopPropagation(); }}
    onPointerDown={event => {
      event.stopPropagation();
      if (event.button !== 0) return;
      event.preventDefault(); event.currentTarget.focus();
      drag.current = { pointerId: event.pointerId, x: event.clientX, width: state.widths[column.id] };
      event.currentTarget.setPointerCapture(event.pointerId);
    }}
    onPointerMove={event => {
      event.stopPropagation();
      if (drag.current?.pointerId === event.pointerId) state.setWidth(column.id, drag.current.width + event.clientX - drag.current.x);
    }}
    onPointerUp={event => {
      event.stopPropagation();
      if (drag.current?.pointerId !== event.pointerId) return;
      drag.current = null;
      event.currentTarget.releasePointerCapture(event.pointerId);
    }}
    onPointerCancel={event => { event.stopPropagation(); drag.current = null; }}
    onLostPointerCapture={() => { drag.current = null; }}
    onKeyDown={event => {
      const step = event.shiftKey ? 40 : 8;
      if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
        state.setWidth(column.id, state.widths[column.id] + (event.key === "ArrowLeft" ? -step : step));
      }
      else if (event.key === "Home") state.setWidth(column.id, column.min);
      else if (event.key === "End") state.setWidth(column.id, columnMax(column));
      else if (event.key === "Enter") state.resetColumn(column.id);
      else return;
      event.preventDefault(); event.stopPropagation();
    }} />;
}

/** Only decorates native thead/tr/th children; body editors and row ownership stay untouched. */
export default function ResizableTable({
  tableId, columns, children, className = "", style, scrollClassName = "", scrollLabel, ...props
}: {
  tableId: string; columns: readonly TableColumnWidth[]; scrollClassName?: string; scrollLabel?: string;
} & HTMLAttributes<HTMLTableElement>) {
  const t = useTranslate(); const state = useTableColumnWidths(tableId, columns); const id = useId();
  let position = 0;
  function headers(nodes: ReactNode): ReactNode {
    return Children.map(nodes, node => {
      if (!isValidElement<{ children?: ReactNode; className?: string }>(node)) return node;
      if (node.type === "caption") return null;
      if (node.type === "th") {
        const column = columns[position++];
        if (!column) return node;
        const labelId = `${id}-${column.id}`;
        return cloneElement(node as ReactElement<HTMLAttributes<HTMLTableCellElement>>, {
          className: `${node.props.className ?? ""} resizable-table-heading`,
        }, <span id={labelId}>{node.props.children}</span>, <ColumnResizeHandle column={column} state={state} labelId={labelId} />);
      }
      if (node.type === "thead" || node.type === "tr") return cloneElement(node, {}, headers(node.props.children));
      return node;
    });
  }
  const decorated = headers(children);
  const totalWidth = columns.reduce((sum, column) => sum + state.widths[column.id], 0);
  return <div className="resizable-table" data-table-id={tableId}>
    <button type="button" className="quiet-action table-column-reset" onClick={state.reset}>{t("tableColumns.reset")}</button>
    <div className={`resizable-table-scroll ${scrollClassName}`} tabIndex={0} role="region" aria-label={scrollLabel ?? t("tableColumns.scroll")}>
      <table {...props} className={`${className} resizable-data-table`} style={{ ...style, width: totalWidth }}>
        {Children.map(children, child => isValidElement(child) && child.type === "caption" ? child : null)}
        <colgroup>{columns.map(column => <col key={column.id} style={{ width: state.widths[column.id] }} />)}</colgroup>
        {decorated}
      </table>
    </div>
  </div>;
}
