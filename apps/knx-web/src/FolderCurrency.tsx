/** Notice that saved project folders may no longer describe a device changed since import. */
import type { ProjectFolderCurrency } from "./bindings/ProjectFolderCurrency";
import type { ProjectFolderCurrencyUnknown } from "./bindings/ProjectFolderCurrencyUnknown";
import { useTranslate, type MessageKey } from "./i18n";

const UNKNOWN_KEYS: Record<ProjectFolderCurrencyUnknown, MessageKey> = {
  NoRetainedSource: "comTable.folderCurrency.unknown.NoRetainedSource",
  AmbiguousSource: "comTable.folderCurrency.unknown.AmbiguousSource",
  SourceIntegrity: "comTable.folderCurrency.unknown.SourceIntegrity",
  SourceLimit: "comTable.folderCurrency.unknown.SourceLimit",
  UnsupportedSchema: "comTable.folderCurrency.unknown.UnsupportedSchema",
  Unreadable: "comTable.folderCurrency.unknown.Unreadable",
  DeviceNotInSource: "comTable.folderCurrency.unknown.DeviceNotInSource",
  AmbiguousDevice: "comTable.folderCurrency.unknown.AmbiguousDevice",
};

/** `null` for an unchanged device; a newer server's state is named as unknown, never as current. */
export function folderCurrencyText(t: ReturnType<typeof useTranslate>, currency: ProjectFolderCurrency): string | null {
  switch (currency.status) {
    case "unchanged": return null;
    case "changedSinceImport": return t("comTable.folderCurrency.changed");
    case "unknown": {
      const known = Object.prototype.hasOwnProperty.call(UNKNOWN_KEYS, currency.reason);
      return t("comTable.folderCurrency.unknown", {
        reason: known ? t(UNKNOWN_KEYS[currency.reason]) : t("comTable.folderCurrency.unrecognised"),
      });
    }
    default: return t("comTable.folderCurrency.unknown", { reason: t("comTable.folderCurrency.unrecognised") });
  }
}

export function FolderCurrencyNotice(props: { currency: ProjectFolderCurrency | undefined }) {
  const t = useTranslate();
  if (props.currency === undefined) return null;
  const text = folderCurrencyText(t, props.currency);
  if (text === null) return null;
  return <p className="com-folder-currency" role="note" data-status={String(props.currency.status)}>{text}</p>;
}
