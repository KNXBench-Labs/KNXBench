/** File menu parts: the ARIA menu button, a roving-tabindex menu and flyout submenus. */
import {
  createContext, useCallback, useContext, useEffect, useId, useLayoutEffect, useMemo, useRef, useState,
  type ButtonHTMLAttributes, type CSSProperties, type FocusEvent, type KeyboardEvent, type MouseEvent, type PointerEvent, type ReactNode,
} from "react";
import { appUiScale } from "./appUiScale";

// Intent delay before a hovered trigger opens, and the grace period before a
// hover-opened submenu closes again. The grace period is what lets a diagonal
// move from the trigger into the panel survive: the pointer may cross the
// menu's padding or a sibling trigger on the way and the panel still wins.
const HOVER_OPEN_DELAY_MS = 120;
const HOVER_CLOSE_DELAY_MS = 320;
// Every length in this file is a CSS layout px of the zoomed root, the unit the stylesheet and the
// written caps and shifts use. Readings in visual px (DOMRect, the window size) are divided by the
// application scale first (`layoutGeometry`); see docs/contracts/file-menu.md, "Units under zoom".
const VIEWPORT_MARGIN_PX = 8;
// Keep in step with `.file-submenu-panel` in styles.css (left/right offset).
const SUBMENU_GAP_PX = 6;
// The one minimum width of a flyout. The panel states it as `--file-submenu-min-width` and the
// stylesheet's `min-width` reads that property, so placement and CSS cannot disagree.
export const MIN_PANEL_WIDTH_PX = 220;

/** How a submenu was opened decides who may close it: only a hover open closes by itself. */
type OpenSource = "hover" | "click" | "key";
interface OpenSubmenu { readonly id: string; readonly source: OpenSource }

interface FileMenuState {
  readonly open: OpenSubmenu | null;
  openSubmenu(id: string, source: OpenSource): void;
  closeSubmenu(): void;
  scheduleOpen(id: string): void;
  scheduleClose(): void;
  cancelPending(): void;
}

const FileMenuContext = createContext<FileMenuState | null>(null);

const MENU = '[role="menu"]';
// Entries are declared, never guessed: `FileMenuItem` for buttons the File block owns,
// `useFileMenuItemProps` for the one button a reused component contributes. A button without the
// marker is not an entry: no role, no arrow-key stop. The menu holds entries only; anything
// interactive (the project comparison's result, say) opens in a dialog instead.
const ITEM = "[data-file-menu-item]";
const ITEM_PROPS = Object.freeze({ role: "menuitem", "data-file-menu-item": "" });
const NO_PROPS = Object.freeze({});

const isDisabled = (el: HTMLElement) => el instanceof HTMLButtonElement && el.disabled;

/** Every declared entry of this menu level, not those of a submenu nested inside it. */
function declaredOf(level: HTMLElement): HTMLElement[] {
  return [...level.querySelectorAll<HTMLElement>(ITEM)].filter((el) => el.closest(MENU) === level);
}

/** The declared entries of this level that can take focus. */
function itemsOf(level: HTMLElement): HTMLElement[] {
  return declaredOf(level).filter((el) => !isDisabled(el));
}

/**
 * Roving tabindex: per menu level exactly one entry is a tab stop, the one that last had focus
 * (or the first enabled one), so a level never costs more than one Tab press.
 */
function syncRoving(root: HTMLElement, focused?: Element | null) {
  for (const level of [root, ...root.querySelectorAll<HTMLElement>(MENU)]) {
    const enabled = itemsOf(level);
    const active = (focused && enabled.find((el) => el === focused))
      ?? enabled.find((el) => el.getAttribute("tabindex") === "0") ?? enabled[0];
    for (const el of declaredOf(level)) el.tabIndex = el === active ? 0 : -1;
  }
}

/** Marks a button as an entry of the File menu; outside a File menu it marks nothing. */
export function useFileMenuItemProps(): Readonly<Record<string, string>> {
  return useContext(FileMenuContext) ? ITEM_PROPS : NO_PROPS;
}

/** A button the File block owns, declared as a menu entry. */
export function FileMenuItem(props: ButtonHTMLAttributes<HTMLButtonElement>) {
  return <button {...useFileMenuItemProps()} {...props} />;
}

/**
 * The File control of the menu button: a `<summary>` that announces its popup and whether it is
 * open, and that opens the menu onto its first (ArrowDown, Enter, Space) or last (ArrowUp) entry.
 */
export function FileMenuSummary({ children }: { readonly children: ReactNode }) {
  const ref = useRef<HTMLElement | null>(null);
  const [expanded, setExpanded] = useState(false);
  const pending = useRef<"first" | "last" | null>(null);

  const entries = (owner: HTMLDetailsElement) => {
    const level = [...owner.querySelectorAll<HTMLElement>(MENU)].find((el) => el.parentElement === owner);
    return level ? itemsOf(level) : [];
  };
  const focusEntry = (owner: HTMLDetailsElement, which: "first" | "last") => {
    const items = entries(owner);
    items[which === "first" ? 0 : items.length - 1]?.focus();
  };

  useEffect(() => {
    const owner = ref.current?.closest("details");
    if (!owner) return;
    setExpanded(owner.open);
    const observer = new MutationObserver(() => {
      setExpanded(owner.open);
      const which = pending.current;
      pending.current = null;
      if (which && owner.open) focusEntry(owner, which);
    });
    observer.observe(owner, { attributes: true, attributeFilter: ["open"] });
    return () => observer.disconnect();
  }, []);

  function handleKeyDown(e: KeyboardEvent<HTMLElement>) {
    if ((e.key !== "ArrowDown" && e.key !== "ArrowUp") || e.altKey || e.ctrlKey || e.metaKey) return;
    const owner = ref.current?.closest("details");
    if (!owner) return;
    e.preventDefault();
    owner.open = true;
    focusEntry(owner, e.key === "ArrowDown" ? "first" : "last");
  }

  // Enter and Space reach the summary as a click with detail 0; the toggle follows the click.
  function handleClick(e: MouseEvent<HTMLElement>) {
    pending.current = e.detail === 0 ? "first" : null;
  }

  return (
    <summary ref={ref} aria-haspopup="menu" aria-expanded={expanded} onKeyDown={handleKeyDown} onClick={handleClick}>
      {children}
    </summary>
  );
}

interface FileMenuContentProps {
  readonly children: ReactNode;
  readonly onClick?: (e: MouseEvent<HTMLDivElement>) => void;
}

/** The File menu's list: owns which submenu is open, the keyboard paths and the hover intent. */
export function FileMenuContent({ children, onClick }: FileMenuContentProps) {
  const ref = useRef<HTMLDivElement | null>(null);
  const [open, setOpen] = useState<OpenSubmenu | null>(null);
  const openRef = useRef(open);
  openRef.current = open;
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const cancelPending = useCallback(() => {
    if (timer.current !== null) clearTimeout(timer.current);
    timer.current = null;
  }, []);
  const openSubmenu = useCallback((id: string, source: OpenSource) => {
    cancelPending();
    setOpen({ id, source });
  }, [cancelPending]);
  const closeSubmenu = useCallback(() => {
    cancelPending();
    setOpen(null);
  }, [cancelPending]);
  // Hover belongs to flyouts. An inline submenu is a disclosure: it moves the rows below it, so a
  // hover that follows its opening or folding would land on a different row and swap the submenu.
  // Asked of the rendered layout (the open panel, the panel a hover would open), never of a
  // breakpoint copied into this file.
  const isInline = useCallback((id: string) => {
    const root = ref.current;
    const panel = root?.querySelector<HTMLElement>(`[data-file-submenu-panel="${id}"]`);
    const anchor = root?.querySelector<HTMLElement>(`[data-file-submenu="${id}"]`);
    return !!panel && !!anchor && laysOutInline(panel, anchor);
  }, []);
  const scheduleOpen = useCallback((id: string) => {
    cancelPending();
    if (openRef.current?.id === id) return;
    timer.current = setTimeout(() => {
      timer.current = null;
      const current = openRef.current;
      if ((current && isInline(current.id)) || isInline(id)) return;
      setOpen({ id, source: "hover" });
    }, HOVER_OPEN_DELAY_MS);
  }, [cancelPending, isInline]);
  const scheduleClose = useCallback(() => {
    cancelPending();
    if (openRef.current?.source !== "hover" || isInline(openRef.current.id)) return;
    timer.current = setTimeout(() => {
      timer.current = null;
      setOpen(null);
    }, HOVER_CLOSE_DELAY_MS);
  }, [cancelPending, isInline]);
  useEffect(() => cancelPending, [cancelPending]);

  // A press focuses its target in the (compatibility) mousedown before the click. While a press is in
  // flight a focus move is not a keyboard move out of the submenu: folding the open submenu there
  // would shift the rows, and the pressed row would no longer be under the pointer at the click. A
  // mouse sends mousedown before pointerup; a touch or pen tap sends pointerdown and pointerup first
  // and its mousedown after them, so mousedown arms the guard again. A press abandoned outside the
  // window (no release ever reaches the page) is forgotten when the window loses focus.
  const pressing = useRef(false);
  useEffect(() => {
    const release = () => { pressing.current = false; };
    const releases = ["pointerup", "pointercancel", "mouseup", "click"] as const;
    for (const type of releases) window.addEventListener(type, release, true);
    window.addEventListener("blur", release);
    return () => {
      for (const type of releases) window.removeEventListener(type, release, true);
      window.removeEventListener("blur", release);
    };
  }, []);
  const press = () => { pressing.current = true; };

  // One tab stop per level, re-derived whenever the menu renders (items come and go with the project).
  useLayoutEffect(() => {
    if (ref.current) syncRoving(ref.current, ref.current.contains(document.activeElement) ? document.activeElement : null);
  });

  // The `<details>` closes from several places (Escape, outside click, any activation); a closed
  // menu must reopen with every submenu folded, not with the one that was open.
  useEffect(() => {
    const owner = ref.current?.closest("details");
    if (!owner) return;
    const observer = new MutationObserver(() => {
      if (!owner.open) closeSubmenu();
    });
    observer.observe(owner, { attributes: true, attributeFilter: ["open"] });
    return () => observer.disconnect();
  }, [closeSubmenu]);

  const focusTrigger = (id: string) =>
    ref.current?.querySelector<HTMLElement>(`[data-file-submenu-trigger="${id}"]`)?.focus();

  // Tab leaves the whole menu: it closes, focus rests on the File control and the browser's own Tab
  // move carries on from there (forward to what follows File, backward to what precedes it).
  function closeMenu() {
    const owner = ref.current?.closest("details");
    closeSubmenu();
    if (!owner) return;
    owner.open = false;
    owner.querySelector<HTMLElement>("summary")?.focus();
  }

  function handleKeyDown(e: KeyboardEvent<HTMLDivElement>) {
    pressing.current = false;
    if (e.altKey || e.ctrlKey || e.metaKey || !(e.target instanceof Element)) return;
    // React bubbles through portals: a dialog an entry renders into the document body is not the menu.
    if (!ref.current?.contains(e.target)) return;
    // Tab from anything inside the menu leaves it, whether or not the control is a declared entry.
    if (e.key === "Tab") return closeMenu();
    const item = e.target.closest<HTMLElement>(ITEM);
    const level = item?.closest<HTMLElement>(MENU);
    if (!level) return;
    const submenuId = level.dataset.fileSubmenuPanel;
    const move = (pick: (items: HTMLElement[], at: number) => HTMLElement | undefined) => {
      if (!item) return;
      const items = itemsOf(level);
      const target = pick(items, items.indexOf(item));
      target?.focus();
      if (target && ref.current) syncRoving(ref.current, target);
      e.preventDefault();
    };
    switch (e.key) {
      case "ArrowDown": return move((items, at) => items[(at + 1) % items.length]);
      case "ArrowUp": return move((items, at) => items[(at - 1 + items.length) % items.length]);
      case "Home": return move((items) => items[0]);
      case "End": return move((items) => items[items.length - 1]);
      case "ArrowRight": {
        const id = item?.dataset.fileSubmenuTrigger;
        if (!id) return;
        e.preventDefault();
        openSubmenu(id, "key");
        return;
      }
      case "ArrowLeft":
      case "Escape": {
        // Innermost first: a submenu answers before the menu around it does. Escape on the parent
        // row of an open submenu (focus never entered it, say because every item is disabled)
        // closes that submenu too.
        const parentOfOpen = e.key === "Escape" && item !== null && item.dataset.fileSubmenuTrigger === openRef.current?.id;
        const target = submenuId ?? (parentOfOpen ? item.dataset.fileSubmenuTrigger : undefined);
        if (!target) return;
        e.preventDefault();
        e.stopPropagation();
        closeSubmenu();
        focusTrigger(target);
        return;
      }
    }
  }

  function handleFocus(e: FocusEvent<HTMLDivElement>) {
    if (ref.current && e.target instanceof Element && e.target.matches(ITEM)) syncRoving(ref.current, e.target);
  }

  function handlePointerOver(e: PointerEvent<HTMLDivElement>) {
    if (e.pointerType !== "mouse" || !(e.target instanceof Element)) return;
    if (e.target.closest("[data-file-submenu-panel]")) return cancelPending();
    const id = e.target.closest<HTMLElement>("[data-file-submenu-trigger]")?.dataset.fileSubmenuTrigger;
    if (id) scheduleOpen(id);
    else scheduleClose();
  }

  // Tab and Shift+Tab out of an open submenu into the rest of the menu close it. Focus that
  // leaves the menu altogether (a picker, a dialog) must not.
  function handleBlur(e: FocusEvent<HTMLDivElement>) {
    const current = openRef.current;
    if (pressing.current) return;
    const next = e.relatedTarget;
    const root = ref.current;
    if (!current || current.source === "hover" || !root || !(next instanceof Node) || !root.contains(next)) return;
    const owner = root.querySelector(`[data-file-submenu="${current.id}"]`);
    if (!owner?.contains(next)) closeSubmenu();
  }

  const state = useMemo<FileMenuState>(
    () => ({ open, openSubmenu, closeSubmenu, scheduleOpen, scheduleClose, cancelPending }),
    [open, openSubmenu, closeSubmenu, scheduleOpen, scheduleClose, cancelPending],
  );

  return (
    <FileMenuContext.Provider value={state}>
      <div
        ref={ref}
        className="file-menu-content"
        role="menu"
        onClick={onClick}
        onKeyDown={handleKeyDown}
        onPointerDownCapture={press}
        onMouseDownCapture={press}
        onPointerOver={handlePointerOver}
        onPointerLeave={(e) => { if (e.pointerType === "mouse") scheduleClose(); }}
        onFocus={handleFocus}
        onBlur={handleBlur}
      >
        {children}
      </div>
    </FileMenuContext.Provider>
  );
}

/** Where a flyout goes: a side, with a width cap when the whole panel does not fit there, or inline. */
export type PanelPlacement =
  | { readonly kind: "flyout"; readonly side: "right" | "left"; readonly maxWidth: number | null }
  | { readonly kind: "inline" };

/** Placement geometry, all in CSS layout px. */
interface PlacementInput {
  readonly viewportWidth: number;
  /** Left edge and natural width of the panel in its default place, right of its row. */
  readonly panelLeft: number;
  readonly panelWidth: number;
  /** Left edge of the row the panel belongs to. */
  readonly anchorLeft: number;
}

/**
 * Chooses the side a flyout opens on. It starts to the right of its row; when that runs past the
 * viewport it flips to the left if the whole panel fits there, and otherwise keeps the side with
 * room for at least `MIN_PANEL_WIDTH_PX` and gives up width. When neither side has that minimum
 * (a CSS `min-width` cannot be overruled by a smaller `max-width`) the panel must not fly out at
 * all: it expands inline under its row, as the stylesheet does on narrow screens.
 */
export function choosePanelPlacement({ viewportWidth, panelLeft, panelWidth, anchorLeft }: PlacementInput): PanelPlacement {
  const rightRoom = viewportWidth - VIEWPORT_MARGIN_PX - panelLeft;
  if (panelWidth <= rightRoom) return { kind: "flyout", side: "right", maxWidth: null };
  const leftRoom = anchorLeft - SUBMENU_GAP_PX - VIEWPORT_MARGIN_PX;
  if (panelWidth <= leftRoom) return { kind: "flyout", side: "left", maxWidth: null };
  if (rightRoom >= MIN_PANEL_WIDTH_PX && rightRoom >= leftRoom) return { kind: "flyout", side: "right", maxWidth: rightRoom };
  if (leftRoom >= MIN_PANEL_WIDTH_PX) return { kind: "flyout", side: "left", maxWidth: leftRoom };
  return { kind: "inline" };
}

/** The vertical placement input, in CSS layout px like `PlacementInput`. */
interface ShiftInput {
  readonly viewportHeight: number;
  readonly panelTop: number;
  readonly panelBottom: number;
}

/** How far a panel moves up: past the bottom edge it rises, but never above the top margin. */
export function choosePanelShift({ viewportHeight, panelTop, panelBottom }: ShiftInput): number {
  const overflow = panelBottom - (viewportHeight - VIEWPORT_MARGIN_PX);
  return overflow > 0 ? Math.min(overflow, Math.max(0, panelTop - VIEWPORT_MARGIN_PX)) : 0;
}

type PanelGeometry = PlacementInput & ShiftInput;

/**
 * Converts the browser's readings into placement input. Under the application zoom
 * (`--app-ui-scale` on the root) DOMRect and the window size are visual px, while the minimum,
 * the gap, the margin and every length written back are layout px: each reading is divided by the
 * scale, so all comparisons and outputs share one unit. The panel's left edge is its row's right
 * edge plus the gap, not the panel's own box: the entry animation starts 6 px to the side, and a
 * placement made as the panel opens would otherwise grant those 6 px a second time.
 */
export function layoutGeometry(
  panel: Pick<DOMRect, "top" | "width" | "height">, anchor: Pick<DOMRect, "left" | "right">,
  viewport: { readonly width: number; readonly height: number }, scale: number,
): PanelGeometry {
  return {
    viewportWidth: viewport.width / scale, viewportHeight: viewport.height / scale,
    panelLeft: anchor.right / scale + SUBMENU_GAP_PX, panelWidth: panel.width / scale, anchorLeft: anchor.left / scale,
    panelTop: panel.top / scale, panelBottom: (panel.top + panel.height) / scale,
  };
}

// Written lengths are rounded to a hundredth of a layout px: a division by 0.8 leaves binary noise.
const px = (value: number) => `${Math.round(value * 100) / 100}px`;

/** Undoes the previous placement pass, so a window that grows gives the flyout back. */
function resetPlacement(panel: HTMLElement, anchor: HTMLElement) {
  delete panel.dataset.side;
  delete anchor.dataset.layout;
  panel.style.removeProperty("--file-submenu-shift");
  panel.style.removeProperty("--file-submenu-max-width");
}

/** True where the stylesheet lays the panel out as a flyout; false where it is an inline expansion. */
const isFlyoutLayout = (panel: HTMLElement) => getComputedStyle(panel).position === "absolute";

// `clientWidth` leaves out a vertical scrollbar; like `innerHeight` it is a window size, not zoomed.
function measure(panel: HTMLElement, anchor: HTMLElement): PanelGeometry {
  return layoutGeometry(panel.getBoundingClientRect(), anchor.getBoundingClientRect(),
    { width: document.documentElement.clientWidth || window.innerWidth, height: window.innerHeight }, appUiScale());
}

/**
 * Applies `choosePanelPlacement` to a panel. Every pass first undoes the previous one. A panel
 * that would run past the bottom edge (at its final width) moves up. Where the stylesheet already
 * makes the panel an inline expansion (narrow or short screens) it is not absolutely positioned
 * and there is nothing to place.
 */
function placePanel(panel: HTMLElement, anchor: HTMLElement) {
  resetPlacement(panel, anchor);
  if (!isFlyoutLayout(panel)) return;
  const geometry = measure(panel, anchor);
  const placement = choosePanelPlacement(geometry);
  if (placement.kind === "inline") {
    anchor.dataset.layout = "inline";
    return;
  }
  if (placement.side === "left") panel.dataset.side = "left";
  if (placement.maxWidth !== null) panel.style.setProperty("--file-submenu-max-width", px(placement.maxWidth));
  // A width cap wraps long entries, so the panel grows: the vertical fit is measured after it.
  const shift = choosePanelShift(placement.maxWidth === null ? geometry : measure(panel, anchor));
  if (shift > 0) panel.style.setProperty("--file-submenu-shift", px(-shift));
}

/**
 * Whether a submenu is laid out inline under its row (an expansion that moves the rows below it)
 * instead of beside it. An open panel answers from its rendered style. A folded one is unfolded
 * for the length of this call, measured the way `placePanel` will measure it, and folded again
 * before anything paints, so the answer also covers a panel that only goes inline once opened.
 */
function laysOutInline(panel: HTMLElement, anchor: HTMLElement): boolean {
  if (!panel.hidden) return !isFlyoutLayout(panel);
  panel.hidden = false;
  try {
    resetPlacement(panel, anchor);
    return !isFlyoutLayout(panel) || choosePanelPlacement(measure(panel, anchor)).kind === "inline";
  } finally {
    panel.hidden = true;
  }
}

const PANEL_STYLE = { "--file-submenu-min-width": `${MIN_PANEL_WIDTH_PX}px` } as CSSProperties;

interface FileSubmenuProps {
  readonly id: string;
  readonly label: string;
  readonly children: ReactNode;
}

/**
 * One submenu of the File menu. Its items stay mounted while it is folded (`hidden`), because
 * several of them own dialogs and pickers that must outlive the panel.
 */
export function FileSubmenu({ id, label, children }: FileSubmenuProps) {
  const menu = useContext(FileMenuContext);
  if (!menu) throw new Error("FileSubmenu must be rendered inside FileMenuContent");
  const uid = useId();
  const triggerId = `${uid}-trigger`;
  const panelId = `${uid}-panel`;
  const wrapper = useRef<HTMLDivElement | null>(null);
  const panel = useRef<HTMLDivElement | null>(null);
  const isOpen = menu.open?.id === id;
  const source = isOpen ? menu.open!.source : null;

  // Placement runs on open, on window resize, whenever the panel's own size changes (a panel whose
  // entries change while it is open, late data, a font or theme swap, can be pushed past an edge)
  // and when the application scale changes: a zoom step resizes neither the window nor the panel's
  // layout box, yet every visual reading and the room in layout px change with it.
  useLayoutEffect(() => {
    const el = panel.current;
    const anchor = wrapper.current;
    if (!isOpen || !el || !anchor) return;
    const sizeOf = () => { const box = el.getBoundingClientRect(); return `${box.width}x${box.height}`; };
    let placedSize = "";
    const place = () => { placePanel(el, anchor); placedSize = sizeOf(); };
    place();
    window.addEventListener("resize", place);
    // Placing resizes the panel itself (width clamp, shift); only a size we did not place re-places.
    // The re-placement waits for the next frame: changing the observed element inside the observer
    // callback makes the browser report an undelivered-notifications loop.
    let frame = 0;
    const observer = typeof ResizeObserver === "undefined"
      ? null
      : new ResizeObserver(() => {
        cancelAnimationFrame(frame);
        frame = requestAnimationFrame(() => { if (sizeOf() !== placedSize) place(); });
      });
    observer?.observe(el);
    const zoom = new MutationObserver(place);
    zoom.observe(document.documentElement, { attributes: true, attributeFilter: ["style"] });
    return () => {
      window.removeEventListener("resize", place);
      cancelAnimationFrame(frame);
      observer?.disconnect();
      zoom.disconnect();
      // A closed submenu leaves no layout mark behind: the menu's scrolling follows the open one.
      delete anchor.dataset.layout;
    };
  }, [isOpen]);

  // Opening by keyboard moves focus into the panel; opening by pointer leaves it where it is.
  useEffect(() => {
    if (isOpen && source === "key") panel.current && itemsOf(panel.current)[0]?.focus();
  }, [isOpen, source]);

  function handleClick(e: MouseEvent<HTMLButtonElement>) {
    // A keyboard-synthesised click (Enter, Space) has detail 0 and always lands in the panel.
    if (e.detail === 0) {
      if (isOpen && panel.current) itemsOf(panel.current)[0]?.focus();
      return menu!.openSubmenu(id, "key");
    }
    if (isOpen && source !== "hover") return menu!.closeSubmenu();
    menu!.openSubmenu(id, "click");
  }

  return (
    <div ref={wrapper} className="file-submenu" data-file-submenu={id} data-open={isOpen ? "true" : undefined}>
      <button
        id={triggerId}
        type="button"
        {...ITEM_PROPS}
        className="file-submenu-trigger"
        data-file-submenu-trigger={id}
        // `App` closes the menu on any button activation that reaches its container; opening a
        // submenu is not an action, so submenu triggers are the only entries that carry this opt-out.
        data-menu-stays-open="true"
        aria-haspopup="menu"
        aria-expanded={isOpen}
        aria-controls={panelId}
        onClick={handleClick}
      >
        <span>{label}</span>
        <span className="file-submenu-chevron" aria-hidden="true">›</span>
      </button>
      <div
        ref={panel}
        id={panelId}
        className="file-submenu-panel"
        role="menu"
        aria-labelledby={triggerId}
        data-file-submenu-panel={id}
        style={PANEL_STYLE}
        hidden={!isOpen}
      >
        {children}
      </div>
    </div>
  );
}
