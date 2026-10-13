/** HTTP client for the bounded literal group-address naming recipe (Goal B7). */
/*
 * `/api/name-recipe/*`, served by `apps/knx-server/src/name_recipe_routes.rs`.
 * Explicit group-address ids in the user's order, a literal prefix and
 * suffix, a preview bound to the read snapshot, and an apply that only
 * names the token of the preview the user is looking at.
 */
import { noteRefusal } from "../../api";

/** The server's own caps (`MAX_RECIPE_SELECTION`; 4 bytes × 1024 code points per affix). */
export const MAX_RECIPE_SELECTION = 256;
export const MAX_RECIPE_AFFIX_BYTES = 4096;

export type RecipeBinding = Readonly<{ serverIncarnation: string; projectIncarnation: number; snapshotRevision: number }>;
export type RecipeRow = Readonly<{ id: number; before: string; after: string; changed: boolean }>;
export type RecipePreview = Readonly<{ token: string; rows: readonly RecipeRow[]; changed: number; noOp: boolean; expiresInSeconds: number }>;
export type RecipeApplied = Readonly<{ changed: boolean; nativeGeneration: number | null; snapshotRevision: number;
  persistence: "session-only" | "native-working-history" }>;

/** A refusal with the server's machine-readable `kind` and English message. */
export type RecipeRefusal = Error & { status: number; kind: string | null };

export class RecipeContractError extends Error {
  constructor() { super("Unsupported or malformed naming-recipe response"); this.name = "RecipeContractError"; }
}

const isRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value);
const isCount = (value: unknown): value is number =>
  typeof value === "number" && Number.isSafeInteger(value) && value >= 0;

function admitPreview(value: unknown, ids: readonly number[]): RecipePreview {
  if (!isRecord(value) || typeof value.token !== "string" || !value.token || value.token.length > 128
    || !Array.isArray(value.rows) || !isCount(value.changed) || typeof value.noOp !== "boolean"
    || !isCount(value.expiresInSeconds) || value.expiresInSeconds > 300) throw new RecipeContractError();
  const rows = value.rows as unknown[];
  // The preview names exactly the chosen addresses, in the chosen order.
  if (rows.length !== ids.length) throw new RecipeContractError();
  rows.forEach((row, index) => {
    if (!isRecord(row) || row.id !== ids[index] || typeof row.before !== "string" || typeof row.after !== "string"
      || typeof row.changed !== "boolean" || row.changed !== (row.before !== row.after)) throw new RecipeContractError();
  });
  const changed = rows.filter((row) => (row as RecipeRow).changed).length;
  if (changed !== value.changed || value.noOp !== (changed === 0)) throw new RecipeContractError();
  return value as RecipePreview;
}

function admitApplied(value: unknown): RecipeApplied {
  if (!isRecord(value) || typeof value.changed !== "boolean"
    || !(value.nativeGeneration === null || isCount(value.nativeGeneration)) || !isCount(value.snapshotRevision)
    || (value.persistence !== "session-only" && value.persistence !== "native-working-history")) throw new RecipeContractError();
  return value as RecipeApplied;
}

async function post(path: string, body: unknown, transport: typeof fetch): Promise<unknown> {
  const response = await transport(path, { method: "POST", credentials: "same-origin",
    headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) });
  const value: unknown = await response.json().catch(() => null);
  if (!response.ok) {
    noteRefusal(path, response.status);
    const message = isRecord(value) && typeof value.error === "string" ? value.error : `Naming recipe request failed (${response.status})`;
    throw Object.assign(new Error(message), { status: response.status,
      kind: isRecord(value) && typeof value.kind === "string" ? value.kind : null }) as RecipeRefusal;
  }
  return value;
}

/** The preview the user confirms is gone (project changed, newer preview, expired, spent). */
export function isStaleRecipe(error: unknown): boolean {
  return isRecord(error) && error.status === 409 && error.kind === "nameRecipeStale";
}

export function previewNameRecipe(binding: RecipeBinding, ids: readonly number[], prefix: string, suffix: string,
  transport: typeof fetch = fetch): Promise<RecipePreview> {
  return post("/api/name-recipe/preview", { ...binding, groupAddresses: ids, prefix, suffix }, transport)
    .then((value) => admitPreview(value, ids));
}

/** `confirmed` is literal: the only caller is the button under the shown preview. */
export function applyNameRecipe(token: string, transport: typeof fetch = fetch): Promise<RecipeApplied> {
  return post("/api/name-recipe/apply", { token, confirmed: true }, transport).then(admitApplied);
}

/** Best effort: a forgotten preview also expires on the server. */
export function cancelNameRecipe(token: string, transport: typeof fetch = fetch): void {
  void post("/api/name-recipe/cancel", { token }, transport).catch(() => undefined);
}

export function utf8Bytes(text: string): number {
  return new TextEncoder().encode(text).length;
}
