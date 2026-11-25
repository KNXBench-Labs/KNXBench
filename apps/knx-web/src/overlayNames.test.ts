/** Guards that every production Overlay names its dialog from a label or a title. */
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

const srcDir = dirname(fileURLToPath(import.meta.url));

/** The opening tag text of every `<Overlay` in `source`, braces balanced so `=>` and JSX props do not end it early. */
function overlayTags(source: string): string[] {
  const tags: string[] = [];
  for (let at = source.indexOf("<Overlay"); at !== -1; at = source.indexOf("<Overlay", at + 1)) {
    if (/[\w]/.test(source[at + "<Overlay".length] ?? "")) continue; // <OverlayFoo
    let depth = 0;
    let end = at;
    for (; end < source.length; end += 1) {
      const c = source[end];
      if (c === "{") depth += 1;
      else if (c === "}") depth -= 1;
      else if (c === ">" && depth === 0 && source[end - 1] !== "=") break;
    }
    tags.push(source.slice(at, end + 1));
  }
  return tags;
}

const files = Object.keys(import.meta.glob("./**/*.tsx"))
  .map(path => path.replace(/^\.\//, ""))
  .filter(path => !path.endsWith(".test.tsx"))
  .sort();

describe("Overlay accessible names (WP-49.4c; Q15 decided 23 Nov 2026)", () => {
  it("finds the Overlay consumers (calibration: the scan is not vacuous)", () => {
    const consumers = files.filter(f => overlayTags(readFileSync(join(srcDir, f), "utf-8")).length > 0);
    // Exact on purpose: a consumer dropping out of the scan must fail. Adding or removing a dialog
    // changes these numbers; update them here in the same change.
    expect(consumers.length, "Overlay consumer files changed: update this count (and the tag count below)").toBe(40);
    const tagCount = consumers.reduce((n, f) => n + overlayTags(readFileSync(join(srcDir, f), "utf-8")).length, 0);
    expect(tagCount, "<Overlay tags changed: update this count (and the file count above)").toBe(43);
    expect(consumers).toContain("RenameWorkbench.tsx");
    expect(overlayTags('<Overlay a={() => 1} b="x">y</Overlay>')).toEqual(['<Overlay a={() => 1} b="x">']);
    expect(overlayTags("<OverlayFoo label=\"x\">")).toEqual([]);
  });

  // The neither/both name-source case is now a COMPILE error via Overlay's
  // exclusive-union `OverlayName` prop type (owner Q15, 23 Nov 2026, WP-49.5), so
  // the source scan that used to check it is removed; the tsc mutants proving the
  // type contract live in the WP-49.5 work record. What types cannot prove —
  // a labelledBy id pointing at nothing — is still checked at runtime below.

  it("points every literal labelledBy at an element id that exists in the same file", () => {
    const dangling: string[] = [];
    for (const file of files) {
      const source = readFileSync(join(srcDir, file), "utf-8");
      for (const tag of overlayTags(source)) {
        const literal = /\blabelledBy="([^"]+)"/.exec(tag)?.[1];
        if (literal && !source.includes(`id="${literal}"`)) dangling.push(`${file}: ${literal}`);
      }
    }
    expect(dangling).toEqual([]);
  });
});
