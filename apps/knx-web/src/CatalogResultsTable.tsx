/** Native sortable, resizable catalogue results table with a column chooser. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { useLayoutEffect, useRef } from "react";
import type { CatalogItem } from "./api";
import { useTranslate } from "./i18n";
import { LanguageFallbackBadge, fellBack } from "./languageFallback";
import CatalogItemIdentity from "./CatalogItemIdentity";
import ResizableTable from "./ResizableTable";
import TableColumnChooser from "./TableColumnChooser";
import { CATALOG_TABLE_COLUMNS, visibleColumns } from "./tableColumns";
import { useTableColumnVisibility } from "./useTableColumnVisibility";
import {
  applicationLabel, manufacturerLabel, maskLabel, mediumLabel, nextCatalogSort, orderNumberLabel,
  productLabel, sectionLabel, type CatalogSort,
} from "./catalogTableSort";
import "./catalogTable.css";

export const CATALOG_TABLE_ID = "catalog-results";
export const catalogRowId = (index: number) => `catalog-row-${index}`;

export default function CatalogResultsTable(props: {
  /** Already in visible order; this component never reorders. */
  rows: readonly CatalogItem[];
  sort: CatalogSort;
  onSort: (sort: CatalogSort) => void;
  manufacturerNames: ReadonlyMap<string, string>;
  /** Product language selected for catalogue text, `null` when none. */
  language: string | null;
  highlightIndex: number;
  selectedId: string | null;
  disabled: boolean;
  /** The workspace is on screen; a hidden one must not scroll. */
  active: boolean;
  onPick: (item: CatalogItem) => void;
}) {
  const { rows, sort, onSort, manufacturerNames, language, highlightIndex, selectedId, disabled, active, onPick } = props;
  const t = useTranslate();
  const scroller = useRef<HTMLDivElement>(null);
  const activeRow = useRef<HTMLTableRowElement>(null);
  const { visible } = useTableColumnVisibility("catalog-results", CATALOG_TABLE_COLUMNS.map((column) => column.id));
  const columns = visibleColumns(CATALOG_TABLE_COLUMNS, visible);
  // Keep the keyboard-highlighted row visible vertically; a row wider than the scroller
  // must not drag the table sideways, so the user's horizontal position is restored.
  useLayoutEffect(() => {
    if (!active) return;
    const left = scroller.current?.scrollLeft;
    activeRow.current?.scrollIntoView?.({ block: "nearest", inline: "nearest" });
    if (scroller.current && left !== undefined) scroller.current.scrollLeft = left;
  }, [highlightIndex, rows, active]);
  const dash = "—";
  const cellText = (item: CatalogItem, id: string): string => {
    switch (id) {
      case "manufacturer": return manufacturerLabel(item, manufacturerNames);
      case "number": return orderNumberLabel(item) || dash;
      case "application": return applicationLabel(item) || dash;
      case "section": return sectionLabel(item) || dash;
      case "mask": return maskLabel(item) || dash;
      case "medium": return mediumLabel(item) || dash;
      default: return "";
    }
  };
  return <div className="catalog-table-block">
    <div className="catalog-table-toolbar">
      <TableColumnChooser tableId="catalog-results" tableLabel="tableColumns.name.catalog-results"
        columns={CATALOG_TABLE_COLUMNS.map((column) => ({ id: column.id, label: column.label }))} />
    </div>
    <div ref={scroller} className="catalog-table-scroll" tabIndex={0}>
      <ResizableTable tableId="catalog-results" columns={columns} showReset={false} className="catalog-table"
        scrollLabel={t("catalogTable.label")}>
        <thead><tr>{columns.map(({ sort: column, label }) => {
          const active = sort.column === column;
          return <th key={column} scope="col" data-column={column} aria-sort={active ? (sort.descending ? "descending" : "ascending") : "none"}>
            <button type="button" className="catalog-sort" onClick={() => onSort(nextCatalogSort(sort, column))}>
              <span>{t(label)}</span>
              <span className="catalog-sort-indicator" aria-hidden="true">{active ? (sort.descending ? "↓" : "↑") : ""}</span>
            </button>
          </th>;
        })}</tr></thead>
        <tbody>{rows.map((item, i) => {
          const selected = selectedId === item.id;
          return <tr key={item.id} id={catalogRowId(i)} data-item-id={item.id}
            ref={i === highlightIndex ? activeRow : undefined}
            className={`search-result${selected ? " selected" : ""}${i === highlightIndex ? " active" : ""}${disabled ? " disabled" : ""}`}
            onClick={() => onPick(item)}>
            {columns.map((column) => column.id === "product"
              ? <td key={column.id} className="catalog-product-cell">
                <button type="button" className="catalog-select" aria-pressed={selected} disabled={disabled}>{productLabel(item)}</button>
                {language !== null && fellBack(item.name, item.nameLanguage) && (
                  <LanguageFallbackBadge selected={language} source={item.sourceLanguage} />
                )}
                {item.visibleDescription && <span className="catalog-description">{item.visibleDescription}</span>}
                {language !== null && fellBack(item.visibleDescription, item.visibleDescriptionLanguage) && (
                  <LanguageFallbackBadge selected={language} source={item.sourceLanguage} />
                )}
                <CatalogItemIdentity item={item} />
              </td>
              : <td key={column.id}>{cellText(item, column.id)}</td>)}
          </tr>;
        })}</tbody>
      </ResizableTable>
    </div>
  </div>;
}
