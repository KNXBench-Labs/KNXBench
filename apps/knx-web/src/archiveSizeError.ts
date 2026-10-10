/** Presents the verified archive-size refusal in the user's language without changing its data. */
import { translate } from "./i18n";
import { getActiveUiLanguage } from "./uiLanguage";

/** Older servers expose no structured size fields. Recognize only the exact
 * ContainerError::TooLarge text; unknown messages stay raw. Error/body,
 * progress snapshots and server session-log diagnostics remain untouched. */
export function readableArchiveSizeError(message: string): string | undefined {
  const match = /^the archive declares (0|[1-9][0-9]{0,15}) uncompressed bytes, more than the (0|[1-9][0-9]{0,15})-byte limit$/.exec(message);
  if (!match || match[0] !== message) return undefined;
  const total = Number(match[1]);
  const limit = Number(match[2]);
  if (!Number.isSafeInteger(total) || !Number.isSafeInteger(limit) || limit <= 0 || total <= limit) return undefined;
  const language = getActiveUiLanguage();
  let locale = "en";
  try {
    locale = Intl.NumberFormat.supportedLocalesOf(language)[0] ?? "en";
  } catch {
    // A valid private-use pack tag need not be accepted by Intl.
  }
  const format = new Intl.NumberFormat(locale, { maximumFractionDigits: 2, useGrouping: false });
  const mib = 1024 * 1024;
  // Round the excess upward so even one byte above a whole-MiB limit
  // cannot misleadingly display the same number as that limit.
  return translate("toast.error.archiveTooLarge", {
    size: format.format(Math.ceil(total / mib * 100) / 100),
    limit: format.format(limit / mib),
  });
}
