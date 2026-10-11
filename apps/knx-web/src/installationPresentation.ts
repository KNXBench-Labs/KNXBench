/** Display-only installation names; imported names and stable IDs stay untouched. */
import type { Translate } from "./i18n";

export function installationLabel(t: Translate, installation: { id: number; name: string }): string {
  return installation.name.trim() ? installation.name : t("installation.unnamed", { id: installation.id });
}
