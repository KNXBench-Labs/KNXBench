/** Admit passive USB-serial candidate responses without treating them as connection evidence. */
export interface SerialCandidate { path: string; target: string }
export type SerialCandidates =
  | { status: "available"; candidates: SerialCandidate[]; truncated: boolean; omittedEntries: number }
  | { status: "unavailable"; reason: "missingDirectory" | "permissionDenied" | "io" }
  | { status: "unsupported" };

function record(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}
function keys(value: Record<string, unknown>, names: string[]): boolean {
  return Object.keys(value).length === names.length && names.every(name => Object.hasOwn(value, name));
}
function aliasPath(value: unknown): value is string {
  if (typeof value !== "string" || !value.startsWith("/dev/serial/by-id/usb-")) return false;
  const name = value.slice("/dev/serial/by-id/".length);
  return name.length > 4 && new TextEncoder().encode(name).length <= 255 && name === name.trim()
    && !/[\u0000-\u001f\u007f-\u009f/\\\ud800-\udfff]/u.test(name);
}
function targetPath(value: unknown): value is string {
  return typeof value === "string" && /^\/dev\/tty(?:USB|ACM)[0-9]+$/u.test(value)
    && !value.endsWith("\n") && value.length <= 260;
}
export function admitSerialCandidates(value: unknown): SerialCandidates {
  const malformed = () => { throw new Error("Malformed USB-serial candidate response"); };
  if (!record(value)) return malformed();
  if (value.status === "unsupported" && keys(value, ["status"])) return { status: "unsupported" };
  if (value.status === "unavailable" && keys(value, ["status", "reason"])
    && (value.reason === "missingDirectory" || value.reason === "permissionDenied" || value.reason === "io")) {
    return { status: "unavailable", reason: value.reason };
  }
  if (value.status !== "available" || !keys(value, ["status", "candidates", "truncated", "omittedEntries"])
    || !Array.isArray(value.candidates) || value.candidates.length > 64 || typeof value.truncated !== "boolean"
    || !Number.isSafeInteger(value.omittedEntries) || typeof value.omittedEntries !== "number"
    || value.omittedEntries < 0 || value.omittedEntries + value.candidates.length > 512) return malformed();
  const paths = new Set<string>(), targets = new Set<string>();
  const candidates = value.candidates.map((row: unknown): SerialCandidate => {
    if (!record(row) || !keys(row, ["path", "target"]) || !aliasPath(row.path) || !targetPath(row.target)
      || paths.has(row.path) || targets.has(row.target)) return malformed();
    paths.add(row.path); targets.add(row.target);
    return { path: row.path, target: row.target };
  });
  return { status: "available", candidates, truncated: value.truncated, omittedEntries: value.omittedEntries };
}
