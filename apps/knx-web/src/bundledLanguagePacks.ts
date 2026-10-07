/** Shipped fun-language packs: available without importing or modifying saved settings. */
import type { LanguagePack } from "./languagePack";
import { messages as barMessages } from "./messages/bar";
import { messages as tlhMessages } from "./messages/tlh";

export const BUNDLED_LANGUAGE_PACKS: readonly LanguagePack[] = [
  {
    formatVersion: 1,
    tag: "bar",
    name: "Boarisch",
    englishName: "Bavarian (playful)",
    basedOn: "de",
    packVersion: "1.0.0",
    messages: barMessages,
  },
  {
    formatVersion: 1,
    tag: "tlh",
    name: "Klingonisch/Klingon",
    englishName: "Klingon (playful)",
    basedOn: "en",
    packVersion: "1.0.0",
    messages: tlhMessages,
  },
];
