/** Synthetic, complete v1 palettes for theme-pack contract regressions. */
// SPDX-License-Identifier: AGPL-3.0-or-later
export function themePackFixture() {
  return {
    format: "knxbench-theme", formatVersion: 1, tokenVersion: 1,
    id: "user-blueprint", name: "Blueprint", version: "1.0", colorScheme: "light",
    tokens: {
      "--knx-bg": "#f5f6fa", "--knx-surface": "#ffffff", "--knx-foreground": "#202438",
      "--knx-muted": "#60697d", "--knx-border": "#dce0e9", "--knx-border-hover": "#7041dc",
      "--knx-accent": "#7041dc", "--knx-on-accent": "#ffffff", "--knx-accent-tertiary": "#7041dc",
      "--knx-gradient-primary": "linear-gradient(0deg, #7041dc, #7041dc)",
      "--knx-gradient-display": "linear-gradient(90deg, #7041dc,#7041dc)",
      "--knx-error-color": "#b52c3b", "--knx-warning-color": "#946000", "--knx-success-color": "#137551",
      "--knx-overlay-backdrop": "#18203566", "--knx-overlay-shadow": "#17213916",
      "--knx-shadow-raised": "0px 1px 2px 0px #1721390a", "--knx-shadow-hover": "0px 2px 5px 0px #17213914",
      "--knx-font-heading": '"Inter", sans-serif', "--knx-font-body": '"Inter", sans-serif',
      "--knx-font-mono": '"JetBrains Mono", monospace',
      "--knx-radius-card": "8px", "--knx-radius-button": "6px", "--knx-radius-input": "5px",
      "--knx-backdrop-image": "none", "--knx-backdrop-size": "auto", "--knx-backdrop-mask": "none",
    } as Record<string, string>,
  };
}
