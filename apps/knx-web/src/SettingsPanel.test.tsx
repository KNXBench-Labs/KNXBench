// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import SettingsPanel from "./SettingsPanel";
import { THEMES } from "./theme";
import { MOTION_LEVELS, MOTION_STYLES, useMotion } from "./motion";
import { resetProductLanguageForTests, useProductLanguage } from "./productLanguage";
import { resetUiLanguageForTests } from "./uiLanguage";
import { useTranslate } from "./i18n";
import type { ProductLanguage } from "./api";
import { exportEnglishTemplate, importLanguagePack, resetLanguagePacksForTests } from "./languagePack";
import type { LanguagePack } from "./languagePack";

let host: HTMLDivElement | undefined;

afterEach(() => {
  host?.remove();
  host = undefined;
  window.localStorage.clear();
  document.documentElement.removeAttribute("data-motion-level");
  document.documentElement.removeAttribute("data-motion-style");
  document.documentElement.removeAttribute("lang");
  resetProductLanguageForTests();
  resetUiLanguageForTests();
  resetLanguagePacksForTests();
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
  it("opens from the gear button (rendered by App.tsx) and shows its five labelled selects", async () => {
    const { root } = await renderPanel();

    expect(host!.querySelector('[role="dialog"]')).not.toBeNull();
    expect(host!.querySelector('select[aria-label="Theme"]')).not.toBeNull();
    expect(host!.querySelector('select[aria-label="Motion style"]')).not.toBeNull();
    expect(host!.querySelector('select[aria-label="Motion level"]')).not.toBeNull();
    expect(host!.querySelector('select[aria-label="Product data language"]')).not.toBeNull();
    expect(host!.querySelector('select[aria-label="UI language"]')).not.toBeNull();

    root.unmount();
  });

  it("closes on Escape (the Overlay shell's handler, not a local one)", async () => {
    const { root, onClose } = await renderPanel();

    const panel = host!.querySelector('[role="dialog"]')!;
    await act(async () => {
      panel.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    });

    expect(onClose).toHaveBeenCalledTimes(1);
    root.unmount();
  });

  it("closes on a click outside the panel", async () => {
    const { root, onClose } = await renderPanel();

    const overlay = host!.querySelector(".settings-panel")!.parentElement!;
    await act(async () => {
      overlay.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(onClose).toHaveBeenCalledTimes(1);
    root.unmount();
  });

  it("does not close on a click inside the panel", async () => {
    const { root, onClose } = await renderPanel();

    const panel = host!.querySelector(".settings-panel")!;
    await act(async () => {
      panel.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    });

    expect(onClose).not.toHaveBeenCalled();
    root.unmount();
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
    root.unmount();
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
    root.unmount();
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

    root.unmount();
  });

  it("selecting a product language persists it", async () => {
    const languages: ProductLanguage[] = [{ language: "de-DE", rows: 7385 }];
    const { root } = await renderPanel(vi.fn(), languages);

    const select = host!.querySelector<HTMLSelectElement>('select[aria-label="Product data language"]')!;
    await act(async () => {
      select.value = "de-DE";
      select.dispatchEvent(new Event("change", { bubbles: true }));
    });

    expect(window.localStorage.getItem("knx-desktop:product-language")).toBe("de-DE");

    root.unmount();
  });

  it("renders a disabled select with an explanation when no product database is installed", async () => {
    const { root } = await renderPanel(vi.fn(), []);

    const select = host!.querySelector<HTMLSelectElement>('select[aria-label="Product data language"]')!;

    expect(select.disabled).toBe(true);
    expect(Array.from(select.options).map((o) => o.text)).toEqual(["No product database installed"]);

    root.unmount();
  });

  it("lists exactly the catalogues messages/ actually ships, named in themselves", async () => {
    const { root } = await renderPanel();

    const select = host!.querySelector<HTMLSelectElement>('select[aria-label="UI language"]')!;
    const options = Array.from(select.options).map((o) => ({ value: o.value, text: o.text }));

    expect(options).toEqual([
      { value: "en", text: "English" },
      { value: "de", text: "Deutsch" },
    ]);

    root.unmount();
  });

  it("the UI language select shows the active language", async () => {
    const { root } = await renderPanel();

    const select = host!.querySelector<HTMLSelectElement>('select[aria-label="UI language"]')!;
    expect(select.value).toBe("en");

    root.unmount();
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
    expect(window.localStorage.getItem("knx-desktop:ui-language")).toBe("de");

    root.unmount();
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

    root.unmount();
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
    root.unmount();
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
    root.unmount();
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

    root.unmount();
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

    root.unmount();
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

    root.unmount();
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

      const result = importLanguagePack(parsed);
      expect(result.ok).toBe(true);
      // Matches the exported template's own message content.
      const template = exportEnglishTemplate();
      expect((parsed as { messages: Record<string, string> }).messages).toEqual(template.messages);
    } finally {
      URL.createObjectURL = originalCreateObjectURL;
      URL.revokeObjectURL = originalRevokeObjectURL;
    }

    root.unmount();
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

    root.unmount();
  });

  it("shows no packs installed when none are, and a hint about the template's shadowed tag", async () => {
    const { root } = await renderPanel();

    expect(host!.textContent).toContain("No language packs installed.");
    expect(host!.textContent).toContain("shadowed by the built-in English catalogue");

    root.unmount();
  });
});
