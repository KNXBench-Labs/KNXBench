import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { describe, expect, it } from "vitest";

/**
 * §20 of the overlay-shell plan lifted its own rule with prose: "a third
 * overlay is added" would mean go extract a shared shell. Prose does not
 * fail a build, so nobody noticed when a fourth and fifth consumer landed
 * before the extraction happened. This guard makes the next copy-paste of
 * the `.search-overlay` markup fail the test suite instead of quietly
 * accumulating: `Overlay.tsx` is allowed to own that class name, nothing
 * else under `src` is.
 */

const NEEDLE = "search-overlay";
const ALLOWED_FILE = "Overlay.tsx";

describe("overlay shell copy-paste guard", () => {
  it(`only ${ALLOWED_FILE} mentions "${NEEDLE}"`, () => {
    const srcDir = dirname(fileURLToPath(import.meta.url));
    // Glob is used only to enumerate filenames (the object's keys); no
    // module is ever imported, so this doesn't run into the "?raw resolves
    // to the empty string" trap — file contents are read via node:fs below.
    const tsxModules = import.meta.glob("./**/*.tsx");
    const relativePaths = Object.keys(tsxModules)
      .map((path) => path.replace(/^\.\//, ""))
      .filter((path) => !path.endsWith(".test.tsx"))
      .sort();

    const offenders = relativePaths.filter((relativePath) => {
      const contents = readFileSync(join(srcDir, relativePath), "utf-8");
      return contents.includes(NEEDLE) && relativePath !== ALLOWED_FILE;
    });

    expect(
      offenders,
      `found "${NEEDLE}" copy-pasted into: ${offenders.join(", ")}. ` +
        `Import Overlay from "./Overlay" instead of hand-rolling another one.`,
    ).toEqual([]);

    // Sanity check: if the scan above silently found nothing at all (e.g. a
    // future refactor breaks the glob or the file list), this would still
    // catch it — Overlay.tsx must be found and must contain the needle.
    expect(relativePaths).toContain(ALLOWED_FILE);
    const overlayContents = readFileSync(join(srcDir, ALLOWED_FILE), "utf-8");
    expect(overlayContents).toContain(NEEDLE);
  });
});
