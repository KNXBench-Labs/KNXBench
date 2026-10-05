/** AR08: tells a project-password refusal of POST /api/project/import from every other failure. */

export type ProjectPasswordRefusal = "required" | "wrong";

/** `422` with `kind: projectPasswordRequired` / `projectPasswordWrong`
 * (`apps/knx-server/src/domain.rs`); anything else is an ordinary error. */
export function projectPasswordRefusal(error: unknown): ProjectPasswordRefusal | null {
  const { status, body } = (error ?? {}) as { status?: unknown; body?: { kind?: unknown } | null };
  if (status !== 422 || !body) return null;
  if (body.kind === "projectPasswordRequired") return "required";
  if (body.kind === "projectPasswordWrong") return "wrong";
  return null;
}
