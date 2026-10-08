/** Guards the German catalogue's informal du-form: no formal Sie/Ihr address may creep back in. */
import { describe, expect, it } from "vitest";
import { messages } from "./messages/de";

// User decision 2026-10-08: the German UI says "du". The pronoun "Sie"
// (she/it/they, e.g. "Sie bleibt unverändert") stays legal; what is refused
// is a possessive or dative of the formal address, a verb directly followed
// by "Sie" ("Wählen Sie", "liegen Sie"), and "Sie" after a conjunction or
// relative pronoun ("wenn Sie", "die Sie"), which in this catalogue only
// ever means the reader.
const FORMAL = [
  /\b(Ihr|Ihre|Ihren|Ihrem|Ihrer|Ihres|Ihnen)\b/u,
  /\p{L}+en Sie\b/u,
  /\b(wenn|wie|bevor|dass|ob|sobald|die|den|das|für|wo|was|und) Sie\b/u,
];

function formalAddress(catalogue: Record<string, string>): string[] {
  return Object.entries(catalogue)
    .filter(([, text]) => FORMAL.some((pattern) => pattern.test(text)))
    .map(([key]) => key);
}

describe("German catalogue address", () => {
  it("uses the informal du-form throughout", () => {
    expect(formalAddress(messages)).toEqual([]);
  });

  it("refuses the formal forms it is meant to catch and keeps the pronoun", () => {
    expect(formalAddress({
      a: "Wählen Sie einen Knoten.",
      b: "Ihre Sitzung ist beendet.",
      c: "wenn Sie es speichern",
      d: "nach Ihrer Bestätigung",
      e: "Sie bleibt unverändert, bis du sie bearbeitest.",
      f: "Die Telegramme liegen im Puffer. Sie behalten ihre Adressen.",
    })).toEqual(["a", "b", "c", "d"]);
  });
});
