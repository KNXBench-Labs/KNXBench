/** Bounded rankings derived only from retained, real bus telegrams. */
import type { BusTelegramRow } from "./api";

export const STATISTICS_TOP_LIMIT = 10;

interface CountEntry {
  label: string;
  count: number;
}

function countBy(rows: BusTelegramRow[], field: (row: BusTelegramRow) => string): CountEntry[] {
  const counts = new Map<string, number>();
  for (const row of rows) {
    const label = field(row);
    if (label && label !== "-") counts.set(label, (counts.get(label) ?? 0) + 1);
  }
  return [...counts].map(([label, count]) => ({ label, count }))
    .sort((a, b) => b.count - a.count || (a.label < b.label ? -1 : a.label > b.label ? 1 : 0))
    .slice(0, STATISTICS_TOP_LIMIT);
}

export function calculateBusMonitorStatistics(rows: BusTelegramRow[]) {
  // A SessionClosed row is local status, not a telegram on the wire. A
  // missing source or destination contributes no fabricated address count.
  const telegrams = rows.filter((row) => row.service !== "SessionClosed");
  return {
    observedRows: telegrams.length,
    services: countBy(telegrams, (row) => row.service),
    destinations: countBy(telegrams, (row) => row.destination),
    sources: countBy(telegrams, (row) => row.source),
  };
}
