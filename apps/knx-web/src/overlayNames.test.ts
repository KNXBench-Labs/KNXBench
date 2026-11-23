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

describe("Overlay accessible names (WP-49.4c)", () => {
  it("finds the Overlay consumers (calibration: the scan is not vacuous)", () => {
    const consumers = files.filter(f => overlayTags(readFileSync(join(srcDir, f), "utf-8")).length > 0);
    expect(consumers.length).toBeGreaterThanOrEqual(35);
    expect(consumers).toContain("RenameWorkbench.tsx");
    expect(overlayTags('<Overlay a={() => 1} b="x">y</Overlay>')).toEqual(['<Overlay a={() => 1} b="x">']);
    expect(overlayTags("<OverlayFoo label=\"x\">")).toEqual([]);
  });

  it("gives every Overlay exactly one name source: a label or a labelledBy", () => {
    const unnamed: string[] = [];
    for (const file of files) {
      for (const tag of overlayTags(readFileSync(join(srcDir, file), "utf-8"))) {
        const sources = [/\blabel=/.test(tag), /\blabelledBy=/.test(tag)].filter(Boolean).length;
        if (sources !== 1) unnamed.push(`${file}: ${tag.slice(0, 80)}`);
      }
    }
    expect(unnamed, "an Overlay without a name renders an unnamed dialog; the props type cannot enforce it").toEqual([]);
  });

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
