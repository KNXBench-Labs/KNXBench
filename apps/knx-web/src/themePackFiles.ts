/** Bounded theme-file transport; admission is owned by themePack.ts. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { MAX_THEME_PACK_BYTES, parseThemePackText, validateThemePack } from "./themePack";
import { canonicalJson } from "./canonicalJson";
import type { ThemePack, ThemePackDiagnostic } from "./themePack";
export type ThemePackFileDiagnostic = Omit<ThemePackDiagnostic, "kind"> & {
  kind: ThemePackDiagnostic["kind"] | "invalidEncoding" | "fileRead";
};
export type ThemePackFileResult = { ok: true; pack: ThemePack } | { ok: false; diagnostic: ThemePackFileDiagnostic };
export type ThemePackExportResult = { ok: true; text: string; fileName: string } | { ok: false; diagnostic: ThemePackDiagnostic };
/** Recovery is a separately labelled inert document, not an importable theme. */
export const MAX_THEME_RECOVERY_BYTES = 1_048_576;
export function exportThemeRecovery(raw: unknown, selectedTheme: unknown): ThemePackExportResult {
  try {
    const text = canonicalJson({ format: "knxbench-theme-recovery", formatVersion: 1,
      selectedTheme, uiThemePacks: raw }, 2) + "\n";
    if (new TextEncoder().encode(text).byteLength > MAX_THEME_RECOVERY_BYTES) {
      return { ok: false, diagnostic: { kind: "sizeLimit", path: "$" } };
    }
    return { ok: true, text, fileName: "knxbench-theme-recovery.json" };
  } catch { return { ok: false, diagnostic: { kind: "invalidJson", path: "$" } }; }
}
export function exportThemePack(raw: unknown): ThemePackExportResult {
  const admitted = validateThemePack(raw);
  if (!admitted.ok) return admitted;
  const text = canonicalJson(admitted.pack, 2) + "\n";
  return { ok: true, text, fileName: `${admitted.pack.id}.knx-theme.json` };
}
const fileFailure = (kind: ThemePackFileDiagnostic["kind"]): ThemePackFileResult =>
  ({ ok: false, diagnostic: { kind, path: "$" } });
export async function readThemePackFile(file: Pick<File, "size" | "arrayBuffer">): Promise<ThemePackFileResult> {
  if (!Number.isInteger(file.size) || file.size < 0 || file.size > MAX_THEME_PACK_BYTES) return fileFailure("sizeLimit");
  let bytes: ArrayBuffer;
  try { bytes = await file.arrayBuffer(); } catch { return fileFailure("fileRead"); }
  if (bytes.byteLength > MAX_THEME_PACK_BYTES) return fileFailure("sizeLimit");
  let text: string;
  try { text = new TextDecoder("utf-8", { fatal: true, ignoreBOM: false }).decode(bytes); }
  catch { return fileFailure("invalidEncoding"); }
  return parseThemePackText(text);
}
