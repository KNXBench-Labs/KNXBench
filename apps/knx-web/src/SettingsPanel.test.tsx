/** Tests for SettingsPanel's theme/motion/language/accent controls and language-pack UI. */
// @vitest-environment happy-dom
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { parseRules } from "./themeTokens";
import SettingsPanel from "./SettingsPanel";
import { THEMES } from "./theme";
import { MOTION_LEVELS, MOTION_STYLES, useMotion } from "./motion";
import { resetProductLanguageForTests, useProductLanguage } from "./productLanguage";
import { resetUiLanguageForTests } from "./uiLanguage";
import { useTranslate } from "./i18n";
import type { ProductLanguage } from "./api";
import { exportEnglishTemplate, importLanguagePack, resetLanguagePacksForTests } from "./languagePack";
import type { LanguagePack } from "./languagePack";
import { getSetting, initSettings } from "./settingsStore";
import { resetSettingsForTests } from "./settingsStore";
import { rememberProgrammingConsent } from "./programmingConsent";

// The Debug control is exercised against its own server-contract tests;
// SettingsPanel's existing preferences tests must not issue an unrelated GET.
vi.mock("./settingsStore", async (importOriginal) => ({
  ...await importOriginal<typeof import("./settingsStore")>(),
  readPersistedBooleanSetting: vi.fn().mockResolvedValue(false),
}));

let host: HTMLDivElement | undefined;

beforeEach(() => {
  vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true);
});

afterEach(() => {
  host?.remove();
  host = undefined;
  window.localStorage.clear();
  resetSettingsForTests();
  document.documentElement.removeAttribute("data-motion-level");
  document.documentElement.removeAttribute("data-motion-style");
  document.documentElement.removeAttribute("lang");
  resetProductLanguageForTests();
  resetUiLanguageForTests();
  resetLanguagePacksForTests();
  vi.unstubAllGlobals();
});

function dutchPack(overrides: Partial<LanguagePack> = {}): LanguagePack {
  return {
    formatVersion: 1,
    tag: "nl-NL",
    name: "Nederlands",
    englishName: "Dutch",
    messages: {
      "toolbar.save": "Opslaan",
    },
    ...overrides,
  };
}

/** Builds a `File` the way a browser file-input's `FileList` would, for
 * driving `SettingsPanel`'s own import control the same way a user would:
 * pick a file, let the component read it, not a direct call into
 * `languagePack.ts` that bypasses the UI entirely. */
function jsonFile(name: string, data: unknown): File {
  return new File([JSON.stringify(data)], name, { type: "application/json" });
}

// A sibling that never remounts across the test, reading the same
// `uiLanguage.ts` store as SettingsPanel's own select but through
// `useTranslate()` — the call sites the extraction tasks will use. Its
// only job is to prove the "no remount, no prop drilling" requirement:
// SettingsPanel doesn't hand this component anything, yet flipping the
// select changes what it renders.
function Reader() {
  const t = useTranslate();
  return <div data-testid="reader">{t("toolbar.save")}</div>;
}

// Wires SettingsPanel to the real `useMotion()` hook, exactly as App.tsx
// does — a mocked callback would only prove a handler fired, not that the
// control reaches the CSS's actual input (`document.documentElement`'s
// `data-motion-*` attributes).
function Harness(props: { onClose: () => void; productLanguages?: readonly ProductLanguage[] }) {
  const { level, setLevel, style, setStyle } = useMotion();
  const [productLanguage, setProductLanguage] = useProductLanguage();
  return (
    <>
      <Reader />
      <SettingsPanel
        themes={THEMES}
        activeThemeId="bitcoin-defi"
        onSelectTheme={vi.fn()}
        motionStyles={MOTION_STYLES}
        activeMotionStyle={style}
        onSelectMotionStyle={setStyle}
        motionLevels={MOTION_LEVELS}
        activeMotionLevel={level}
        onSelectMotionLevel={setLevel}
        productLanguages={props.productLanguages ?? []}
        activeProductLanguage={productLanguage}
        onSelectProductLanguage={setProductLanguage}
        autosaveEnabled={true}
        onSelectAutosaveEnabled={vi.fn()}
        autosaveIntervalMinutes={5}
        onSelectAutosaveIntervalMinutes={vi.fn()}
        onClose={props.onClose}
      />
    </>
  );
}

async function renderPanel(onClose = vi.fn(), productLanguages?: readonly ProductLanguage[]) {
  host = document.createElement("div");
  document.body.appendChild(host);
  const root = createRoot(host);
  await act(async () => {
    root.render(<Harness onClose={onClose} productLanguages={productLanguages} />);
  });
  return { root, onClose };
}

describe("SettingsPanel", () => {
  it("renders and cleans up in a configured React act environment without hiding warnings", async () => {
    const errors = vi.spyOn(console, "error");
    try {
      const { root } = await renderPanel();
      await act(async () => root.unmount());
      expect(errors.mock.calls).toEqual([]);
    } finally { errors.mockRestore(); }
  });

  it("uses a roomy shared resize shell with a two-column layout that stacks on narrow windows", async () => {
    const { root } = await renderPanel();
    const panel = host!.querySelector<HTMLElement>(".settings-panel")!;
    expect(panel.classList.contains("search-panel-resizable")).toBe(true);
    expect(panel.style.width).toBe("860px");
    expect(panel.style.height).toBe("680px");
    expect(panel.querySelector(".overlay-resize-key")).not.toBeNull();
    const css = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "styles.css"), "utf8");
    const desktop = css.match(/\.settings-panel \{([^}]*)\}/)?.[1] ?? "";
    expect(desktop).toMatch(/grid-template-columns:\s*repeat\(2,\s*minmax\(0,\s*1fr\)\)/);
    expect(css).toMatch(/@media \(max-width: 48rem\) \{\s*\.settings-panel \{\s*grid-template-columns:\s*minmax\(0,\s*1fr\)/);
    const input = css.match(/\.settings-field input:not\(\[type="checkbox"\]\) \{([^}]*)\}/)?.[1] ?? "";
    expect(input).toMatch(/width:\s*100%/);
    expect(input).toMatch(/box-sizing:\s*border-box/);
    const checkbox = css.match(/\.search-panel input\[type="checkbox"\] \{([^}]*)\}/)?.[1] ?? "";
    expect(checkbox).toMatch(/width:\s*1rem/);
    expect(checkbox).toMatch(/min-height:\s*1rem/);
    expect(checkbox).toMatch(/padding:\s*0/);
    act(() => root.unmount());
  });

  it("groups appearance, language/data, and consumed bus preferences", async () => {
    const { root } = await renderPanel();
    const headings = Array.from(host!.querySelectorAll(".settings-section > h3")).map(
      (heading) => heading.textContent,
    );
    expect(headings).toEqual(["Appearance", "Language & data", "Autosave", "Bus & diagnostics", "Debug · device control"]);
    expect(host!.querySelector(".settings-section-debug input[type='checkbox']")).not.toBeNull();
    const gateway = host!.querySelector<HTMLInputElement>(
      '.settings-section-bus input[placeholder="192.0.2.10:3671"]',
    )!;
    await act(async () => {
      Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!.call(
        gateway,
        "192.0.2.10:3671",
      );
      gateway.dispatchEvent(new Event("input", { bubbles: true }));
    });
    expect(getSetting("preferredGateway")).toBe("192.0.2.10:3671");
    expect(host!.querySelector(".settings-section-bus .line-scan-exclusions")).not.toBeNull();
    act(() => root.unmount());
  });

  it("says programming asks every time until a 'don't ask again' is stored", async () => {
    const { root } = await renderPanel();
    const field = host!.querySelector(".settings-field-programming-consent")!;
    expect(field.textContent).toContain("Asked before every programming operation.");
    expect(field.querySelector("button")).toBeNull();
    act(() => root.unmount());
  });

  it("shows a remembered programming consent by stage name and resets it", async () => {
    rememberProgrammingConsent("alpha", "0.1.0-alpha.1");
    const { root } = await renderPanel();
    const field = host!.querySelector(".settings-field-programming-consent")!;
    expect(field.textContent).toContain("Not asked again for Alpha builds.");
    await act(async () => field.querySelector("button")!.click());
    expect(getSetting("programmingConsent")).toBeUndefined();
    expect(field.textContent).toContain("Asked before every programming operation.");
    act(() => root.unmount());
  });

  it("autosave interval is disabled while autosave is off, and both controls call back", async () => {
    const onSelectAutosaveEnabled = vi.fn();
    const onSelectAutosaveIntervalMinutes = vi.fn();
    host = document.createElement("div");
    document.body.appendChild(host);
    const root = createRoot(host);
    await act(async () => {
      root.render(
        <SettingsPanel
          themes={THEMES}
          activeThemeId="bitcoin-defi"
          onSelectTheme={vi.fn()}
          motionStyles={MOTION_STYLES}
          activeMotionStyle="apple"
          onSelectMotionStyle={vi.fn()}
          motionLevels={MOTION_LEVELS}
          activeMotionLevel="standard"
          onSelectMotionLevel={vi.fn()}
          productLanguages={[]}
          activeProductLanguage={null}
          onSelectProductLanguage={vi.fn()}
          autosaveEnabled={false}
          onSelectAutosaveEnabled={onSelectAutosaveEnabled}
          autosaveIntervalMinutes={5}
          onSelectAutosaveIntervalMinutes={onSelectAutosaveIntervalMinutes}
          onClose={vi.fn()}
        />,
      );
    });
    const checkbox = host!.querySelector<HTMLInputElement>(
      ".settings-section-autosave input[type=\"checkbox\"]",
    )!;
    const interval = host!.querySelector<HTMLInputElement>(
      ".settings-section-autosave input[type=\"number\"]",
    )!;
    expect(checkbox.checked).toBe(false);
    expect(interval.disabled).toBe(true);
    await act(async () => {
      checkbox.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });
    expect(onSelectAutosaveEnabled).toHaveBeenCalledWith(true);
    await act(async () => {
      Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, "value")!.set!.call(
        interval,
        "10",
      );
      interval.dispatchEvent(new Event("input", { bubbles: true }));
    });
    expect(onSelectAutosaveIntervalMinutes).toHaveBeenCalledWith(10);
    act(() => root.unmount());
  });

  it("shows the server fallback when no typed settings diagnostic is present", async () => {
    const warning = vi.spyOn(console, "warn").mockImplementation(() => {});
    try {
      vi.stubGlobal("fetch", vi.fn(async () => new Response(JSON.stringify({
        status: "loaded",
        schemaVersion: 1,
        settings: {},
        message: "A future settings event occurred.",
      }), { status: 200, headers: { "Content-Type": "application/json" } })));
      await act(async () => initSettings());

      const { root } = await renderPanel();
      expect(host!.querySelector(".settings-diagnostic")!.textContent)
        .toBe("A future settings event occurred.");
      act(() => root.unmount());
      expect(warning).toHaveBeenCalledExactlyOnceWith("KNXBench: A future settings event occurred.");
    } finally { warning.mockRestore(); }
  });

  it("opens from the gear button (rendered by App.tsx) and shows its five labelled selects", async () => {
    const { root } = await renderPanel();

    expect(host!.querySelector('[role="dialog"]')).not.toBeNull();
    expect(host!.querySelector('select[aria-label="Theme"]')).not.toBeNull();
    expect(host!.querySelector('select[aria-label="Group address notation"]')).toBeNull();
    expect(host!.querySelector('select[aria-label="Motion style"]')).not.toBeNull();
    expect(host!.querySelector('select[aria-label="Motion level"]')).not.toBeNull();
    expect(host!.querySelector('select[aria-label="Product data language"]')).not.toBeNull();
    expect(host!.querySelector('select[aria-label="UI language"]')).not.toBeNull();

    act(() => root.unmount());
  });

  it("closes on Escape (the Overlay shell's handler, not a local one)", async () => {
    const { root, onClose } = await renderPanel();

    const panel = host!.querySelector('[role="dialog"]')!;
    await act(async () => {
      panel.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    });

    expect(onClose).toHaveBeenCalledTimes(1);
    act(() => root.unmount());
  });

  it("closes on a click outside the panel", async () => {
    const { root, onClose } = await renderPanel();

    const overlay = host!.querySelector(".settings-panel")!.parentElement!;
    await act(async () => {
      overlay.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(onClose).toHaveBeenCalledTimes(1);
    act(() => root.unmount());
  });

  it("does not close on a click inside the panel", async () => {
    const { root, onClose } = await renderPanel();

    const panel = host!.querySelector(".settings-panel")!;
    await act(async () => {
      panel.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(onClose).not.toHaveBeenCalled();
    act(() => root.unmount());
  });

  it("changing the motion level select sets document.documentElement's data-motion-level", async () => {
    const { root } = await renderPanel();

    const select = host!.querySelector<HTMLSelectElement>('select[aria-label="Motion level"]')!;
    expect(document.documentElement.getAttribute("data-motion-level")).toBe("standard");

    await act(async () => {
      select.value = "off";
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });

    expect(document.documentElement.getAttribute("data-motion-level")).toBe("off");
    act(() => root.unmount());
  });

  it("changing the motion style select sets document.documentElement's data-motion-style", async () => {
    const { root } = await renderPanel();

    const select = host!.querySelector<HTMLSelectElement>('select[aria-label="Motion style"]')!;
    expect(document.documentElement.getAttribute("data-motion-style")).toBe("apple");

    await act(async () => {
      select.value = "glitch";
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });

    expect(document.documentElement.getAttribute("data-motion-style")).toBe("glitch");
    act(() => root.unmount());
  });

  it("renders one option per product language plus the package default", async () => {
    const languages: ProductLanguage[] = [
      { language: "de-DE", rows: 7385 },
      { language: "en-US", rows: 7327 },
    ];
    const { root } = await renderPanel(vi.fn(), languages);

    const select = host!.querySelector<HTMLSelectElement>('select[aria-label="Product data language"]')!;
    const options = Array.from(select.options).map((o) => ({ value: o.value, text: o.text }));

    expect(options).toEqual([
      { value: "", text: "Package default" },
      { value: "de-DE", text: "de-DE (7385 strings)" },
      { value: "en-US", text: "en-US (7327 strings)" },
    ]);
    expect(select.disabled).toBe(false);

    act(() => root.unmount());
  });

  it("selecting a product language persists it", async () => {
    const languages: ProductLanguage[] = [{ language: "de-DE", rows: 7385 }];
    const { root } = await renderPanel(vi.fn(), languages);

    const select = host!.querySelector<HTMLSelectElement>('select[aria-label="Product data language"]')!;
    await act(async () => {
      select.value = "de-DE";
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });

    expect(getSetting("productLanguage")).toBe("de-DE");

    act(() => root.unmount());
  });

  it("renders a disabled select with an explanation when no product database is installed", async () => {
    const { root } = await renderPanel(vi.fn(), []);

    const select = host!.querySelector<HTMLSelectElement>('select[aria-label="Product data language"]')!;

    expect(select.disabled).toBe(true);
    expect(Array.from(select.options).map((o) => o.text)).toEqual(["No product database installed"]);

    act(() => root.unmount());
  });

  it("lists exactly the catalogues messages/ actually ships, named in themselves", async () => {
    const { root } = await renderPanel();

    const select = host!.querySelector<HTMLSelectElement>('select[aria-label="UI language"]')!;
    const options = Array.from(select.options).map((o) => ({ value: o.value, text: o.text }));

    expect(options).toEqual([
      { value: "en", text: "English" },
      { value: "de", text: "Deutsch" },
    ]);

    act(() => root.unmount());
  });

  it("the UI language select shows the active language", async () => {
    const { root } = await renderPanel();

    const select = host!.querySelector<HTMLSelectElement>('select[aria-label="UI language"]')!;
    expect(select.value).toBe("en");

    act(() => root.unmount());
  });

  it("changing the UI language re-renders an already-mounted sibling and sets document.documentElement.lang", async () => {
    const { root } = await renderPanel();

    const select = host!.querySelector<HTMLSelectElement>('select[aria-label="UI language"]')!;
    const reader = host!.querySelector('[data-testid="reader"]')!;

    expect(reader.textContent).toBe("Save");
    expect(document.documentElement.getAttribute("lang")).toBe("en");

    await act(async () => {
      select.value = "de";
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });

    expect(select.value).toBe("de");
    expect(reader.textContent).toBe("Speichern");
    expect(document.documentElement.getAttribute("lang")).toBe("de");
    expect(getSetting("uiLanguage")).toBe("de");

    act(() => root.unmount());
  });
});

describe("SettingsPanel — language packs (T25 task 7)", () => {
  it("an imported pack appears in the UI-language select, named in itself", async () => {
    importLanguagePack(dutchPack());
    const { root } = await renderPanel();

    const select = host!.querySelector<HTMLSelectElement>('select[aria-label="UI language"]')!;
    const options = Array.from(select.options).map((o) => ({ value: o.value, text: o.text }));

    expect(options).toEqual([
      { value: "en", text: "English" },
      { value: "de", text: "Deutsch" },
      { value: "nl-NL", text: "Nederlands" },
    ]);

    act(() => root.unmount());
  });

  it("activating an installed pack re-renders an already-mounted sibling with its strings", async () => {
    importLanguagePack(dutchPack());
    const { root } = await renderPanel();

    const select = host!.querySelector<HTMLSelectElement>('select[aria-label="UI language"]')!;
    const reader = host!.querySelector('[data-testid="reader"]')!;

    await act(async () => {
      select.value = "nl-NL";
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });

    expect(reader.textContent).toBe("Opslaan");
    act(() => root.unmount());
  });

  it("removing the active pack falls the UI back to English", async () => {
    importLanguagePack(dutchPack());
    const { root } = await renderPanel();

    const select = host!.querySelector<HTMLSelectElement>('select[aria-label="UI language"]')!;
    const reader = host!.querySelector('[data-testid="reader"]')!;

    await act(async () => {
      select.value = "nl-NL";
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });
    expect(reader.textContent).toBe("Opslaan");

    const removeButton = host!.querySelector<HTMLButtonElement>(
      '[aria-label="Remove Nederlands"]',
    )!;
    await act(async () => {
      removeButton.click();
    });

    expect(reader.textContent).toBe("Save");
    act(() => root.unmount());
  });

  // Fix round 1: `handleRemovePack` no longer calls `setUiLanguage("en")`
  // to force the fallback above — the packs store now notifies its own
  // subscribers instead. This test proves the fix didn't just move the
  // symptom: the *stored* active-language preference must survive a
  // removal untouched, not get silently rewritten to "en".
  it("removing the active pack leaves the stored active-language preference untouched", async () => {
    importLanguagePack(dutchPack());
    const { root } = await renderPanel();

    const select = host!.querySelector<HTMLSelectElement>('select[aria-label="UI language"]')!;
    await act(async () => {
      select.value = "nl-NL";
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });
    expect(getSetting("uiLanguage")).toBe("nl-NL");

    const removeButton = host!.querySelector<HTMLButtonElement>(
      '[aria-label="Remove Nederlands"]',
    )!;
    await act(async () => {
      removeButton.click();
    });

    expect(getSetting("uiLanguage")).toBe("nl-NL");

    act(() => root.unmount());
  });

  it("re-importing the removed pack's tag restores the language without re-selecting it", async () => {
    importLanguagePack(dutchPack());
    const { root } = await renderPanel();

    const select = host!.querySelector<HTMLSelectElement>('select[aria-label="UI language"]')!;
    const reader = host!.querySelector('[data-testid="reader"]')!;

    await act(async () => {
      select.value = "nl-NL";
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });
    expect(reader.textContent).toBe("Opslaan");

    const removeButton = host!.querySelector<HTMLButtonElement>(
      '[aria-label="Remove Nederlands"]',
    )!;
    await act(async () => {
      removeButton.click();
    });
    expect(reader.textContent).toBe("Save");

    // Nothing here touches the UI-language select — re-importing the
    // same tag is the only action taken.
    await act(async () => {
      importLanguagePack(dutchPack());
    });

    expect(select.value).toBe("nl-NL");
    expect(reader.textContent).toBe("Opslaan");

    act(() => root.unmount());
  });

  it("importing a partial pack shows the report: applied/missing counts, unknown keys, plural support", async () => {
    const { root } = await renderPanel();

    const input = host!.querySelector<HTMLInputElement>('.language-pack-manager input[type="file"]')!;
    const file = jsonFile(
      "partial.json",
      dutchPack({ messages: { "toolbar.save": "Opslaan", "some.future.key": "???" } }),
    );
    await act(async () => {
      Object.defineProperty(input, "files", { value: [file], configurable: true });
      input.dispatchEvent(new Event("change", { bubbles: true }));
      // File.text() is async; let the microtask queue drain.
      await Promise.resolve();
      await Promise.resolve();
    });

    const report = host!.querySelector(".language-pack-report")!;
    expect(report.textContent).toContain('"Nederlands" imported.');
    expect(report.textContent).toContain("1 string translated.");
    expect(report.textContent).toContain("this build doesn't recognise");
    expect(report.textContent).toContain("some.future.key");
    expect(report.textContent).toContain("Plural forms are supported for this language.");

    act(() => root.unmount());
  });

  // The export-side hint (`exportEnglishTemplate`'s own doc comment, and
  // "shows no packs installed... shadowed tag" above) was already
  // covered. This is the import side: a pack that translates the
  // exported template in place and gets re-imported with its `tag` left
  // at `"en"` — the trap `languagePack.ts`'s `exportEnglishTemplate` warns
  // about — must still report that it's shadowed, not just import silently.
  it("importing a pack tagged \"en\" reports that it is shadowed by the built-in catalogue", async () => {
    const { root } = await renderPanel();

    const input = host!.querySelector<HTMLInputElement>('.language-pack-manager input[type="file"]')!;
    const file = jsonFile(
      "en-in-place.json",
      dutchPack({ tag: "en", name: "English (translated in place, oops)" }),
    );
    await act(async () => {
      Object.defineProperty(input, "files", { value: [file], configurable: true });
      input.dispatchEvent(new Event("change", { bubbles: true }));
      await Promise.resolve();
      await Promise.resolve();
    });

    const report = host!.querySelector(".language-pack-report")!;
    expect(report.textContent).toContain("matches a built-in language");
    expect(report.textContent).toContain('"en"');

    act(() => root.unmount());
  });

  it("a rejected pack shows its reason in words, and never appears in the select", async () => {
    const { root } = await renderPanel();

    const input = host!.querySelector<HTMLInputElement>('.language-pack-manager input[type="file"]')!;
    const file = jsonFile("bad.json", { formatVersion: 1, name: "X", messages: {} });
    await act(async () => {
      Object.defineProperty(input, "files", { value: [file], configurable: true });
      input.dispatchEvent(new Event("change", { bubbles: true }));
      await Promise.resolve();
      await Promise.resolve();
    });

    const report = host!.querySelector(".language-pack-report")!;
    expect(report.textContent).toContain("Import rejected:");
    expect(report.textContent).toMatch(/tag/i);

    const select = host!.querySelector<HTMLSelectElement>('select[aria-label="UI language"]')!;
    expect(Array.from(select.options).map((o) => o.value)).toEqual(["en", "de"]);

    act(() => root.unmount());
  });

  // KNOWN_LIMITATIONS.md §67: the rejection reason used to be
  // `languagePack.ts`'s raw English validation message, dropped verbatim
  // into `languagePack.importReport.rejected`'s `{reason}` slot even when
  // the surrounding sentence was German. It must now come out translated
  // *inside* that sentence, not merely translatable on its own — this
  // switches the UI language to German first, then checks the whole
  // rendered sentence, not an isolated `t("languagePack.rejection....")`
  // call.
  it("§67: a rejected pack's own reason is translated too, inside the translated sentence", async () => {
    const { root } = await renderPanel();

    const uiLanguageSelect = host!.querySelector<HTMLSelectElement>('select[aria-label="UI language"]')!;
    await act(async () => {
      uiLanguageSelect.value = "de";
      uiLanguageSelect.dispatchEvent(new Event("change", { bubbles: true }));
    });

    const input = host!.querySelector<HTMLInputElement>('.language-pack-manager input[type="file"]')!;
    const file = jsonFile("bad.json", {
      formatVersion: 1,
      tag: "xx-not-a-language",
      name: "X",
      messages: {},
    });
    await act(async () => {
      Object.defineProperty(input, "files", { value: [file], configurable: true });
      input.dispatchEvent(new Event("change", { bubbles: true }));
      await Promise.resolve();
      await Promise.resolve();
    });

    const report = host!.querySelector(".language-pack-report")!;
    // The wrapper sentence is German ("Import abgelehnt: …")...
    expect(report.textContent).toContain("Import abgelehnt:");
    // ...and so, now, is the reason inside it.
    expect(report.textContent).toContain('ist kein wohlgeformtes BCP-47-Tag');
    // The old English clause must not survive alongside the German one.
    expect(report.textContent).not.toMatch(/is not a well-formed/i);

    act(() => root.unmount());
  });

  it("a hand-typed grandfathered tag gets a hint at its modern replacement", async () => {
    const { root } = await renderPanel();

    const input = host!.querySelector<HTMLInputElement>('.language-pack-manager input[type="file"]')!;
    const file = jsonFile("klingon.json", {
      formatVersion: 1,
      tag: "i-klingon",
      name: "tlhIngan Hol",
      messages: {},
    });
    await act(async () => {
      Object.defineProperty(input, "files", { value: [file], configurable: true });
      input.dispatchEvent(new Event("change", { bubbles: true }));
      await Promise.resolve();
      await Promise.resolve();
    });

    const report = host!.querySelector(".language-pack-report")!;
    expect(report.textContent).toContain('"i-klingon"');
    expect(report.textContent).toContain('"tlh"');

    act(() => root.unmount());
  });

  it("exporting the English template produces a document the loader accepts", async () => {
    const { root } = await renderPanel();

    let capturedBlob: Blob | undefined;
    const originalCreateObjectURL = URL.createObjectURL;
    const originalRevokeObjectURL = URL.revokeObjectURL;
    URL.createObjectURL = ((blob: Blob) => {
      capturedBlob = blob;
      return "blob:mock";
    }) as typeof URL.createObjectURL;
    URL.revokeObjectURL = vi.fn();

    const exportButton = Array.from(host!.querySelectorAll("button")).find(
      (b) => b.textContent === "Export English template…",
    )!;
    await act(async () => {
      exportButton.click();
    });

    try {
      expect(capturedBlob).toBeDefined();
      const text = await capturedBlob!.text();
      const parsed = JSON.parse(text) as unknown;

      await act(async () => {
        const result = importLanguagePack(parsed);
        expect(result.ok).toBe(true);
      });
      // Matches the exported template's own message content.
      const template = exportEnglishTemplate();
      expect((parsed as { messages: Record<string, string> }).messages).toEqual(template.messages);
    } finally {
      URL.createObjectURL = originalCreateObjectURL;
      URL.revokeObjectURL = originalRevokeObjectURL;
    }

    act(() => root.unmount());
  });

  it("exporting an installed pack round-trips it, unknown fields and all", async () => {
    importLanguagePack(dutchPack({ futureField: "keep me" } as unknown as LanguagePack));
    const { root } = await renderPanel();

    let capturedBlob: Blob | undefined;
    const originalCreateObjectURL = URL.createObjectURL;
    const originalRevokeObjectURL = URL.revokeObjectURL;
    URL.createObjectURL = ((blob: Blob) => {
      capturedBlob = blob;
      return "blob:mock";
    }) as typeof URL.createObjectURL;
    URL.revokeObjectURL = vi.fn();

    try {
      const exportButton = host!.querySelector<HTMLButtonElement>('[aria-label="Export Nederlands"]')!;
      await act(async () => {
        exportButton.click();
      });

      const text = await capturedBlob!.text();
      const parsed = JSON.parse(text) as { futureField?: string };
      expect(parsed.futureField).toBe("keep me");
    } finally {
      URL.createObjectURL = originalCreateObjectURL;
      URL.revokeObjectURL = originalRevokeObjectURL;
    }

    act(() => root.unmount());
  });

  it("shows no packs installed when none are, and a hint about the template's shadowed tag", async () => {
    const { root } = await renderPanel();

    expect(host!.textContent).toContain("No language packs installed.");
    expect(host!.textContent).toContain("shadowed by the built-in English catalogue");

    act(() => root.unmount());
  });
});

/**
 * The accent control under a theme that declares no `[data-accent="…"]`
 * variations (ADR-0022: Cupertino, Neon Grid and Bitcoin DeFi treat the
 * accent as identity). The control is disabled there, and a disabled
 * control that neither looks disabled nor says why is just a control that
 * ignores you.
 */
describe("SettingsPanel's accent control", () => {
  function AccentHarness(props: { activeThemeId: string }) {
    return (
      <SettingsPanel
        appearance={{
          accent: "violet",
          density: "compact",
          setAccent: vi.fn(),
          setDensity: vi.fn(),
        }}
        themes={THEMES}
        activeThemeId={props.activeThemeId}
        onSelectTheme={vi.fn()}
        motionStyles={MOTION_STYLES}
        activeMotionStyle="apple"
        onSelectMotionStyle={vi.fn()}
        motionLevels={MOTION_LEVELS}
        activeMotionLevel="standard"
        onSelectMotionLevel={vi.fn()}
        productLanguages={[]}
        activeProductLanguage={null}
        onSelectProductLanguage={vi.fn()}
        autosaveEnabled={true}
        onSelectAutosaveEnabled={vi.fn()}
        autosaveIntervalMinutes={5}
        onSelectAutosaveIntervalMinutes={vi.fn()}
        onClose={vi.fn()}
      />
    );
  }

  async function renderWithTheme(activeThemeId: string) {
    host = document.createElement("div");
    document.body.appendChild(host);
    const root = createRoot(host);
    await act(async () => {
      root.render(<AccentHarness activeThemeId={activeThemeId} />);
    });
    const select = host!.querySelector<HTMLSelectElement>('select[aria-label="Accent color"]')!;
    return { root, select };
  }

  it("is enabled under System, which resolves to a palette that varies by accent", async () => {
    const { root, select } = await renderWithTheme("system");

    expect(select.disabled).toBe(false);
    expect(select.getAttribute("aria-describedby")).toBeNull();
    expect(host!.textContent).not.toContain("keeps its own accent");

    act(() => root.unmount());
  });

  it("is enabled under a palette that declares accent variations", async () => {
    const { root, select } = await renderWithTheme("porcelain");

    expect(select.disabled).toBe(false);

    act(() => root.unmount());
  });

  it("is disabled under a theme whose accent is its identity", async () => {
    const { root, select } = await renderWithTheme("neon-grid");

    expect(select.disabled).toBe(true);

    act(() => root.unmount());
  });

  // Without this, the sentence explaining the disabled control is on
  // screen and nowhere else: a screen reader announces the select as
  // "unavailable" and stops, and the user is left guessing.
  it("points aria-describedby at the sentence explaining why, when disabled", async () => {
    const { root, select } = await renderWithTheme("cupertino");

    const describedBy = select.getAttribute("aria-describedby");
    expect(describedBy, "the disabled accent select explains itself to nobody").not.toBeNull();
    const hint = host!.ownerDocument.getElementById(describedBy!);
    expect(hint, `aria-describedby="${describedBy}" points at no element`).not.toBeNull();
    expect(hint!.textContent).toBe(
      "This theme keeps its own accent; the accent setting has no effect here.",
    );

    act(() => root.unmount());
  });
});

/**
 * The other half of the same finding, which no DOM assertion can reach:
 * the disabled accent select has to *look* disabled. The UA stylesheet's
 * own `select:disabled` greying is user-agent origin, so every author rule
 * in `styles.css` beats it — `input, select { color: var(--knx-foreground);
 * background: var(--knx-surface) }` did exactly that, and the disabled
 * dropdown rendered identically to a working one.
 *
 * This reads the stylesheet rather than a computed style: the cascade
 * question is "does an author rule for `:disabled` exist at all", and a
 * happy-dom computed style would answer a different, weaker question.
 */
describe("styles.css's disabled-field treatment", () => {
  const css = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "styles.css"), "utf-8");
  const disabledRules = parseRules(css).filter((rule) =>
    rule.selector.split(",").some((part) => /(^|\s)select:disabled\b/.test(part.trim())),
  );

  it("has an author rule for select:disabled", () => {
    expect(
      disabledRules.length,
      "no author rule targets select:disabled, so the UA's greying loses to `input, select`",
    ).toBeGreaterThan(0);
  });

  it("makes it visibly and behaviourally distinct, the same way button:disabled does", () => {
    const declared = new Set(
      disabledRules.flatMap((rule) => rule.declarations.map((d) => d.property)),
    );
    expect([...declared], "a disabled field must read as disabled").toContain("opacity");
    expect([...declared]).toContain("cursor");
  });
});
