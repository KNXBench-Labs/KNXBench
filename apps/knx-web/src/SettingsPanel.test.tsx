// @vitest-environment happy-dom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, describe, expect, it, vi } from "vitest";
import SettingsPanel from "./SettingsPanel";
import { THEMES } from "./theme";
import { MOTION_LEVELS, MOTION_STYLES, useMotion } from "./motion";
import { resetProductLanguageForTests, useProductLanguage } from "./productLanguage";
import type { ProductLanguage } from "./api";

let host: HTMLDivElement | undefined;

afterEach(() => {
  host?.remove();
  host = undefined;
  window.localStorage.clear();
  document.documentElement.removeAttribute("data-motion-level");
  document.documentElement.removeAttribute("data-motion-style");
  resetProductLanguageForTests();
});

// Wires SettingsPanel to the real `useMotion()` hook, exactly as App.tsx
// does — a mocked callback would only prove a handler fired, not that the
// control reaches the CSS's actual input (`document.documentElement`'s
// `data-motion-*` attributes).
function Harness(props: { onClose: () => void; productLanguages?: readonly ProductLanguage[] }) {
  const { level, setLevel, style, setStyle } = useMotion();
  const [productLanguage, setProductLanguage] = useProductLanguage();
  return (
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
  it("opens from the gear button (rendered by App.tsx) and shows its four labelled selects", async () => {
    const { root } = await renderPanel();

    expect(host!.querySelector('[role="dialog"]')).not.toBeNull();
    expect(host!.querySelector('select[aria-label="Theme"]')).not.toBeNull();
    expect(host!.querySelector('select[aria-label="Motion style"]')).not.toBeNull();
    expect(host!.querySelector('select[aria-label="Motion level"]')).not.toBeNull();
    expect(host!.querySelector('select[aria-label="Product data language"]')).not.toBeNull();

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
});
