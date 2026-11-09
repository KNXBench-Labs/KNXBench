/** Localizes known parameter metadata without changing source tokens or engineering values. */
import type { MessageKey, Translate } from "./i18n";

const KEYS = {
  kind: {
    Restriction: "parameters.kind.Restriction",
    Number: "parameters.kind.Number",
    Text: "parameters.kind.Text",
    None: "parameters.kind.None",
    Float: "parameters.kind.Float",
    IPAddress: "parameters.kind.IPAddress",
    Picture: "parameters.kind.Picture",
    Raw: "parameters.kind.Raw",
    Color: "parameters.kind.Color",
    Date: "parameters.kind.Date",
    Time: "parameters.kind.Time",
    Other: "parameters.kind.Other",
  },
  source: {
    Stored: "parameters.valueSource.Stored",
    ProgramDefault: "parameters.valueSource.ProgramDefault",
  },
  access: {
    None: "parameters.access.None",
    Read: "parameters.access.Read",
    ReadWrite: "parameters.access.ReadWrite",
  },
} as const satisfies Record<string, Record<string, MessageKey>>;

/** Unknown/new metadata stays verbatim rather than acquiring an invented interpretation. */
export function parameterMetadataLabel(t: Translate, category: keyof typeof KEYS, value: string): string {
  const keys: Readonly<Record<string, MessageKey>> = KEYS[category];
  return Object.hasOwn(keys, value) ? t(keys[value]) : value;
}
