/** The community Hall of Fame snapshot: admitted as untrusted data, shown as generated. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { CONTRIBUTION_REPOSITORY } from "./ContributionButton";

/**
 * `tools/community_hall_of_fame.py` generates `docs/community/hall-of-fame.json`
 * (schema 1, docs/contracts/community-recognition.md). About only shows it: the
 * order, positions, XP, titles and medals are the generator's, so the app keeps
 * no second scoring logic. A snapshot with any malformed or duplicate entry is
 * refused whole, the same rule the generator applies, rather than shown in part.
 */
export interface HallOfFameEntry {
  login: string;
  /** Only for a well-formed GitHub handle; everything else stays plain text. */
  profileUrl: string | null;
  position: number;
  xp: number;
  issues: number;
  replies: number;
  resolutions: number;
  title: string;
  medals: string[];
}

export interface HallOfFameSnapshot {
  generatedAt: string;
  entries: HallOfFameEntry[];
}

export type HallOfFameResult = { ok: true; snapshot: HallOfFameSnapshot } | { ok: false; reason: string };

/** The live board, always on this project's own repository, whatever the file names. */
export const HALL_OF_FAME_BOARD_URL = `${CONTRIBUTION_REPOSITORY}/blob/main/docs/community/HALL_OF_FAME.md`;

// GitHub handles: 1–39 ASCII letters, digits or single inner hyphens.
const GITHUB_HANDLE = /^[A-Za-z0-9](?:[A-Za-z0-9]|-(?=[A-Za-z0-9])){0,38}$/;
const MAX_TEXT = 200;

export function githubProfileUrl(login: string): string | null {
  return GITHUB_HANDLE.test(login) ? `https://github.com/${login}` : null;
}

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);
const isCount = (value: unknown): value is number => Number.isSafeInteger(value) && (value as number) >= 0;
const isText = (value: unknown): value is string => typeof value === "string" && value.length > 0 && value.length <= MAX_TEXT;

function admitEntry(row: unknown, index: number): HallOfFameEntry | string {
  if (!isRecord(row)) return `entry ${index} is not an object`;
  const { login, position, xp, issues, replies, resolutions, rank, medals } = row;
  if (!isText(login)) return `entry ${index} has no handle`;
  if (!isCount(position) || position < 1) return `${login}: position`;
  if (!isCount(xp) || !isCount(issues) || !isCount(replies) || !isCount(resolutions)) return `${login}: counters`;
  if (!isRecord(rank) || !isText(rank.title)) return `${login}: title`;
  if (!Array.isArray(medals) || !medals.every(isText)) return `${login}: medals`;
  return { login, profileUrl: githubProfileUrl(login), position, xp, issues, replies, resolutions,
    title: rank.title, medals: [...medals] };
}

export function parseHallOfFame(text: string): HallOfFameResult {
  let data: unknown;
  try {
    data = JSON.parse(text);
  } catch {
    return { ok: false, reason: "not JSON" };
  }
  if (!isRecord(data) || data.schema_version !== 1) return { ok: false, reason: "not schema 1" };
  const generatedAt = data.generated_at;
  if (typeof generatedAt !== "string" || Number.isNaN(Date.parse(generatedAt))) return { ok: false, reason: "timestamp" };
  if (!Array.isArray(data.contributors)) return { ok: false, reason: "no contributors" };
  const entries: HallOfFameEntry[] = [];
  const seen = new Set<string>();
  for (const [index, row] of data.contributors.entries()) {
    const entry = admitEntry(row, index);
    if (typeof entry === "string") return { ok: false, reason: entry };
    // GitHub handles are case-insensitive: one person, one row.
    const key = entry.login.toLowerCase();
    if (seen.has(key)) return { ok: false, reason: `${entry.login} twice` };
    seen.add(key);
    entries.push(entry);
  }
  return { ok: true, snapshot: { generatedAt, entries } };
}
