/** Reads which shared button variant a control paints as, and how separators draw. */
import type { Locator } from "@playwright/test";

export type ControlRole = "primary" | "secondary" | "quiet" | "danger" | "other";

export interface ControlPaint {
  role: ControlRole;
  text: string;
  ink: string;
  background: string;
  edge: string;
  width: number;
  height: number;
}

/**
 * Classifies a control by its computed paint against the active theme's own
 * tokens, so the same assertion holds in every shipped theme: danger draws
 * error ink, primary an accent fill (or the theme's primary gradient) under
 * on-accent ink, secondary a surface fill with the border colour, quiet no
 * fill and no edge at rest. The box is the layout box, unaffected by transforms.
 */
export async function controlPaint(control: Locator): Promise<ControlPaint> {
  return control.evaluate((node) => {
    const element = node as HTMLElement;
    const probe = document.createElement("i");
    document.body.append(probe);
    const resolve = (value: string) => { probe.style.color = value; return getComputedStyle(probe).color; };
    const alpha = (color: string) => { const parts = color.match(/[\d.]+/g)!.map(Number); return parts[3] ?? 1; };
    const token = (name: string) => resolve(`var(${name})`);
    const tokens = {
      error: token("--knx-error-color"), accent: token("--knx-accent"), onAccent: token("--knx-on-accent"),
      surface: token("--knx-surface"), border: token("--knx-border"), foreground: token("--knx-foreground"),
    };
    probe.remove();
    const style = getComputedStyle(element);
    const ink = style.color;
    const fill = style.backgroundColor;
    const image = style.backgroundImage;
    const edge = style.borderTopColor;
    let role: ControlRole = "other";
    if (ink === tokens.error) role = "danger";
    else if (ink === tokens.onAccent && (fill === tokens.accent || image.includes("gradient"))) role = "primary";
    else if (alpha(fill) === 0 && alpha(edge) === 0 && image === "none") role = "quiet";
    else if (fill === tokens.surface && edge === tokens.border && image === "none") role = "secondary";
    return {
      role, text: (element.textContent ?? "").trim(), ink, background: `${fill} ${image}`, edge,
      width: element.offsetWidth, height: element.offsetHeight,
    };
  });
}

/** WCAG-style contrast between a separator's painted line and the pane colour behind it. */
export async function separatorLine(separator: Locator) {
  return separator.evaluate((node) => {
    const probe = document.createElement("div");
    document.body.append(probe);
    const rgb = (value: string) => {
      probe.style.color = value;
      const m = getComputedStyle(probe).color.match(/[\d.]+/g)!.map(Number);
      return { r: m[0], g: m[1], b: m[2], a: m[3] ?? 1 };
    };
    const opaque = (element: Element | null): { r: number; g: number; b: number } => {
      if (!element) return { r: 255, g: 255, b: 255 };
      const c = rgb(getComputedStyle(element).backgroundColor);
      if (c.a >= 0.999) return c;
      const under = opaque(element.parentElement);
      return { r: c.r * c.a + under.r * (1 - c.a), g: c.g * c.a + under.g * (1 - c.a), b: c.b * c.a + under.b * (1 - c.a) };
    };
    const line = getComputedStyle(node, "::before");
    const paint = rgb(line.backgroundColor);
    const behind = opaque(node.parentElement);
    const area = rgb(getComputedStyle(node).backgroundColor);
    probe.remove();
    const mixed = { r: paint.r * paint.a + behind.r * (1 - paint.a), g: paint.g * paint.a + behind.g * (1 - paint.a), b: paint.b * paint.a + behind.b * (1 - paint.a) };
    const lum = ({ r, g, b }: { r: number; g: number; b: number }) => {
      const c = [r, g, b].map((v) => { const s = v / 255; return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4; });
      return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
    };
    const [hi, lo] = [lum(mixed), lum(behind)].sort((x, y) => y - x);
    const box = node.getBoundingClientRect();
    const own = Number.parseFloat(line.height === "auto" ? "0" : line.height);
    return { contrast: (hi + 0.05) / (lo + 0.05), thickness: Math.min(own, box.height), content: line.content, paint: line.backgroundColor, area: area.a, width: box.width };
  });
}
