/** Pure presentation helpers for previewed catalogue updates and their backups. */
import type { CatalogueProgramUpdate } from "./api";

/** Bytes as a short, locale-formatted size (binary units). */
export function formatBytes(bytes: number, locale: string): string {
  const units = ["B", "KiB", "MiB", "GiB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) { value /= 1024; unit += 1; }
  const digits = unit === 0 ? 0 : 1;
  return `${new Intl.NumberFormat(locale, { maximumFractionDigits: digits, minimumFractionDigits: digits }).format(value)} ${units[unit]}`;
}

/** Installed versions of the same application, as written in the catalogue. */
export function installedVersionList(program: CatalogueProgramUpdate, unknown: string): string {
  return program.installedVersions.map((version) => version.applicationVersion ?? unknown).join(", ");
}
