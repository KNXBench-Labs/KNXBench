/** Keeps the document language correct without rewriting it for every translated component. */
// @vitest-environment happy-dom
import { act, StrictMode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, expect, it, vi } from "vitest";
import { resetSettingsForTests } from "./settingsStore";
import { resetUiLanguageForTests, useUiLanguage } from "./uiLanguage";

let root: Root | undefined;
let host: HTMLDivElement | undefined;
let selectLanguage: ((language: string) => void) | undefined;

function Reader() {
  const [language, setLanguage] = useUiLanguage();
  selectLanguage = setLanguage;
  return <span>{language}</span>;
}

async function renderReaders(count: number) {
  if (!host) {
    host = document.createElement("div");
    document.body.append(host);
    root = createRoot(host);
  }
  await act(async () => {
    root!.render(<StrictMode>{Array.from({ length: count }, (_, id) => <Reader key={id} />)}</StrictMode>);
  });
}

afterEach(async () => {
  if (root) await act(async () => root!.unmount());
  host?.remove();
  root = undefined;
  host = undefined;
  selectLanguage = undefined;
  vi.restoreAllMocks();
  resetSettingsForTests();
  resetUiLanguageForTests();
  document.documentElement.removeAttribute("lang");
});

it("mounting many subscribers applies the document language once, including StrictMode replay", async () => {
  document.documentElement.removeAttribute("lang");
  const write = vi.spyOn(document.documentElement, "setAttribute");
  await renderReaders(128);
  expect(document.documentElement.getAttribute("lang")).toBe("en");
  expect(write.mock.calls.filter(([name]) => name === "lang")).toEqual([["lang", "en"]]);
  // Additional translated children must not rewrite an already correct root attribute.
  await renderReaders(256);
  expect(write.mock.calls.filter(([name]) => name === "lang")).toEqual([["lang", "en"]]);
});

it("does not rewrite a document language that already matches on first mount", async () => {
  document.documentElement.setAttribute("lang", "en");
  const write = vi.spyOn(document.documentElement, "setAttribute");
  await renderReaders(128);
  expect(document.documentElement.getAttribute("lang")).toBe("en");
  expect(write.mock.calls.filter(([name]) => name === "lang")).toEqual([]);
});

it("a language change updates the attribute once and still reaches every mounted subscriber", async () => {
  await renderReaders(128);
  const write = vi.spyOn(document.documentElement, "setAttribute");
  for (const language of ["de", "en", "art-x-sindarin"]) {
    write.mockClear();
    await act(async () => { selectLanguage!(language); });
    expect(document.documentElement.getAttribute("lang")).toBe(language);
    expect(write.mock.calls.filter(([name]) => name === "lang")).toEqual([["lang", language]]);
    expect(Array.from(host!.querySelectorAll("span"), node => node.textContent)).toEqual(Array(128).fill(language));
  }
});

it("a later subscriber repairs a document attribute changed outside the language hook", async () => {
  await renderReaders(1);
  document.documentElement.setAttribute("lang", "fr");
  const write = vi.spyOn(document.documentElement, "setAttribute");
  await renderReaders(2);
  expect(document.documentElement.getAttribute("lang")).toBe("en");
  expect(write.mock.calls.filter(([name]) => name === "lang")).toEqual([["lang", "en"]]);
});
