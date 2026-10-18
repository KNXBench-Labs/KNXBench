/** Read-only server measurement; paths never cross this boundary. */
export interface StatusMetrics {
  project: {
    state: "available" | "notSavedYet" | "unavailable" | "error";
    projectIncarnation: number | null;
    snapshotRevision: number | null;
    bytes: number | null;
  };
  catalogue: {
    state: "available" | "unavailable" | "error";
    bytes: number | null;
    counts: { manufacturers: number; products: number; applicationPrograms: number } | null;
  };
}

export function formatDecimalMb(bytes: number, locale: string): string {
  const format = new Intl.NumberFormat(locale, { minimumFractionDigits: 1, maximumFractionDigits: 1 });
  return bytes < 100_000 ? `< ${format.format(0.1)} MB` : `${format.format(bytes / 1_000_000)} MB`;
}
