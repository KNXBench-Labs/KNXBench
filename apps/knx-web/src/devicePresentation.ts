/** Display-only device names shared by every device consumer; stored names stay untouched. */
import { isBlankName } from "./blankName";
import type { Translate } from "./i18n";

/** The exact nonblank name; a blank one shows an unnamed marker with the stable ID. */
export function deviceLabel(t: Translate, device: { id: number; name: string }): string {
  return isBlankName(device.name) ? t("device.unnamed", { id: device.id }) : device.name;
}
