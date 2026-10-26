/** Native sortable table of catalogue results; selection stays a per-row button. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { useLayoutEffect, useRef } from "react";
import type { CatalogItem } from "./api";
import { useTranslate } from "./i18n";
import { LanguageFallbackBadge, fellBack } from "./languageFallback";
import CatalogItemIdentity from "./CatalogItemIdentity";
import {
  applicationLabel, manufacturerLabel, nextCatalogSort, orderNumberLabel, productLabel, type CatalogSort, type CatalogSortColumn,
} from "./catalogTableSort";
import "./catalogTable.css";

export const CATALOG_TABLE_ID = "catalog-results";
export const catalogRowId = (index: number) => `catalog-row-${index}`;

const COLUMNS = [
  { column: "product", label: "catalogTable.product" },
  { column: "manufacturer", label: "catalogTable.manufacturer" },
  { column: "number", label: "catalogTable.number" },
  { column: "application", label: "catalogTable.application" },
] as const satisfies readonly { column: CatalogSortColumn; label: string }[];

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
  // Keep the keyboard-highlighted row visible vertically; a row wider than the scroller
  // must not drag the table sideways, so the user's horizontal position is restored.
  useLayoutEffect(() => {
    if (!active) return;
    const left = scroller.current?.scrollLeft;
    activeRow.current?.scrollIntoView?.({ block: "nearest", inline: "nearest" });
    if (scroller.current && left !== undefined) scroller.current.scrollLeft = left;
  }, [highlightIndex, rows, active]);
  const dash = "—";
  return <div ref={scroller} className="catalog-table-scroll" role="region" tabIndex={0} aria-label={t("catalogTable.label")}>
    <table id={CATALOG_TABLE_ID} className="catalog-table">
      <thead><tr>{COLUMNS.map(({ column, label }) => {
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
          <td className="catalog-product-cell">
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
          <td>{manufacturerLabel(item, manufacturerNames)}</td>
          <td>{orderNumberLabel(item) || dash}</td>
          <td>{applicationLabel(item) || dash}</td>
        </tr>;
      })}</tbody>
    </table>
  </div>;
}
