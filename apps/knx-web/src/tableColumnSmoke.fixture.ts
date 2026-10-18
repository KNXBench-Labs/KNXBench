/** Shared mounted-surface checks keep column schemas aligned with their actual native headers. */
import { act } from "react";
import { expect, it } from "vitest";

type Mounted = { host: ParentNode; cleanup?: () => Promise<void> };
export function tableColumnSmoke(
  cases: readonly (readonly [id: string, columns: number])[],
  mount: (id: string) => Promise<Mounted>,
) {
  it.each(cases)("column smoke: %s exposes %i resize separators", async (id, count) => {
    const actEnvironment = globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT?: boolean };
    const environment = actEnvironment.IS_REACT_ACT_ENVIRONMENT;
    actEnvironment.IS_REACT_ACT_ENVIRONMENT = true;
    const mounted = await mount(id);
    try {
      const table = mounted.host.querySelector(`[data-table-id="${id}"]`);
      expect(table, `mount the production ${id} surface`).not.toBeNull();
      expect(table!.querySelectorAll('[role="separator"]')).toHaveLength(count);
      const headings = table!.querySelectorAll("thead th");
      expect(headings.length).toBe(table!.querySelectorAll("colgroup col").length);
      for (const heading of [...headings].slice(0, count)) {
        const separator = heading.querySelector<HTMLElement>('[role="separator"]')!;
        expect(separator).not.toBeNull();
        const before = Number(separator.getAttribute("aria-valuenow"));
        await act(async () => separator.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowLeft", bubbles: true })));
        expect(Number(separator.getAttribute("aria-valuenow")), `${id} heading can shrink`).toBeLessThan(before);
      }
    } finally { await mounted.cleanup?.(); actEnvironment.IS_REACT_ACT_ENVIRONMENT = environment; }
  });
}
