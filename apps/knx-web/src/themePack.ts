/** Pure admission boundary for versioned, declarative theme packs. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import type { Accent } from "./appearance";
import { evaluateThemeContrast } from "./themeTokens";
import type { ContrastViolation, CssRule } from "./themeTokens";
export interface ThemePack {
  format: "knxbench-theme"; formatVersion: 1; tokenVersion: 1;
  id: string; name: string; version: string; colorScheme: "light" | "dark";
  tokens: Record<ThemeToken, string>;
  accents?: Partial<Record<Accent, Record<"--knx-accent" | "--knx-on-accent", string>>>;
}
export interface ThemePackDiagnostic {
  kind: "invalidJson" | "invalidContract" | "unsupportedFormat" | "unsupportedVersion" | "invalidMetadata"
    | "invalidTokens" | "invalidValue" | "invalidAccents" | "contrast"
    | "duplicateKey" | "sizeLimit" | "depthLimit" | "nodeLimit" | "identityMismatch" | "storeLimit" | "missingSelection";
  path: string;
  violations?: ContrastViolation[];
}
export type ThemePackResult = { ok: true; pack: ThemePack } | { ok: false; diagnostic: ThemePackDiagnostic };
export interface ThemePackStore {
  packs: ThemePack[];
  diagnostics: { id: string | null; diagnostic: ThemePackDiagnostic }[];
  valid: boolean;
}
export const MAX_INSTALLED_THEME_PACKS = 16;
export const MAX_THEME_STORE_BYTES = 524_288;
export function readThemePackStore(raw: unknown): ThemePackStore {
  const rejected = (kind: ThemePackDiagnostic["kind"]): ThemePackStore => ({
    packs: [], valid: false, diagnostics: [{ id: null, diagnostic: { kind, path: "$.uiThemePacks" } }],
  });
  if (raw === undefined) return { packs: [], diagnostics: [], valid: true };
  if (!record(raw)) return rejected("invalidContract");
  if (Object.keys(raw).length > MAX_INSTALLED_THEME_PACKS) return rejected("storeLimit");
  try {
    if (new TextEncoder().encode(JSON.stringify(raw)).length > MAX_THEME_STORE_BYTES) return rejected("storeLimit");
  } catch { return rejected("invalidContract"); }
  const packs: ThemePack[] = [], diagnostics: ThemePackStore["diagnostics"] = [];
  for (const [key, value] of Object.entries(raw)) {
    const result = validateThemePack(value);
    const id = safeText(key, 48) && USER_THEME_ID.test(key) ? key : null;
    if (!result.ok) diagnostics.push({ id, diagnostic: result.diagnostic });
    else if (result.pack.id !== key) diagnostics.push({ id, diagnostic: { kind: "identityMismatch", path: "$.id" } });
    else packs.push(result.pack);
  }
  return { packs, diagnostics, valid: diagnostics.length === 0 };
}
function failure(kind: ThemePackDiagnostic["kind"], path = "$", violations?: ContrastViolation[]): ThemePackResult {
  return { ok: false, diagnostic: { kind, path, ...(violations ? { violations } : {}) } };
}
function record(value: unknown): value is Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return false;
  const prototype: unknown = Object.getPrototypeOf(value);
  if (prototype !== null && prototype !== Object.prototype) return false;
  return Object.values(Object.getOwnPropertyDescriptors(value)).every((entry) => Object.hasOwn(entry, "value") && entry.enumerable);
}
function safeText(value: unknown, max: number): value is string {
  if (typeof value !== "string" || [...value].length > max
    || /[\u0000-\u001f\u007f-\u009f\u061c\u200e\u200f\u202a-\u202e\u2066-\u2069]/u.test(value)) return false;
  return ![...value].some((char) => {
    const point = char.codePointAt(0)!;
    return point >= 0xd800 && point <= 0xdfff;
  });
}
const REQUIRED_FIELDS = ["format", "formatVersion", "tokenVersion", "id", "name", "version", "colorScheme", "tokens"];
const USER_THEME_ID = /^user-[a-z0-9][a-z0-9-]{0,42}$/;
export const THEME_TOKEN_CLASSES = {
  "--knx-bg": "opaque", "--knx-surface": "opaque", "--knx-foreground": "opaque",
  "--knx-accent": "opaque", "--knx-on-accent": "opaque",
  "--knx-muted": "colour", "--knx-border": "colour", "--knx-border-hover": "colour",
  "--knx-accent-tertiary": "colour", "--knx-error-color": "colour", "--knx-warning-color": "colour",
  "--knx-success-color": "colour", "--knx-overlay-backdrop": "colour", "--knx-overlay-shadow": "colour",
  "--knx-gradient-primary": "gradient", "--knx-gradient-display": "gradient",
  "--knx-shadow-raised": "shadow", "--knx-shadow-hover": "shadow",
  "--knx-font-heading": "font", "--knx-font-body": "font", "--knx-font-mono": "font",
  "--knx-radius-card": "radius", "--knx-radius-button": "radius", "--knx-radius-input": "radius",
  "--knx-backdrop-image": "gradient", "--knx-backdrop-mask": "gradient", "--knx-backdrop-size": "size",
} as const;
export type ThemeToken = keyof typeof THEME_TOKEN_CLASSES;
const TOKEN_KEYS = Object.keys(THEME_TOKEN_CLASSES) as ThemeToken[];
export const THEME_PACK_ACCENTS = ["violet", "mint", "blue", "amber", "rose"] as const satisfies readonly Accent[];
const HEX_COLOUR = /^#[0-9a-f]{6}(?:[0-9a-f]{2})?$/i;
const OPAQUE_COLOUR = /^#[0-9a-f]{6}(?:ff)?$/i;
const FONT_VALUES: readonly string[] = ['"Inter", sans-serif', '"Space Grotesk", sans-serif',
  '"JetBrains Mono", monospace', "system-ui, sans-serif", "ui-monospace, monospace"];
const DECIMAL = "-?(?:0|[1-9][0-9]*)(?:\\.[0-9]+)?";
const DECIMAL_VALUE = new RegExp(`^${DECIMAL}$`);
function boundedNumber(value: string, min: number, max: number): boolean {
  if (!DECIMAL_VALUE.test(value) || (min >= 0 && value.startsWith("-"))) return false;
  const number = Number(value);
  // Bounds here are integers. Inspect the decimal tail before floating-point
  // conversion can round e.g. 360.000000000000000000001 down to 360.
  const negative = value.startsWith("-");
  const [whole, fraction = ""] = (negative ? value.slice(1) : value).split(".");
  const integral = negative ? -Number(whole) : Number(whole);
  if (integral < min || integral > max) return false;
  if (/[1-9]/.test(fraction) && (negative ? integral === min : integral === max)) return false;
  return Number.isFinite(number) && number >= min && number <= max;
}
function gradient(value: string): boolean {
  if (value === "none") return true;
  const match = /^linear-gradient\((.*)\)$/.exec(value);
  if (!match) return false;
  const parts = match[1].split(",");
  if (parts.length < 3 || parts.length > 5 || !parts[0].endsWith("deg")) return false;
  if (!boundedNumber(parts[0].slice(0, -3), 0, 360)) return false;
  return parts.slice(1).every((part) => HEX_COLOUR.test(part.startsWith(" ") ? part.slice(1) : part));
}
function validTokenValue(kind: typeof THEME_TOKEN_CLASSES[ThemeToken], value: string): boolean {
  if (kind === "opaque") return OPAQUE_COLOUR.test(value);
  if (kind === "colour") return HEX_COLOUR.test(value);
  if (kind === "gradient") return gradient(value);
  if (kind === "radius") return value.endsWith("px") && boundedNumber(value.slice(0, -2), 0, 999);
  if (kind === "size") {
    if (value === "auto") return true;
    const parts = value.split(" ");
    return parts.length === 2 && parts.every((part) => part.endsWith("px") && boundedNumber(part.slice(0, -2), 1, 512));
  }
  if (kind === "shadow") {
    if (value === "none") return true;
    const parts = value.split(" ");
    return parts.length === 5 && HEX_COLOUR.test(parts[4]) && parts.slice(0, 4).every((part, index) =>
      part.endsWith("px") && boundedNumber(part.slice(0, -2), index === 2 ? 0 : -64, index === 2 ? 128 : 64));
  }
  if (kind === "font") return FONT_VALUES.includes(value);
  return false;
}
export function validateThemePack(raw: unknown): ThemePackResult {
  if (!record(raw) || REQUIRED_FIELDS.some((key) => !Object.hasOwn(raw, key))
    || Object.keys(raw).some((key) => !REQUIRED_FIELDS.includes(key) && key !== "accents")) {
    return failure("invalidContract");
  }
  if (raw.format !== "knxbench-theme") return failure("unsupportedFormat", "$.format");
  if (raw.formatVersion !== 1) return failure("unsupportedVersion", "$.formatVersion");
  if (raw.tokenVersion !== 1) return failure("unsupportedVersion", "$.tokenVersion");
  if (!safeText(raw.id, 48) || !USER_THEME_ID.test(raw.id)) return failure("invalidMetadata", "$.id");
  for (const [key, max] of [["name", 80], ["version", 32]] as const) {
    if (!safeText(raw[key], max) || !(raw[key] as string).trim()) return failure("invalidMetadata", `$.${key}`);
  }
  if (raw.colorScheme !== "light" && raw.colorScheme !== "dark") return failure("invalidMetadata", "$.colorScheme");
  if (!record(raw.tokens) || Object.keys(raw.tokens).length !== TOKEN_KEYS.length
    || TOKEN_KEYS.some((key) => !Object.hasOwn(raw.tokens as object, key))) return failure("invalidTokens", "$.tokens");
  for (const key of TOKEN_KEYS) {
    const value = raw.tokens[key];
    if (!safeText(value, 512) || !validTokenValue(THEME_TOKEN_CLASSES[key], value)) return failure("invalidValue", `$.tokens.${key}`);
  }
  const accents: ThemePack["accents"] = {};
  if (Object.hasOwn(raw, "accents")) {
    if (!record(raw.accents)) return failure("invalidAccents", "$.accents");
    for (const [id, entry] of Object.entries(raw.accents)) {
      if (!THEME_PACK_ACCENTS.some((known) => known === id) || !record(entry)
        || Object.keys(entry).length !== 2 || !Object.hasOwn(entry, "--knx-accent") || !Object.hasOwn(entry, "--knx-on-accent")) {
        return failure("invalidAccents", "$.accents");
      }
      const accent = entry["--knx-accent"], onAccent = entry["--knx-on-accent"];
      if (!safeText(accent, 512) || !OPAQUE_COLOUR.test(accent) || !safeText(onAccent, 512) || !OPAQUE_COLOUR.test(onAccent)) {
        return failure("invalidAccents", `$.accents.${id}`);
      }
      accents[id as Accent] = { "--knx-accent": accent, "--knx-on-accent": onAccent };
    }
  }
  const pack: ThemePack = {
    format: "knxbench-theme", formatVersion: 1, tokenVersion: 1, id: raw.id,
    name: raw.name as string, version: raw.version as string, colorScheme: raw.colorScheme,
    tokens: Object.fromEntries(TOKEN_KEYS.map((key) => [key, (raw.tokens as Record<string, string>)[key]])) as ThemePack["tokens"],
    ...(Object.hasOwn(raw, "accents") ? { accents } : {}),
  };
  const rule = (tokens: Record<string, string>): CssRule => ({
    selector: `:root[data-theme="${pack.id}"]`, line: 1, depth: 0, ancestors: [],
    declarations: Object.entries(tokens).map(([property, value]) => ({ property, value, line: 1 })),
  });
  const theme = { id: pack.id, rule: rule(pack.tokens) };
  const baseViolations = evaluateThemeContrast(theme);
  if (baseViolations.length) return failure("contrast", "$.tokens", baseViolations);
  for (const [accent, tokens] of Object.entries(accents)) {
    const violations = evaluateThemeContrast(theme, { id: pack.id, accent, rule: rule(tokens) });
    if (violations.length) return failure("contrast", `$.accents.${accent}`, violations);
  }
  return { ok: true, pack };
}
export const MAX_THEME_PACK_BYTES = 65_536;
class TextAdmissionError extends Error {
  constructor(readonly kind: "duplicateKey" | "depthLimit" | "nodeLimit") { super(kind); }
}
/** Walk syntax-checked source text: JSON.parse cannot expose overwritten keys.
 * Keys are decoded independently; strings containing braces/escaped quotes
 * are never mistaken for structure. No typed pack escapes before this pass. */
function inspectJsonText(text: string): void {
  let index = 0, nodes = 0;
  const whitespace = () => { while (/[ \t\r\n]/.test(text[index] ?? "")) index++; };
  const string = (): string => {
    const start = index++;
    while (text[index] !== '"') {
      index += text[index] === "\\" ? 2 : 1;
    }
    index++;
    return JSON.parse(text.slice(start, index)) as string;
  };
  const value = (depth: number): void => {
    if (++nodes > 1024) throw new TextAdmissionError("nodeLimit");
    whitespace();
    const open = text[index];
    if (open === "{" || open === "[") {
      if (depth >= 8) throw new TextAdmissionError("depthLimit");
      const close = open === "{" ? "}" : "]";
      const keys = new Set<string>();
      index++; whitespace();
      if (text[index] === close) { index++; return; }
      while (true) {
        if (open === "{") {
          const key = string();
          if (keys.has(key)) throw new TextAdmissionError("duplicateKey");
          keys.add(key); whitespace(); index++; // Syntax-checked colon.
        }
        value(depth + 1); whitespace();
        if (text[index] === close) { index++; return; }
        index++; whitespace(); // Syntax-checked comma.
      }
    }
    if (open === '"') { string(); return; }
    // Syntax-checked number/boolean/null: stop before the next delimiter.
    while (index < text.length && !/[,\]} \t\r\n]/.test(text[index])) index++;
  };
  value(0);
}
export function parseThemePackText(text: string): ThemePackResult {
  if (text.length > MAX_THEME_PACK_BYTES || new TextEncoder().encode(text).length > MAX_THEME_PACK_BYTES) {
    return failure("sizeLimit");
  }
  try {
    const raw: unknown = JSON.parse(text);
    inspectJsonText(text);
    return validateThemePack(raw);
  } catch (error) {
    if (error instanceof TextAdmissionError) return failure(error.kind);
    return failure("invalidJson");
  }
}
