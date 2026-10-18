/** Shared mounted-surface checks keep column schemas aligned with their actual native headers. */
import { expect, it } from "vitest";

type Mounted = { host: ParentNode; cleanup?: () => Promise<void> };
export function tableColumnSmoke(
  cases: readonly (readonly [id: string, columns: number])[],
  mount: (id: string) => Promise<Mounted>,
) {
  it.each(cases)("column smoke: %s exposes %i resize separators", async (id, count) => {
    const mounted = await mount(id);
    try {
      const table = mounted.host.querySelector(`[data-table-id="${id}"]`);
      expect(table, `mount the production ${id} surface`).not.toBeNull();
      expect(table!.querySelectorAll('[role="separator"]')).toHaveLength(count);
      const headings = table!.querySelectorAll("thead th");
      expect(headings.length).toBe(table!.querySelectorAll("colgroup col").length);
      for (const heading of [...headings].slice(0, count)) expect(heading.querySelector('[role="separator"]')).not.toBeNull();
    } finally { await mounted.cleanup?.(); }
  });
}
