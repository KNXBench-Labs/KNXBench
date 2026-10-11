/** Stable preference identity; no network side effects or project-model fields. */
export const UPDATE_CHECK_SETTING = "releaseChecksEnabled";
export function releaseChecksEnabled(value: unknown): boolean { return value === undefined || value === true; }
