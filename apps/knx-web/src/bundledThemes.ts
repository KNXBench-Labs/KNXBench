/** Theme packs shipped with the application, admitted exactly like an imported file. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import crtText from "../themes/modern-retro-green-crt.knx-theme.json?raw";
import { parseThemePackText, type ThemePack } from "./themePack";

/**
 * Bundled packs are data, not code: the `.knx-theme.json` files under
 * `apps/knx-web/themes/` go through the same `parseThemePackText` admission
 * as a user's import, so a shipped theme can never use a token or value a
 * user file could not. A file that fails admission breaks the build's tests
 * (and the module) loudly rather than vanishing from the dropdown.
 *
 * They are never written into `uiThemePacks`: selecting one stores only its
 * id. An installed pack with the same id is the user's own data and takes
 * the bundled one's place (`theme.ts`, `findThemePack`).
 */
function admit(text: string, file: string): ThemePack {
  const result = parseThemePackText(text);
  if (!result.ok) throw new Error(`bundled theme ${file} was not admitted: ${JSON.stringify(result.diagnostic)}`);
  return result.pack;
}

export const BUNDLED_THEME_PACKS: readonly ThemePack[] = [
  admit(crtText, "modern-retro-green-crt.knx-theme.json"),
];
