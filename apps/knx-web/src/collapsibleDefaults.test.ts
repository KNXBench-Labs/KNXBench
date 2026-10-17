/** Keep native content disclosures opt-in across the application. */
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, it } from "vitest";

function sources(directory: string): string[] {
  return readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
    const path = join(directory, entry.name);
    return entry.isDirectory() ? sources(path)
      : entry.name.endsWith(".tsx") && !entry.name.includes(".test.") ? [path] : [];
  });
}

it("does not eagerly open native content disclosures", () => {
  const paths = sources(fileURLToPath(new URL(".", import.meta.url)));
  expect(paths.length).toBeGreaterThan(0);
  const eager = paths.flatMap(path => [...readFileSync(path, "utf8").matchAll(/<details\b(?:\{[^}]*\}|[^>])*>/g)]
    .filter(([tag]) => /\sopen(?=\s|>|=\{true\}|="true")/.test(tag))
    .map(([tag]) => `${path}: ${tag}`));
  expect(eager).toEqual([]);
});
