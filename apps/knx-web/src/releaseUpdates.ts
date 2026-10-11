/** Release selection is independent of UI, imported projects and KNX protocol state. */
export interface ReleaseUpdate { version: string; url: string; prerelease: boolean }
export const RELEASES_PAGE = "https://github.com/KNXBench-Labs/KNXBench/releases";

interface Version { text: string; core: string[]; pre: string[] }
export function isReleaseVersion(value: unknown): value is string {
  return typeof value === "string" && parseVersion(value) !== null;
}
function parseVersion(text: string): Version | null {
  const match = /^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/.exec(text);
  if (!match) return null;
  const pre = match[4]?.split(".") ?? [];
  if (pre.some(part => /^[0-9]+$/.test(part) && part.length > 1 && part[0] === "0")) return null;
  return { text, core: match.slice(1, 4), pre };
}
// Decimal strings avoid precision loss for large valid SemVer identifiers.
function numeric(a: string, b: string): number {
  return a.length === b.length ? (a === b ? 0 : a < b ? -1 : 1) : a.length < b.length ? -1 : 1;
}
function compare(a: Version, b: Version): number {
  for (let i = 0; i < 3; i++) { const order = numeric(a.core[i], b.core[i]); if (order) return order; }
  if (!a.pre.length || !b.pre.length) return a.pre.length === b.pre.length ? 0 : a.pre.length ? -1 : 1;
  for (let i = 0; i < Math.max(a.pre.length, b.pre.length); i++) {
    const left = a.pre[i], right = b.pre[i];
    if (left === undefined || right === undefined) return left === right ? 0 : left === undefined ? -1 : 1;
    if (left === right) continue;
    const ln = /^[0-9]+$/.test(left), rn = /^[0-9]+$/.test(right);
    return ln && rn ? numeric(left, right) : ln !== rn ? (ln ? -1 : 1) : left < right ? -1 : 1;
  }
  return 0;
}

/** GitHub's release list includes prereleases; its /latest endpoint does not. */
export function selectReleaseUpdate(current: string, releases: unknown): ReleaseUpdate | null {
  const running = parseVersion(current);
  if (!running || !Array.isArray(releases)) return null;
  let best = running, selected: ReleaseUpdate | null = null;
  for (const raw of releases) {
    if (!raw || typeof raw !== "object") continue;
    const row = raw as Record<string, unknown>;
    if (row.draft !== false || typeof row.prerelease !== "boolean" || typeof row.tag_name !== "string" || typeof row.published_at !== "string" || !Number.isFinite(Date.parse(row.published_at))) continue;
    const tag = row.tag_name;
    const candidate = parseVersion(tag.startsWith("v") ? tag.slice(1) : tag);
    if (!candidate || (!running.pre.length && (row.prerelease || candidate.pre.length)) || compare(candidate, best) <= 0) continue;
    best = candidate;
    selected = { version: candidate.text, url: `${RELEASES_PAGE}/tag/${encodeURIComponent(tag)}`, prerelease: row.prerelease || candidate.pre.length > 0 };
  }
  return selected;
}
