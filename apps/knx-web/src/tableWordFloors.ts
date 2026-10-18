/** Measures cached word widths from native table cells for pane-fitting floors. */
import { appUiScale } from "./appUiScale";

const CAP = 320;
const pixels = (value: string) => parseFloat(value) || 0;
const insets = (style: CSSStyleDeclaration) => pixels(style.paddingLeft) + pixels(style.paddingRight)
  + pixels(style.borderLeftWidth) + pixels(style.borderRightWidth);
const typography = ["fontFamily", "fontSize", "fontWeight", "fontStyle", "fontStretch", "fontVariant",
  "letterSpacing", "textTransform"] as const;

/** One cache per mounted table; measurements survive width edits, never enter preferences. */
export class TableWordFloors {
  private cache = new Map<string, number>();
  private cells = new WeakMap<HTMLTableCellElement, { width: number; keys: string[] }>();

  clear() { this.cache.clear(); this.cells = new WeakMap(); }

  /** Invalidate typography/content only where observed records reach native cells. */
  invalidate(records: MutationRecord[], table: HTMLTableElement): boolean {
    let relevant = false;
    for (const record of records) {
      const target = record.target instanceof Element ? record.target : record.target.parentElement;
      if (!target || target.closest("table") !== table
        || target.closest('.sr-only, [role="separator"], .table-sort-indicator')) continue;
      // Mounted hidden rows already participate. The monitor's arrival paint
      // changes background only, so neither can change a word requirement.
      if (record.attributeName === "hidden") continue;
      if (record.attributeName === "class") {
        const typographyClasses = (value: string) => value.split(/\s+/).filter(name => name !== "bus-monitor-row-new").sort().join(" ");
        if (typographyClasses(record.oldValue ?? "") === typographyClasses(target.getAttribute("class") ?? "")) continue;
      }
      relevant = true;
      const cell = target.closest<HTMLTableCellElement>("th, td");
      if (cell) this.cells.delete(cell);
      // A row/section style affects descendants. For childList, new cells have
      // no cached measurement; moved/unchanged rows retain theirs, even on sort.
      else if (record.type === "attributes") {
        for (const descendant of target.querySelectorAll<HTMLTableCellElement>("th, td")) this.cells.delete(descendant);
      }
    }
    return relevant;
  }

  measure(table: HTMLTableElement, count: number): number[] {
    const scale = appUiScale();
    const floors = Array<number>(count).fill(0);
    const used = new Set<string>();
    const pending = new Map<string, HTMLSpanElement>();
    const requirements: { cell: HTMLTableCellElement; key: string; extra: number; minimum: number }[] = [];
    const changed = new Map<HTMLTableCellElement, { width: number; keys: string[]; column: number }>();
    const styles = new Map<Element, CSSStyleDeclaration>();
    const css = (element: Element) => {
      let style = styles.get(element);
      if (!style) { style = getComputedStyle(element); styles.set(element, style); }
      return style;
    };
    for (const row of Array.from(table.rows)) {
      let column = 0;
      for (const cell of Array.from(row.cells)) {
        const index = column; column += cell.colSpan;
        // Group headings and embedded editors span columns: they must not
        // inflate the first column, nor measure a nested table twice.
        if (index >= count || cell.colSpan !== 1) continue;
        const cached = this.cells.get(cell);
        if (cached) {
          floors[index] = Math.max(floors[index], cached.width);
          for (const key of cached.keys) used.add(key);
          continue;
        }
        const record = { width: 0, keys: [] as string[], column: index };
        changed.set(cell, record);
        const walker = document.createTreeWalker(cell, NodeFilter.SHOW_TEXT);
        while (walker.nextNode()) {
          const node = walker.currentNode;
          const parent = node.parentElement!;
          if (parent.closest("table") !== table || parent.closest('.sr-only, [role="separator"]')) continue;
          const tokens = node.textContent?.match(/[^\s\u200b\u00ad]+/gu) ?? [];
          if (!tokens.length) continue;
          const textStyle = css(parent);
          const fontKey = typography.map(property => textStyle[property]).join("|");
          let extra = 0, minimum = 0;
          for (let element: HTMLElement | null = parent; element; element = element.parentElement) {
            const style = css(element);
            const margin = element === cell ? 0 : pixels(style.marginLeft) + pixels(style.marginRight);
            const own = insets(style) + margin;
            extra += own;
            // Fixed-layout table cells ignore CSS min-width; nested controls
            // (e.g. an indented disclosure) still need their own footprint.
            minimum = Math.max(minimum + own, element === cell ? 0
              : pixels(style.minWidth) + margin + (style.boxSizing === "border-box" ? 0 : insets(style)));
            // A nonwrapping flex label shares its line with icons (e.g. sort).
            // Their measured width and gap must remain outside the word budget.
            if (style.display.includes("flex") && style.flexWrap !== "wrap") {
              const branch = element === parent ? null : Array.from(element.children).find(child => child.contains(parent));
              for (const sibling of Array.from(element.children)) {
                if (sibling === branch || !sibling.matches('[aria-hidden="true"]')) continue;
                extra += sibling.getBoundingClientRect().width / scale + pixels(style.columnGap);
              }
            }
            if (element === cell) break;
          }
          for (const token of tokens) {
            const key = `${fontKey}\n${token}`;
            used.add(key);
            if (!this.cache.has(key) && !pending.has(key)) {
              const span = document.createElement("span");
              span.setAttribute("data-table-word-measure", "");
              span.style.cssText = "position:fixed;left:0;top:0;visibility:hidden;white-space:pre;width:max-content;max-width:none;padding:0;border:0;margin:0";
              for (const property of typography) span.style[property] = textStyle[property];
              span.textContent = token;
              pending.set(key, span);
            }
            record.keys.push(key);
            requirements.push({ cell, key, extra, minimum });
          }
        }
      }
    }
    // Batch all writes before any geometry reads. Width-only rerenders reuse
    // the cache; a content change measures only previously unseen font/tokens.
    const fragment = document.createDocumentFragment();
    for (const span of pending.values()) fragment.append(span);
    document.body.append(fragment);
    for (const [key, span] of pending) this.cache.set(key, span.getBoundingClientRect().width / scale);
    for (const span of pending.values()) span.remove();
    for (const { cell, key, extra, minimum } of requirements) {
      const record = changed.get(cell)!;
      record.width = Math.max(record.width, Math.min(CAP, Math.ceil(Math.max(minimum, this.cache.get(key)! + extra))));
    }
    for (const [cell, record] of changed) { this.cells.set(cell, record); floors[record.column] = Math.max(floors[record.column], record.width); }
    for (const key of this.cache.keys()) if (!used.has(key)) this.cache.delete(key);
    return floors;
  }
}
