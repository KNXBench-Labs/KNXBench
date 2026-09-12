import { useCallback } from "react";
import { messages as enMessages } from "./messages/en";
import type { MessageKey } from "./messages/en";
import { messages as deMessages } from "./messages/de";
import {
  type BuiltInUiLanguage,
  type UiLanguage,
  getActiveUiLanguage,
  useUiLanguage,
} from "./uiLanguage";
import { getLanguagePack, useLanguagePacks } from "./languagePack";

export type { MessageKey } from "./messages/en";

/**
 * A plural key's *base* — `"foo"` for a catalogue that has `"foo.one"` and
 * `"foo.other"` entries — derived from whichever full keys actually end in
 * `.other`, so a call site can only ever name a base that really has a
 * plural pair behind it. Currently `never`: none of the ten seeded keys
 * are plural. Extraction tasks that add a `key.one`/`key.other` pair widen
 * this automatically, no rework here required — this is the "third locale
 * without rework" property the brief calls for, just applied to plural
 * bases instead of locales.
 */
type PluralBase<K> = K extends `${infer Base}.other` ? Base : never;
export type PluralMessageKey = PluralBase<MessageKey>;

/** Anything `t()`/`translate()` accept as a key: a real message, or the
 * base of a plural pair. */
export type TranslatableKey = MessageKey | PluralMessageKey;

/** `{name}`-style placeholder values, and/or the `count` a plural lookup
 * selects its category from. */
export type MessageParams = Record<string, string | number>;

const catalogs: Record<BuiltInUiLanguage, Record<string, string>> = {
  en: enMessages,
  de: deMessages,
};

/**
 * The catalogue for `language`: a built-in catalogue when `language` is
 * `"en"`/`"de"`, otherwise the `messages` of an installed pack matching
 * that tag (`languagePack.ts`). `undefined` when `language` names a pack
 * that isn't installed — removed, or a stale/hand-edited active tag —
 * which `translateFor` treats exactly like a catalogue with no entries at
 * all: straight through to the English fallback, never an error.
 */
function resolveCatalog(language: UiLanguage): Record<string, string> | undefined {
  if (language === "en" || language === "de") return catalogs[language];
  return getLanguagePack(language)?.messages;
}

/**
 * `Intl.PluralRules` for `language`, cached per tag. Built-in languages
 * (`"en"`/`"de"`) always resolve — every runtime this app ships on
 * supports them, so this cache behaves exactly as it did before pack tags
 * existed. A pack tag's rules are looked up the same way
 * `languagePack.ts`'s `pluralRulesSupportedFor` does — via
 * `supportedLocalesOf`, not by constructing and hoping it throws — and
 * `null` is cached (not retried) for a tag the runtime has no data for,
 * so `resolveFromCatalog` can degrade to the `"other"` category instead
 * of ever letting `Intl.PluralRules` throw out of the lookup.
 */
const pluralRulesCache = new Map<string, Intl.PluralRules | null>();

function getPluralRules(language: string): Intl.PluralRules | null {
  if (pluralRulesCache.has(language)) return pluralRulesCache.get(language) ?? null;

  let rules: Intl.PluralRules | null = null;
  try {
    if (Intl.PluralRules.supportedLocalesOf(language).length > 0) {
      rules = new Intl.PluralRules(language);
    }
  } catch {
    rules = null;
  }
  pluralRulesCache.set(language, rules);
  return rules;
}

/**
 * Substitutes `{name}` placeholders from `params`. A placeholder with no
 * matching param is left as literal text (`"{name}"`) rather than blanked
 * out — a missing param is a call-site bug, and leaving the brace visible
 * makes that bug obvious in the rendered UI instead of hiding it.
 *
 * Exported standalone (not folded into `translate()`) so extraction tasks
 * and tests can exercise substitution against an arbitrary template
 * without needing a real catalogue key to hang it on.
 */
export function formatTemplate(template: string, params?: MessageParams): string {
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (match, name: string) => {
    const value = params[name];
    return value === undefined ? match : String(value);
  });
}

/**
 * Looks `key` up directly in `catalog`; if that misses but `${key}.other`
 * exists, treats `key` as a plural base instead — selecting `.one`/`.other`
 * (or whatever category `Intl.PluralRules` names for `language`, falling
 * back to `.other` for any category the two-entry convention doesn't
 * cover, e.g. Arabic's `.few`/`.many`, *and* for a `language` the runtime
 * has no plural data for at all — a pack tag such as `"art-x-sindarin"` —
 * see `getPluralRules`) using `params.count` (default `0`). Returns
 * `undefined` on a genuine miss, leaving the fallback-to-English decision
 * to the caller.
 *
 * Exported standalone, same reasoning as `formatTemplate`: it takes a
 * plain `Record<string, string>`, so plural-branch tests can hand it a
 * small synthetic catalogue instead of depending on a real plural key
 * existing in `messages/en.ts` (there isn't one yet — see `PluralBase`
 * above).
 */
export function resolveFromCatalog(
  catalog: Record<string, string>,
  key: string,
  params: MessageParams | undefined,
  language: string,
): string | undefined {
  const direct = catalog[key];
  if (direct !== undefined) return direct;

  const otherKey = `${key}.other`;
  if (catalog[otherKey] === undefined) return undefined;

  const count = typeof params?.count === "number" ? params.count : 0;
  const category = getPluralRules(language)?.select(count) ?? "other";
  return catalog[`${key}.${category}`] ?? catalog[otherKey];
}

/**
 * The core lookup: `key` in `language`'s catalogue, English on a miss
 * (D3 — deliberately the opposite of the product-data language's
 * no-fallback rule; see `messages/de.ts`'s header), placeholders/plurals
 * resolved, key itself as a last-resort label if even English has
 * nothing (unreachable given `MessageKey`/`de.ts`'s typing, but a key
 * beats a blank string if it ever happens).
 *
 * `language` is an open tag (T25 task 6): a built-in id or an installed
 * pack's tag, resolved via `resolveCatalog`. The fallback chain is
 * exactly two long — `language`, then English — never German, never
 * another installed pack, and never the pack's own `basedOn` (that field
 * isn't even read here). A Bavarian pack missing a key falls to English,
 * not to German, even though `basedOn: "de"` would suggest otherwise —
 * see `languagePack.ts`'s `LanguagePack.basedOn` for why.
 */
export function translateFor(
  language: UiLanguage,
  key: TranslatableKey,
  params?: MessageParams,
): string {
  const active = resolveCatalog(language);
  let template = active ? resolveFromCatalog(active, key, params, language) : undefined;

  if (template === undefined && language !== "en") {
    template = resolveFromCatalog(catalogs.en, key, params, "en");
  }

  if (template === undefined) {
    return key;
  }

  return formatTemplate(template, params);
}

/**
 * Module-level equivalent of `useTranslate()`'s `t`, for code that runs
 * outside a component — `commandRegistry.ts`'s static `COMMANDS` array,
 * for instance, or any other non-hook helper. Reads the current UI
 * language fresh on every call via `getActiveUiLanguage()` rather than
 * subscribing to changes, so anything built from it once (a module-level
 * array, a memoised value) needs to be rebuilt when the language changes —
 * that's the extraction tasks' problem to solve per call site, not
 * something this function can paper over.
 */
export function translate(key: TranslatableKey, params?: MessageParams): string {
  return translateFor(getActiveUiLanguage(), key, params);
}

/** The shape `useTranslate()` returns: a plain, stable-identity-per-language
 * function, safe to put in a `useMemo`/`useCallback` dependency array or to
 * call directly from a JSX attribute (`aria-label={t("toolbar.settings")}`),
 * not just from JSX children. */
export type Translate = (key: TranslatableKey, params?: MessageParams) => string;

/**
 * Reads the active UI language via `useUiLanguage()` and returns a bound
 * `t(key, params?)`. The returned function's identity changes when the
 * language does *or* when the installed language packs change — also
 * subscribing to `languagePack.ts`'s `useLanguagePacks()` is what makes
 * an already-mounted caller notice an import or a removal on its own:
 * without it, a pack changing underneath an unrelated re-render would
 * leave this hook returning a `t` that still resolves through
 * `translateFor` correctly in theory, but nothing would have triggered
 * the re-render to call it again. `packs` itself is never read — only
 * its identity, as an `useCallback` dependency and (via
 * `useSyncExternalStore` inside `useLanguagePacks()`) a subscription —
 * `translateFor` re-resolves the active catalogue fresh on every call.
 */
export function useTranslate(): Translate {
  const [language] = useUiLanguage();
  const packs = useLanguagePacks();
  return useCallback<Translate>(
    (key, params) => translateFor(language, key, params),
    [language, packs],
  );
}
