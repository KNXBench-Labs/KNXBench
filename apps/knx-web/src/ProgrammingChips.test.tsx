/** Bounded, locale-owned chip presentation without repeated per-device formatting. */
import { renderToStaticMarkup } from "react-dom/server";
import { expect, it, vi } from "vitest";
import { createProgrammingChips } from "./ProgrammingChips";
import { translateFor, type Translate } from "./i18n";
const getTranslator = (language: "en" | "de"): Translate => (key, params) => translateFor(language, key, params);
import type { PendingProgramming } from "./bindings/PendingProgramming";

const none: PendingProgramming = { address: false, application: false, parameters: false, group_addresses: false };
const every: PendingProgramming = { address: true, application: true, parameters: true, group_addresses: true };

it("prepares a repeated state once, independent of the number or identity of devices", () => {
  const t = vi.fn(getTranslator("en"));
  const cell = createProgrammingChips(t, "en");
  const first = cell({ pending_programming: every });
  const calls = t.mock.calls.length;
  for (let i = 0; i < 5000; i++) expect(cell({ pending_programming: { ...every } })).toBe(first);
  expect(t.mock.calls.length).toBe(calls);
  expect(cell({ pending_programming: none })).not.toBe(first);
  expect(renderToStaticMarkup(first)).toContain('data-part="group_addresses"');
});

it("keeps the sixteen states, missing state and a changed locale distinct", () => {
  const cell = createProgrammingChips(getTranslator("en"), "en");
  const views = new Set();
  for (let bits = 0; bits < 16; bits++) {
    const state = { address: !!(bits & 1), application: !!(bits & 2), parameters: !!(bits & 4), group_addresses: !!(bits & 8) };
    const view = cell({ pending_programming: state }); views.add(view);
    expect(cell({ pending_programming: { ...state } })).toBe(view);
  }
  views.add(cell({}));
  expect(views.size).toBe(17);
  const unknown = renderToStaticMarkup(cell({}));
  const clean = renderToStaticMarkup(cell({ pending_programming: none }));
  expect(unknown).toContain('data-state="unknown"');
  expect(clean).toContain('data-state="clean"');
  const en = renderToStaticMarkup(cell({ pending_programming: every }));
  const de = renderToStaticMarkup(createProgrammingChips(getTranslator("de"), "de")({ pending_programming: every }));
  expect(en).toContain("Application"); expect(de).toContain("Applikation");
});
