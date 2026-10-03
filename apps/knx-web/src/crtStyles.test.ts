/** Verifies application-owned CRT styling and explicit motion guards. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, it } from "vitest";

const css = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "styles.css"), "utf8");

it("defines a CRT motion style without injecting execution into theme packs", () => {
  expect(css).toContain(':root[data-motion-style="crt"]');
  expect(css).toMatch(/data-motion-level="standard"[^}]+--knx-transition-duration: 250ms/s);
  expect(css).toContain("@keyframes knx-crt-flash");
  expect(css).toContain("@keyframes knx-crt-sweep");
});

it("keeps the production CRT layer explicitly motionless unless CSS permits it", () => {
  const layer = css.split("/* CRT interaction feedback")[1];
  expect(layer).toBeDefined();
  expect(layer).toContain("transition-property: none");
  expect(layer).toContain("animation-name: none");
  expect(layer).toContain("@media (prefers-reduced-motion: no-preference)");
  expect(layer).toContain("pointer-events: none");
});
