/** "Spalten" flyout: per-table checkbox list over the table's declared columns. */
import { useEffect, useRef, useState } from "react";
import { useTranslate, type MessageKey } from "./i18n";
import { useTableColumnVisibility } from "./useTableColumnVisibility";

/**
 * One `<details>` menu whose checkboxes toggle the columns a table renders.
 * `columns` is the full declared set in display order; `tableId` names the
 * persisted set. Hidden columns keep their stored width, so re-showing one
 * restores the width the user last dragged it to.
 */
export default function TableColumnChooser(props: {
  tableId: string;
  columns: readonly { id: string; label: MessageKey }[];
  /** Screen-reader name of the table the menu configures. */
  tableLabel: MessageKey;
  className?: string;
}) {
  const { tableId, columns, tableLabel, className } = props;
  const t = useTranslate();
  const menu = useRef<HTMLDetailsElement>(null);
  const [open, setOpen] = useState(false);
  const [align, setAlign] = useState<"start" | "end">("start");
  const { visible, setVisible, reset } = useTableColumnVisibility(tableId, columns.map((column) => column.id));
  useEffect(() => {
    if (!open) return;
    const outside = (event: PointerEvent) => { if (!menu.current?.contains(event.target as Node)) menu.current!.open = false; };
    document.addEventListener("pointerdown", outside);
    return () => document.removeEventListener("pointerdown", outside);
  }, [open]);
  const close = () => { if (menu.current) menu.current.open = false; };
  return <details ref={menu} className={`table-column-chooser ${className ?? ""}`} onToggle={() => {
    const details = menu.current; if (!details) return;
    setOpen(details.open);
    if (!details.open) return;
    const page = (details.closest("section, main") ?? document.documentElement).getBoundingClientRect();
    const summary = details.querySelector("summary")!.getBoundingClientRect();
    const width = details.querySelector<HTMLElement>(".table-column-chooser-content")?.offsetWidth ?? 0;
    setAlign(summary.left + width <= page.right ? "start" : "end");
  }} onKeyDown={(event) => {
    if (event.key === "Escape" && menu.current?.open) { event.stopPropagation(); close(); menu.current?.querySelector("summary")?.focus(); }
  }}>
    <summary aria-label={t("tableColumns.chooseNamed", { table: t(tableLabel) })}>{t("tableColumns.choose")}</summary>
    <div className="table-column-chooser-content" data-align={align} role="group" aria-label={t("tableColumns.chooseNamed", { table: t(tableLabel) })}>
      {columns.map((column) => <label key={column.id} className="table-column-chooser-item">
        <input type="checkbox" checked={visible[column.id] !== false}
          onChange={(event) => setVisible(column.id, event.target.checked)} />
        {t(column.label)}
      </label>)}
      <button type="button" className="quiet-action table-column-chooser-reset" onClick={reset}>{t("tableColumns.chooseReset")}</button>
    </div>
  </details>;
}
