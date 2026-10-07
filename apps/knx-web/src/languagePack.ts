/** User-importable UI language packs: validate, store, import, and export beyond built-in ones. */
import { useMemo, useSyncExternalStore } from "react";
import { BUNDLED_LANGUAGE_PACKS } from "./bundledLanguagePacks";
import { messages as enMessages } from "./messages/en";
import { getSetting, setSettingOrThrow, subscribeToSettings } from "./settingsStore";

/**
 * Key inside the settings document for installed language packs, a
 * sibling of `uiLanguage.ts`'s `UI_LANGUAGE_STORAGE_KEY`: that key names
 * which language is *active*, this one holds the packs that make an
 * imported-but-not-compiled-in language available to name in the first
 * place. A JSON object keyed by `tag`, so importing a pack with a tag
 * that's already installed replaces it — an upgrade, not a duplicate.
 *
 * The one preference stored as a real nested object rather than a string:
 * the browser era had no choice but to escape it into a `localStorage`
 * value, and a settings file full of escaped JSON would be unreadable by
 * the human it is sitting on disk for.
 */
export const LANGUAGE_PACKS_STORAGE_KEY = "uiLanguagePacks";

/** The format version this build writes. A pack may declare a different
 * (including higher) `formatVersion` and is still accepted — see
 * `parseLanguagePack` — this constant is only what `exportEnglishTemplate`
 * stamps on a freshly generated template. */
export const LANGUAGE_PACK_FORMAT_VERSION = 1;

/**
 * A language pack: the on-disk/JSON shape a user imports to teach the UI
 * a language nobody compiled in. `formatVersion`, `tag`, `name` and
 * `messages` are required and validated by `parseLanguagePack`; every
 * other field, known or not, rides along unvalidated-but-preserved so a
 * pack written for a later format version survives a round trip through
 * this one instead of being silently stripped down to what this build
 * happens to recognise.
 */
export interface LanguagePack {
  [key: string]: unknown;

  /** The format this document was written against. This build only reads
   * versions it knows about; it does not reject a higher one — the
   * unknown-field passthrough below is what keeps a newer pack intact
   * even though this build can't interpret whatever new meaning it adds. */
  formatVersion: number;

  /** BCP 47 language tag. Validated for *shape* only — see
   * `isWellFormedBcp47Tag` — never against a registry: an unregistered
   * private-use tag (`"art-x-sindarin"`) is exactly the kind of language
   * this feature exists to let in. */
  tag: string;

  /** How the language names itself — what a language picker shows. */
  name: string;

  /** English name of the language, for diagnostics/logging only; never
   * shown in place of `name`. */
  englishName?: string;

  /**
   * Metadata for the human translator's own bookkeeping — which
   * catalogue they translated from — and NOTHING else. The lookup chain
   * a missing key falls through is always exactly [active language,
   * English], full stop (user ruling, 2026-09-12). This field must never
   * be read as a fallback target, no matter how tempting it looks when a
   * Bavarian pack is obviously closer to German than to English — see
   * `i18n.ts`'s `translateFor`, which does not import this field at all.
   */
  basedOn?: string;

  /** The pack author's own version string for their translation, distinct
   * from `formatVersion` (the file format) — free-form, not validated. */
  packVersion?: string;

  /** The plural categories the pack's messages were written against
   * (informational). Actual category *selection* at lookup time always
   * goes through `Intl.PluralRules` for `tag`, degrading to `"other"`
   * when the runtime has no data for it — see `i18n.ts`. */
  pluralCategories?: string[];

  /** Dotted message key -> translated string. A pack that translates 12
   * of 150 keys is complete as far as this type is concerned: the other
   * 138 are simply absent, and `translateFor` fills them from English. */
  messages: Record<string, string>;
}

/**
 * Everything `importLanguagePack` reports back about one import, so the
 * caller (Task 7's Settings UI) can show the user what actually happened
 * instead of a bare "imported" toast.
 */
export interface LanguagePackImportReport {
  tag: string;
  name: string;
  /** Keys the pack translates that this build also knows. */
  appliedKeyCount: number;
  /** Keys the pack translates that this build has never heard of — listed
   * in full, not just counted, because they mean the pack targets a
   * different application version and the user may want to know which
   * ones. */
  unknownKeys: string[];
  /** Keys this build has that the pack leaves untranslated. Not an error —
   * this is exactly what the English fallback exists for — so only a
   * count plus a short sample, not the full list. */
  missingKeyCount: number;
  missingKeysSample: string[];
  /** Whether `Intl.PluralRules` has real data for `tag` on this runtime.
   * `false` means plural lookups for this pack always resolve to the
   * `"other"` category. */
  pluralRulesSupported: boolean;
}

/**
 * Every way `parseLanguagePack`/`importLanguagePack` can reject a pack,
 * as a discriminated union instead of free-form prose — the
 * `CreationDiagnostic` pattern (`api.ts`, used by `CatalogBrowser.tsx`'s
 * `describeCreationDiagnostic`) applied to language packs, closing
 * KNOWN_LIMITATIONS.md §67. `SettingsPanel.tsx`'s `describeRejectionReason`
 * maps each `kind` to its own catalogue key so the reason renders in the
 * active UI language *inside* the already-translated "Import rejected: …"
 * sentence, instead of `{reason}` always being this type's English
 * `LanguagePackParseResult.error`/`LanguagePackImportResult.error`, which
 * still exists — untranslated — purely as the fallback a future `kind`
 * this build doesn't recognise would need, and as what tests match against
 * when they don't care about translation.
 */
export type LanguagePackRejectionReason =
  | { kind: "notObject" }
  | { kind: "formatVersionMissing" }
  | { kind: "tagMissing" }
  | { kind: "tagMalformed"; tag: string }
  | { kind: "nameMissing" }
  | { kind: "messagesMissing" }
  | { kind: "messageValueNotString"; key: string; valueType: string }
  | { kind: "englishNameNotString" }
  | { kind: "basedOnNotString" }
  | { kind: "packVersionNotString" }
  | { kind: "pluralCategoriesInvalid" }
  | { kind: "storageFailure"; detail: string };

export type LanguagePackParseResult =
  | { ok: true; pack: LanguagePack }
  | { ok: false; error: string; reason: LanguagePackRejectionReason };

export type LanguagePackImportResult =
  | { ok: true; report: LanguagePackImportReport }
  | { ok: false; error: string; reason: LanguagePackRejectionReason };

/** How many missing-key names `LanguagePackImportReport.missingKeysSample`
 * carries — enough to give a translator a starting point, not so many the
 * report reads like the full list `unknownKeys` deliberately is. */
const MISSING_KEYS_SAMPLE_SIZE = 5;

/**
 * A structural (not registry) check for BCP 47: language, optional
 * extlang/script/region/variant subtags, optional private-use suffix, or
 * a standalone private-use tag (`x-...`) — RFC 5646's `langtag`
 * production minus the rarely-used `extension` singleton production
 * (`-a-...`, `-u-...`), which is deliberately left out: without it, a
 * string like `"xx-not-a-language"` is correctly rejected as not shaped
 * like a language tag, whereas RFC 5646 taken completely literally would
 * accept "a" as a valid (if unregistered) extension singleton and
 * "language" as its subtag. This still happily accepts tags with no
 * registry entry at all — `"tlh"` (Klingon), `"bar"` (Bavarian),
 * `"art-x-sindarin"` — because rejecting an unregistered tag would be
 * exactly the gatekeeping this feature exists to avoid.
 */
const BCP47_PATTERN =
  /^(?:(?:[A-Za-z]{2,3}(?:-[A-Za-z]{3}){0,3}|[A-Za-z]{4,8})(?:-[A-Za-z]{4})?(?:-(?:[A-Za-z]{2}|[0-9]{3}))?(?:-(?:[A-Za-z0-9]{5,8}|[0-9][A-Za-z0-9]{3}))*(?:-x(?:-[A-Za-z0-9]{1,8})+)?|x(?:-[A-Za-z0-9]{1,8})+)$/;

/** What `parseLanguagePack`'s error message shows a user whose tag was
 * rejected, so the fix is obvious without them reading RFC 5646. */
export const BCP47_SHAPE_HINT =
  'a well-formed BCP 47 tag, e.g. "nl-NL", "tlh" (Klingon), "bar" (Bavarian), or "art-x-sindarin" (a private-use tag for anything unregistered)';

export function isWellFormedBcp47Tag(tag: string): boolean {
  return BCP47_PATTERN.test(tag);
}

/**
 * RFC 5646 §4.5's handful of "irregular" grandfathered tags — the ones
 * `BCP47_PATTERN` above has no production for at all (an `i-`/`sgn-`
 * prefix isn't a legal primary subtag under any reading this project
 * uses) — each mapped to its modern IANA-registered replacement.
 * Accepting them isn't this feature's job (an unrecognised tag is simply
 * rejected by `parseLanguagePack`); this table only powers
 * `grandfatheredHint`, which lets a caller be a little kinder about *why*
 * one was rejected when it happens to recognise it. Deliberately small:
 * these are the tags actually listed in the RFC, not a guess at every
 * historical form anyone ever used.
 */
const GRANDFATHERED_TAG_HINTS: Record<string, string> = {
  "i-ami": "ami",
  "i-bnn": "bnn",
  "i-hak": "hak",
  "i-klingon": "tlh",
  "i-lux": "lb",
  "i-navajo": "nv",
  "i-pwn": "pwn",
  "i-tao": "tao",
  "i-tay": "tay",
  "i-tsu": "tsu",
  "sgn-be-fr": "sfb",
  "sgn-be-nl": "vgt",
  "sgn-ch-de": "sgg",
};

/**
 * The modern replacement for a rejected grandfathered tag, or `undefined`
 * when `tag` isn't one of RFC 5646 §4.5's irregular forms (including
 * when it isn't even a string — callers hand this a parsed JSON field of
 * unknown shape). Case-insensitive, matching how the RFC's examples are
 * usually typed by hand.
 */
export function grandfatheredHint(tag: unknown): string | undefined {
  if (typeof tag !== "string") return undefined;
  return GRANDFATHERED_TAG_HINTS[tag.toLowerCase()];
}

/** The English prose for a rejection `reason` — what `LanguagePackParseResult.error`/
 * `LanguagePackImportResult.error` carries, and what `fail()` below always
 * derives its `error` from, so the two can never drift apart. Never call
 * this from the UI: `SettingsPanel.tsx` renders `reason` through its own
 * translated catalogue keys instead (see `LanguagePackRejectionReason`'s
 * doc comment). */
function rejectionReasonMessage(reason: LanguagePackRejectionReason): string {
  switch (reason.kind) {
    case "notObject":
      return "A language pack must be a JSON object.";
    case "formatVersionMissing":
      return '"formatVersion" is required and must be a number.';
    case "tagMissing":
      return '"tag" is required and must be a non-empty string.';
    case "tagMalformed":
      return `"tag" (${JSON.stringify(reason.tag)}) is not ${BCP47_SHAPE_HINT}.`;
    case "nameMissing":
      return '"name" is required and must be a non-empty string.';
    case "messagesMissing":
      return '"messages" is required and must be an object mapping keys to strings.';
    case "messageValueNotString":
      return `"messages.${reason.key}" must be a string, got ${reason.valueType}.`;
    case "englishNameNotString":
      return '"englishName" must be a string when present.';
    case "basedOnNotString":
      return '"basedOn" must be a string when present.';
    case "packVersionNotString":
      return '"packVersion" must be a string when present.';
    case "pluralCategoriesInvalid":
      return '"pluralCategories" must be an array of strings when present.';
    case "storageFailure":
      return `Could not save the change: the browser's storage rejected the write (${reason.detail}).`;
  }
}

function fail(
  reason: LanguagePackRejectionReason,
): { ok: false; error: string; reason: LanguagePackRejectionReason } {
  return { ok: false, error: rejectionReasonMessage(reason), reason };
}

/**
 * Validates the *shape* of a parsed JSON value against the required
 * fields (`formatVersion`, `tag`, `name`, `messages`) and the types of
 * the optional ones when present. Every other top-level field — known to
 * a future format version or not — is carried into the returned `pack`
 * unexamined and unmodified, which is what makes the round trip in
 * `languagePack.test.ts` ("an unknown top-level field survives") hold:
 * this function only ever adds validation, it never subtracts fields.
 */
export function parseLanguagePack(raw: unknown): LanguagePackParseResult {
  if (typeof raw !== "object" || raw === null || Array.isArray(raw)) {
    return fail({ kind: "notObject" });
  }
  const obj = raw as Record<string, unknown>;

  if (typeof obj.formatVersion !== "number" || !Number.isFinite(obj.formatVersion)) {
    return fail({ kind: "formatVersionMissing" });
  }
  if (typeof obj.tag !== "string" || obj.tag.length === 0) {
    return fail({ kind: "tagMissing" });
  }
  if (!isWellFormedBcp47Tag(obj.tag)) {
    return fail({ kind: "tagMalformed", tag: obj.tag });
  }
  if (typeof obj.name !== "string" || obj.name.length === 0) {
    return fail({ kind: "nameMissing" });
  }
  if (typeof obj.messages !== "object" || obj.messages === null || Array.isArray(obj.messages)) {
    return fail({ kind: "messagesMissing" });
  }
  const messagesObj = obj.messages as Record<string, unknown>;
  for (const [key, value] of Object.entries(messagesObj)) {
    if (typeof value !== "string") {
      return fail({ kind: "messageValueNotString", key, valueType: typeof value });
    }
  }
  if (obj.englishName !== undefined && typeof obj.englishName !== "string") {
    return fail({ kind: "englishNameNotString" });
  }
  if (obj.basedOn !== undefined && typeof obj.basedOn !== "string") {
    return fail({ kind: "basedOnNotString" });
  }
  if (obj.packVersion !== undefined && typeof obj.packVersion !== "string") {
    return fail({ kind: "packVersionNotString" });
  }
  if (obj.pluralCategories !== undefined) {
    const categories = obj.pluralCategories;
    if (!Array.isArray(categories) || categories.some((c) => typeof c !== "string")) {
      return fail({ kind: "pluralCategoriesInvalid" });
    }
  }

  return {
    ok: true,
    pack: { ...obj, messages: messagesObj as Record<string, string> } as LanguagePack,
  };
}

/**
 * Whether `Intl.PluralRules` has real category data for `tag` on this
 * runtime, checked via `supportedLocalesOf` rather than by constructing
 * the object and hoping it throws: a locale with no data does not
 * reliably throw (implementations differ, and some silently substitute a
 * default locale's rules instead — which would quietly hand a Sindarin
 * pack English's "one"/"other" split under the Sindarin label, worse than
 * admitting there is no data at all). Never throws itself.
 */
export function pluralRulesSupportedFor(tag: string): boolean {
  try {
    return Intl.PluralRules.supportedLocalesOf(tag).length > 0;
  } catch {
    return false;
  }
}

function computeImportReport(pack: LanguagePack): LanguagePackImportReport {
  const knownKeys = new Set(Object.keys(enMessages));
  const packKeys = Object.keys(pack.messages);
  const unknownKeys = packKeys.filter((key) => !knownKeys.has(key));
  const appliedKeyCount = packKeys.length - unknownKeys.length;
  const missingKeys = [...knownKeys].filter((key) => !(key in pack.messages));

  return {
    tag: pack.tag,
    name: pack.name,
    appliedKeyCount,
    unknownKeys,
    missingKeyCount: missingKeys.length,
    missingKeysSample: missingKeys.slice(0, MISSING_KEYS_SAMPLE_SIZE),
    pluralRulesSupported: pluralRulesSupportedFor(pack.tag),
  };
}

// The installed-packs store, structured exactly like `uiLanguage.ts`'s
// cache: a module-level value seeded lazily from `localStorage` on first
// access rather than at module-evaluation time, because `happy-dom` only
// wires up `window.localStorage` per test file and module evaluation can
// happen before that runs.
let cache: Record<string, LanguagePack> | undefined;

/**
 * The reactive half of the store, mirroring `uiLanguage.ts`'s
 * `subscribers`/`getSnapshot` shape: `useLanguagePacks()` below is how
 * `i18n.ts`'s `useTranslate()` (and anything else that renders installed
 * packs) learns that an import or a removal happened, without the
 * mutating call site — `SettingsPanel.tsx`'s `handleRemovePack`, for
 * instance — having to know who else is mounted or coordinate with any
 * other store to make that visible. `listSnapshot` is invalidated
 * (`undefined`) rather than eagerly recomputed on every mutation, so a
 * `getListSnapshot()` call between two mutations still returns the same
 * cached array reference — `useSyncExternalStore` compares snapshots with
 * `Object.is`, and a fresh array on every call would loop it.
 */
let listSnapshot: readonly LanguagePack[] | undefined;
const subscribers = new Set<() => void>();

function getListSnapshot(): readonly LanguagePack[] {
  if (listSnapshot === undefined) {
    listSnapshot = Object.values(getCache());
  }
  return listSnapshot;
}

function subscribeToPacks(onStoreChange: () => void): () => void {
  subscribers.add(onStoreChange);
  return () => subscribers.delete(onStoreChange);
}

function notifyPackSubscribers(): void {
  listSnapshot = undefined;
  for (const onStoreChange of subscribers) onStoreChange();
}

function readStore(): Record<string, LanguagePack> {
  const stored = getSetting(LANGUAGE_PACKS_STORAGE_KEY);
  // An object in the settings document; a string is what the browser era
  // stored and what a hand-edited file may still say, so it is parsed
  // rather than thrown away.
  let parsed: unknown = stored;
  if (typeof stored === "string") {
    try {
      parsed = JSON.parse(stored) as unknown;
    } catch {
      // Corrupted/hand-edited storage is treated as "no packs installed",
      // the same always-safe-default philosophy `theme.ts`'s `loadThemeId`
      // uses for an unrecognised value, not a thrown error mid-render.
      return {};
    }
  }
  if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) return {};
  // A copy, deliberately: `getCache()` is mutated in place by
  // `importLanguagePack`, and mutating the settings document's own object
  // would leave `setSettingOrThrow` comparing a value against itself and
  // concluding nothing had changed.
  return { ...(parsed as Record<string, LanguagePack>) };
}

function getCache(): Record<string, LanguagePack> {
  if (cache === undefined) {
    cache = readStore();
  }
  return cache;
}

function persist(): void {
  setSettingOrThrow(LANGUAGE_PACKS_STORAGE_KEY, { ...getCache() });
}

/**
 * Resets the module-level cache seeded by `getCache()`, the same reason
 * `uiLanguage.ts` needs `resetUiLanguageForTests()`: clearing
 * `localStorage` in `afterEach` doesn't un-seed this module's cache, so a
 * later test in the same file would still observe whatever an earlier
 * test last installed.
 */
export function resetLanguagePacksForTests(): void {
  cache = undefined;
  listSnapshot = undefined;
}

/** All installed packs, in no particular order. */
export function listLanguagePacks(): LanguagePack[] {
  return Object.values(getCache());
}

/**
 * Reads the installed packs through a shared, module-level store, the
 * same pattern as `useUiLanguage()`/`useProductLanguage()`: every
 * mounted caller re-renders the moment `importLanguagePack` or
 * `removeLanguagePack` changes what's installed, with no prop drilling
 * and no remount required. `i18n.ts`'s `useTranslate()` subscribes
 * through this hook purely for the re-render — it re-resolves the active
 * catalogue itself on every call, so it never reads the returned array —
 * while `SettingsPanel.tsx` (the one place packs are actually listed)
 * reads it directly.
 */
export function useLanguagePacks(): readonly LanguagePack[] {
  return useSyncExternalStore(subscribeToPacks, getListSnapshot);
}

/** A user-installed pack wins over a shipped fun pack with the same tag.
 * Removal reveals the shipped pack again; other absent tags resolve to
 * undefined, which translateFor treats as a straight English fallback.
 * Never merge catalogues: missing keys of an imported replacement still
 * go directly to English, not to the shipped pack or its basedOn. */
export function getLanguagePack(tag: string): LanguagePack | undefined {
  return getCache()[tag] ?? BUNDLED_LANGUAGE_PACKS.find((pack) => pack.tag === tag);
}

/** Shipped packs plus user imports, one option per tag. An import replaces
 * a shipped catalogue, but its shipped picker name remains recognizable.
 * Installed-pack management still uses useLanguagePacks(), never this list. */
export function useAvailableLanguagePacks(): readonly LanguagePack[] {
  const installed = useLanguagePacks();
  return useMemo(() => {
    const byTag = new Map(BUNDLED_LANGUAGE_PACKS.map((pack) => [pack.tag, pack]));
    for (const pack of installed) {
      const shipped = BUNDLED_LANGUAGE_PACKS.find((candidate) => candidate.tag === pack.tag);
      byTag.set(pack.tag, shipped ? { ...pack, name: shipped.name } : pack);
    }
    return [...byTag.values()];
  }, [installed]);
}

/** Unwraps the message from whatever `window.localStorage.setItem` threw
 * — realistically a `DOMException` (`QuotaExceededError` when a pack
 * carries enough unread bulk to blow the quota, this being the only
 * store in the app that accepts arbitrary user-supplied JSON of
 * unbounded size), but caught as `unknown` because nothing guarantees
 * that shape. Same fallback `api.ts`'s `errorMessage` uses. */
function storageFailureDetail(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

/**
 * Validates and installs a pack, keyed by its own `tag` (re-importing the
 * same tag replaces the previous install). Validation failure rejects the
 * import outright with the reason; anything past validation is installed
 * unconditionally — unknown/missing keys are reported, not vetoed, per
 * the brief's "a pack that translates 12 of 150 keys is a legitimate
 * pack" rule.
 *
 * The cache write and the `persist()` call are treated as one
 * transaction: if `persist()` throws — `localStorage.setItem` has no
 * guaranteed success, `QuotaExceededError` being the realistic case here
 * — the in-memory mutation is rolled back (restoring whatever pack used
 * to live under this tag, if any) before the rejection is returned, so a
 * failed import can never be smuggled into storage by a *later*
 * successful one calling `persist()` on the same cache object. Rolled
 * back means unchanged, so subscribers are not notified.
 */
export function importLanguagePack(raw: unknown): LanguagePackImportResult {
  const parsed = parseLanguagePack(raw);
  if (!parsed.ok) return parsed;

  const packs = getCache();
  const tag = parsed.pack.tag;
  const previous = packs[tag];
  packs[tag] = parsed.pack;
  try {
    persist();
  } catch (error) {
    if (previous === undefined) delete packs[tag];
    else packs[tag] = previous;
    return fail({ kind: "storageFailure", detail: storageFailureDetail(error) });
  }
  notifyPackSubscribers();

  return { ok: true, report: computeImportReport(parsed.pack) };
}

/** Removes an installed pack. A no-op if `tag` isn't installed. Removing
 * the *active* pack touches nothing in `uiLanguage.ts` — the stored
 * active tag is left exactly as it was, so re-importing the same pack
 * later restores the language with no trip through the select. What
 * *does* need to happen is every already-mounted `useTranslate()` caller
 * noticing that the pack behind its active tag is gone; `i18n.ts`'s
 * `translateFor` already falls back to English the moment
 * `getLanguagePack` returns `undefined`, but only a render triggers that
 * lookup, and nothing forces one without `notifyPackSubscribers()` below.
 *
 * Same transaction shape as `importLanguagePack`: if `persist()` throws,
 * the deletion is rolled back — the pack it was about to remove is put
 * back — instead of leaving it deleted in the cache for a later,
 * unrelated `persist()` call to finish removing on this call's behalf.
 * There is no `LanguagePackImportResult` to reject through here (this
 * isn't an import), so a failed removal is simply undone; the caller
 * sees the pack still present, which is the removal not having
 * happened. */
export function removeLanguagePack(tag: string): void {
  const packs = getCache();
  if (!(tag in packs)) return;
  const previous = packs[tag];
  delete packs[tag];
  try {
    persist();
  } catch {
    packs[tag] = previous;
    return;
  }
  notifyPackSubscribers();
}

/**
 * Exports the resolved installed or shipped pack — unknown fields and all —
 * so a user can share it or hand-edit and re-import it. `undefined` if
 * neither provides `tag`.
 */
export function exportLanguagePack(tag: string): LanguagePack | undefined {
  const pack = getLanguagePack(tag);
  return pack ? { ...pack } : undefined;
}

/**
 * Exports the compiled-in English catalogue as a ready-to-translate
 * template: every key this build knows, English values, a valid-shaped
 * (if placeholder) `tag`/`name` so the export round-trips cleanly through
 * `parseLanguagePack` unmodified. Without this, writing a pack means
 * reading `messages/en.ts` by hand.
 *
 * The template's `tag`/`name` (`"en"`/`"English"`) are a starting point
 * for a translator to overwrite before sharing or re-importing for real,
 * not a claim that this *is* the built-in English catalogue — an
 * installed pack tagged `"en"` is simply shadowed by the real one
 * (`i18n.ts`'s `translateFor` checks the built-in catalogues before any
 * installed pack), so importing the template unedited is harmless, just
 * pointless.
 */
export function exportEnglishTemplate(): LanguagePack {
  let pluralCategories: string[] = ["other"];
  try {
    pluralCategories = [...new Intl.PluralRules("en").resolvedOptions().pluralCategories];
  } catch {
    // Keep the "other"-only default; every runtime we ship on supports
    // "en", so this branch is belt-and-braces, not expected to run.
  }

  return {
    formatVersion: LANGUAGE_PACK_FORMAT_VERSION,
    tag: "en",
    name: "English",
    englishName: "English",
    packVersion: "1.0.0",
    pluralCategories,
    messages: { ...enMessages },
  };
}


// The settings document can be replaced under this cache — by the
// server's answer arriving after first paint, most of all. Same
// invalidation as `uiLanguage.ts`; `getCache()` re-reads on demand.
subscribeToSettings(() => {
  cache = undefined;
  notifyPackSubscribers();
});
