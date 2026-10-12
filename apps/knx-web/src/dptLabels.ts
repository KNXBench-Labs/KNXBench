/** Versioned DPT labels without reference rewrites or codec claims. */
import catalogue from "./dpt-labels-v02.02.01.json";
export const DPT_LABEL_VERSION = catalogue.version;
export const DPT_OPTIONS = catalogue.entries.map(entry => {
  const [main, sub] = entry.number.split(".").map(Number);
  return { ...entry, id: `DPST-${main}-${sub}` };
});
const byId = new Map(DPT_OPTIONS.map(entry => [entry.id, entry]));
/** Display lookup only: accepted aliases stay byte-for-byte values in the editor. */
export function dptEntry(raw: string | null) {
  const match = raw?.match(/^DPST-(\d+)-(\d+)$/);
  if (!match) return undefined;
  const parts = match.slice(1).map(Number);
  if (!parts.every(Number.isSafeInteger)) return undefined;
  return byId.get(`DPST-${parts[0]}-${parts[1]}`);
}
export function dptDisplay(raw: string | null, language: string): string {
  const entry = dptEntry(raw);
  if (entry) return `${language === "de" ? entry.de : entry.en} (${entry.number})`;
  // Unlisted valid references have a number, but no verified name. Main-only
  // references are families, never silently promoted to subtype .001.
  const subtype = raw?.match(/^DPST-(\d+)-(\d+)$/);
  if (subtype && subtype.slice(1).every(part => Number.isSafeInteger(Number(part))))
    return `${Number(subtype[1])}.${String(Number(subtype[2])).padStart(3, "0")}`;
  const main = raw?.match(/^DPT-(\d+)$/);
  if (main && Number.isSafeInteger(Number(main[1]))) return `${Number(main[1])}.*`;
  return raw ?? "—";
}
/** Extra stored/draft options retain unlisted and noncanonical references, never replacing them. */
export function retainedDptOptions(current: string, draft: string): string[] {
  return [...new Set([current, draft])].filter(raw => raw !== "" && !byId.has(raw));
}
