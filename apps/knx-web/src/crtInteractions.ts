/** Provides scoped CRT feedback without consuming or synthesizing native actions. */
// SPDX-License-Identifier: AGPL-3.0-or-later
const ACTIVATION_EVENT = "knxbench:crt-activation";
const FLASH_LIMIT_MS = 120;
const SWEEP_LIMIT_MS = 250;
const EFFECT_GAP_MS = 600;

function milliseconds(value: string): number {
  const match = /^(\d+(?:\.\d+)?)(ms|s)$/.exec(value.trim());
  if (!match) return 0;
  const duration = Number(match[1]) * (match[2] === "s" ? 1000 : 1);
  return Number.isFinite(duration) ? duration : 0;
}

/** Signal an actual manual-save request, not a successful persistence acknowledgment. */
export function requestCrtActivation(element: HTMLElement | null): void {
  element?.dispatchEvent(new Event(ACTIVATION_EVENT, { bubbles: true }));
}

/** Fixed portals do not inherit their anchor's scroll clipping. Intersect it explicitly. */
function visibleBounds(element: HTMLElement, window: Window) {
  const bounds = element.getBoundingClientRect();
  let left = Math.max(0, bounds.left), top = Math.max(0, bounds.top);
  let right = Math.min(window.innerWidth, bounds.right), bottom = Math.min(window.innerHeight, bounds.bottom);
  for (let parent = element.parentElement; parent; parent = parent.parentElement) {
    const style = window.getComputedStyle(parent);
    const clip = parent.getBoundingClientRect();
    if (clip.width <= 0 || clip.height <= 0) continue;
    if (["hidden", "auto", "scroll", "clip"].includes(style.overflowX)) {
      left = Math.max(left, clip.left); right = Math.min(right, clip.right);
    }
    if (["hidden", "auto", "scroll", "clip"].includes(style.overflowY)) {
      top = Math.max(top, clip.top); bottom = Math.min(bottom, clip.bottom);
    }
  }
  return { left, top, width: Math.max(0, right - left), height: Math.max(0, bottom - top) };
}

export function installCrtInteractions(scope: HTMLElement): () => void {
  const document = scope.ownerDocument;
  const window = document.defaultView!;
  const root = document.documentElement;
  const reduced = window.matchMedia("(prefers-reduced-motion: reduce)");
  let flashTarget: HTMLElement | null = null;
  let sweepTarget: HTMLElement | null = null;
  let light: HTMLElement | null = null;
  let flashTimer: number | undefined;
  let sweepTimer: number | undefined;
  let lastFlash = -Infinity;
  let lastSweep = -Infinity;
  let disposed = false;

  function eligible(element: HTMLElement): boolean {
    return scope.contains(element) && element.isConnected
      && !element.closest('[inert], [hidden], [aria-disabled="true"]')
      && !(element instanceof HTMLButtonElement && element.disabled);
  }

  function surface(target: EventTarget | null): HTMLElement | null {
    const element = target instanceof Element ? target.closest<HTMLElement>("[data-crt-surface]") : null;
    return element && ["tree", "row", "save"].includes(element.dataset.crtSurface ?? "") && eligible(element)
      ? element : null;
  }

  function durations(): { flash: number; sweep: number } {
    if (disposed || reduced.matches || root.dataset.motionStyle !== "crt" || root.dataset.motionLevel !== "standard") {
      return { flash: 0, sweep: 0 };
    }
    const style = window.getComputedStyle(root);
    const sweep = Math.min(SWEEP_LIMIT_MS, milliseconds(style.getPropertyValue("--knx-transition-duration")));
    const flash = Math.min(FLASH_LIMIT_MS, sweep, milliseconds(style.getPropertyValue("--knx-feedback-duration")));
    return { flash, sweep };
  }

  function clearFlash(): void {
    window.clearTimeout(flashTimer);
    flashTarget?.removeAttribute("data-crt-flash");
    flashTarget = null;
  }

  function clearSweep(): void {
    window.clearTimeout(sweepTimer);
    light?.remove();
    light = null;
    sweepTarget = null;
  }

  function clearFeedback(): void { clearFlash(); clearSweep(); }

  function flash(element: HTMLElement): void {
    const duration = durations().flash;
    const now = window.performance.now();
    if (!duration || !eligible(element) || now - lastFlash < EFFECT_GAP_MS) return;
    clearFlash();
    lastFlash = now;
    flashTarget = element;
    element.dataset.crtFlash = "true";
    flashTimer = window.setTimeout(clearFlash, duration);
  }

  function sweep(element: HTMLElement | null): void {
    if (element === sweepTarget) return;
    clearSweep();
    const duration = durations().sweep;
    const now = window.performance.now();
    if (!element || element.dataset.crtSurface === "save" || !duration || now - lastSweep < EFFECT_GAP_MS) return;
    const bounds = visibleBounds(element, window);
    if (![bounds.left, bounds.top, bounds.width, bounds.height].every(Number.isFinite)
      || bounds.width <= 0 || bounds.height <= 0) return;
    const rawScale = Number(window.getComputedStyle(root).getPropertyValue("--app-ui-scale")) || 1;
    const scale = rawScale > 0 && Number.isFinite(rawScale) ? rawScale : 1;
    light = document.createElement("span");
    light.className = "crt-interaction-light";
    light.setAttribute("aria-hidden", "true");
    light.setAttribute("inert", "");
    Object.assign(light.style, {
      left: `${bounds.left / scale}px`, top: `${bounds.top / scale}px`,
      width: `${bounds.width / scale}px`, height: `${bounds.height / scale}px`,
    });
    light.style.setProperty("--crt-sweep-width", `${bounds.width / scale}px`);
    const core = document.createElement("span");
    core.className = "crt-beam-core";
    light.append(core);
    document.body.append(light);
    sweepTarget = element;
    lastSweep = now;
    sweepTimer = window.setTimeout(clearSweep, duration);
  }

  function activate(event: Event): void {
    const control = event.target instanceof Element ? event.target.closest<HTMLElement>("[data-crt-activate]") : null;
    if (control && eligible(control)) {
      const element = surface(control);
      if (element) flash(element);
    }
  }

  function requestedActivation(event: Event): void {
    const element = surface(event.target);
    if (element) flash(element);
  }

  function enter(event: Event): void {
    if ((event as PointerEvent).pointerType !== "touch") sweep(surface(event.target));
  }

  function leave(event: Event): void {
    if (surface((event as FocusEvent).relatedTarget) !== sweepTarget) clearSweep();
  }

  // Observe preferences and structure once per shell, never once per row/node.
  const preferences = new MutationObserver(clearFeedback);
  preferences.observe(root, { attributes: true, attributeFilter: [
    "data-motion-level", "data-motion-style", "data-theme", "data-accent", "data-density", "style",
  ] });
  const structure = new MutationObserver(() => {
    if (flashTarget && !eligible(flashTarget)) clearFlash();
    if (sweepTarget && !eligible(sweepTarget)) clearSweep();
  });
  structure.observe(scope, { childList: true, subtree: true, attributes: true,
    attributeFilter: ["disabled", "aria-disabled", "hidden", "inert"] });
  const listeners: [string, EventListener][] = [
    ["click", activate], [ACTIVATION_EVENT, requestedActivation],
    ["pointerover", enter], ["focusin", enter], ["pointerout", leave], ["focusout", leave],
    ["scroll", clearFeedback], ["dragstart", clearFeedback],
  ];
  for (const [type, listener] of listeners) scope.addEventListener(type, listener, true);
  window.addEventListener("resize", clearFeedback);
  window.addEventListener("scroll", clearFeedback, true);
  window.addEventListener("blur", clearFeedback);
  reduced.addEventListener("change", clearFeedback);

  return () => {
    if (disposed) return;
    disposed = true;
    clearFeedback();
    preferences.disconnect();
    structure.disconnect();
    for (const [type, listener] of listeners) scope.removeEventListener(type, listener, true);
    window.removeEventListener("resize", clearFeedback);
    window.removeEventListener("scroll", clearFeedback, true);
    window.removeEventListener("blur", clearFeedback);
    reduced.removeEventListener("change", clearFeedback);
  };
}
