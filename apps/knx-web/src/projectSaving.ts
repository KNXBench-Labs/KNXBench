/** WP-31: a write refused because a Save holds the project file is retryable, never a failure. */
import { translate } from "./i18n";

/** Whether the server refused with `409 projectSaving`: nothing changed, the same request works later. */
export function isProjectSaving(error: unknown): boolean {
  if (typeof error !== "object" || error === null) return false;
  const { status, kind, body } = error as { status?: unknown; kind?: unknown; body?: { kind?: unknown } | null };
  return status === 409 && (kind === "projectSaving" || body?.kind === "projectSaving");
}

/** The translated retry sentence for a `projectSaving` refusal, else `null`. */
export function projectSavingMessage(error: unknown): string | null {
  return isProjectSaving(error) ? translate("projectSaving.retry") : null;
}
