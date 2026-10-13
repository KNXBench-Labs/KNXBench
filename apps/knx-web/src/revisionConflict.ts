/** Recognises Goal A2's typed stale-write refusals (revision-bound-writes contract). */
export type RevisionConflictKind = "parameterWriteConflict" | "renameConflict";

/**
 * `409` with exactly this server `kind`: nothing was applied because the
 * project, its revision or the expected value moved. Any other refusal,
 * including another `409`, is not a conflict and keeps its own message.
 */
export function isRevisionConflict(error: unknown, kind: RevisionConflictKind): boolean {
  if (error === null || typeof error !== "object") return false;
  const { status, body } = error as { status?: unknown; body?: unknown };
  if (status !== 409 || body === null || typeof body !== "object") return false;
  return (body as { kind?: unknown }).kind === kind;
}
