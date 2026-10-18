/** Reads the application root zoom for conversions from visual to layout pixels. */
export function appUiScale(): number {
  const configured = Number.parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--app-ui-scale"));
  return Number.isFinite(configured) && configured > 0 ? configured : 1;
}
