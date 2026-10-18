/** Independently measures browser min-content cells without component columns or width state. */
import { expect, type Locator } from "@playwright/test";

export async function intrinsicFloors(table: Locator, retained: number[] = []) {
  return table.locator("table").evaluate((source: HTMLTableElement, retained) => {
    const count = source.tHead!.rows[0].cells.length;
    const floors = Array.from({ length: count }, (_, i) => retained[i] ?? (i === 0 && source.classList.contains("devices-table") ? 32 : 48));
    // Copy presentation into isolated, auto-layout single-cell tables. The
    // browser resolves intrinsic words, flex icons, nested control minima and
    // padding; neither colgroup nor the adapter's inline widths participate.
    const properties = ["font-family", "font-size", "font-weight", "font-style", "font-stretch", "font-variant",
      "letter-spacing", "text-transform", "padding-top", "padding-right", "padding-bottom", "padding-left",
      "border-left-width", "border-left-style", "border-right-width", "border-right-style",
      "box-sizing", "margin-left", "margin-right", "display", "gap",
      "flex-direction", "flex-wrap", "flex-shrink", "min-width"];
    const sandbox = document.createElement("div");
    sandbox.style.cssText = "position:fixed;visibility:hidden;left:0;top:0;contain:layout style;";
    document.body.append(sandbox);
    try {
      for (const row of Array.from(source.rows)) {
        let index = 0;
        for (const cell of Array.from(row.cells)) {
          const column = index; index += cell.colSpan;
          if (cell.colSpan !== 1 || column >= count) continue;
          const copy = cell.cloneNode(true) as HTMLTableCellElement;
          const originals = [cell, ...Array.from(cell.querySelectorAll("*"))];
          const copies = [copy, ...Array.from(copy.querySelectorAll("*"))];
          for (let i = 0; i < originals.length; i++) {
            const original = originals[i]; const clone = copies[i] as HTMLElement;
            const style = getComputedStyle(original);
            clone.removeAttribute("class"); clone.removeAttribute("id"); clone.removeAttribute("style");
            for (const property of properties) clone.style.setProperty(property, style.getPropertyValue(property));
            clone.style.whiteSpace = "normal"; clone.style.overflowWrap = "normal"; clone.style.wordBreak = "normal";
            if (original.matches('.table-column-label')) clone.style.display = "block";
            if (original.matches('[aria-hidden="true"]')) clone.style.width = style.width;
          }
          copy.querySelectorAll('.sr-only, [role="separator"]').forEach(e => e.remove());
          // Classes were removed above: use the original/copy pairing for noise.
          originals.forEach((e, i) => { if (e.matches('.sr-only, [role="separator"]')) copies[i].remove(); });
          // KNX/DPT tokens include punctuation: browser hyphen/slash break
          // opportunities must not make the independent word oracle too small.
          const text = document.createTreeWalker(copy, NodeFilter.SHOW_TEXT);
          const nodes: Text[] = [];
          while (text.nextNode()) nodes.push(text.currentNode as Text);
          for (const node of nodes) {
            const fragment = document.createDocumentFragment();
            for (const part of node.data.split(/(\s+)/u)) {
              if (!part.trim()) fragment.append(part);
              else { const token = document.createElement("span"); token.style.whiteSpace = "nowrap"; token.textContent = part; fragment.append(token); }
            }
            node.replaceWith(fragment);
          }
          copy.style.minWidth = "0"; copy.style.width = "auto";
          const probe = document.createElement("table");
          probe.style.cssText = "table-layout:auto;width:min-content;min-width:0;border-collapse:separate;border-spacing:0";
          const probeRow = probe.insertRow(); probeRow.append(copy); sandbox.append(probe);
          floors[column] = Math.max(floors[column], Math.min(320, Math.ceil(copy.getBoundingClientRect().width)));
          probe.remove();
        }
      }
      return floors;
    } finally { sandbox.remove(); }
  }, retained);
}

export async function assertFloorOverflow(table: Locator, floors: number[], reviewedTotal: number) {
  const actual = await table.locator("thead th").evaluateAll(cells => cells.map(e => e.getBoundingClientRect().width));
  expect(actual).toHaveLength(floors.length);
  // Native min-content includes a text-free checkbox footprint (44 px),
  // whereas the declared selection column uses 32 px. Text/control branches
  // can also share insets which the production token method sums separately.
  // Bound those differences AND pin the independently reviewed fixture total:
  // even small inflation of an already overflowing floor must fail.
  actual.forEach((width, i) => expect(Math.abs(width - floors[i]), `column ${i}: rendered ${width}, intrinsic ${floors[i]}`)
    .toBeLessThanOrEqual(i === 0 && floors.length === 12 ? 12 : 8));
  expect(Math.abs(actual.reduce((a, b) => a + b, 0) - reviewedTotal), "reviewed compact Porcelain fixture budget").toBeLessThanOrEqual(2);
}
