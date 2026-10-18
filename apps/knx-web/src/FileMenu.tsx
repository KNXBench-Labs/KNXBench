/** File menu building blocks: a role="menu" list and submenus that fly out to the right. */
import {
  createContext, useCallback, useContext, useEffect, useId, useLayoutEffect, useMemo, useRef, useState,
  type FocusEvent, type KeyboardEvent, type MouseEvent, type PointerEvent, type ReactNode,
} from "react";

// Intent delay before a hovered trigger opens, and the grace period before a
// hover-opened submenu closes again. The grace period is what lets a diagonal
// move from the trigger into the panel survive: the pointer may cross the
// menu's padding or a sibling trigger on the way and the panel still wins.
const HOVER_OPEN_DELAY_MS = 120;
const HOVER_CLOSE_DELAY_MS = 320;
const VIEWPORT_MARGIN_PX = 8;
// Keep in step with `.file-submenu-panel` in styles.css (left/right offset, min-width).
const SUBMENU_GAP_PX = 6;
const MIN_PANEL_WIDTH_PX = 160;

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
const ITEM = '[role="menuitem"]';

/** Enabled items that belong to this menu level, not to a submenu nested inside it. */
function itemsOf(level: HTMLElement): HTMLElement[] {
  return [...level.querySelectorAll<HTMLElement>(ITEM)]
    .filter((el) => el.closest(MENU) === level && !(el instanceof HTMLButtonElement && el.disabled));
}

/**
 * The entries of the File menu are buttons rendered by several components this
 * file does not own. Tagging them here keeps the menu pattern complete the day
 * one of them grows another button. A button inside a `role="group"` (the
 * comparison report) is content, not an entry.
 */
function tagMenuItems(root: HTMLElement) {
  for (const button of root.querySelectorAll<HTMLButtonElement>("button:not([role])")) {
    if (!button.closest('[role="group"]')) button.setAttribute("role", "menuitem");
  }
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
  const scheduleOpen = useCallback((id: string) => {
    cancelPending();
    if (openRef.current?.id === id) return;
    timer.current = setTimeout(() => {
      timer.current = null;
      setOpen({ id, source: "hover" });
    }, HOVER_OPEN_DELAY_MS);
  }, [cancelPending]);
  const scheduleClose = useCallback(() => {
    cancelPending();
    if (openRef.current?.source !== "hover") return;
    timer.current = setTimeout(() => {
      timer.current = null;
      setOpen(null);
    }, HOVER_CLOSE_DELAY_MS);
  }, [cancelPending]);
  useEffect(() => cancelPending, [cancelPending]);

  // Menu items: tagged on every render and whenever a child component adds a button of its own.
  useLayoutEffect(() => {
    const root = ref.current;
    if (!root) return;
    tagMenuItems(root);
    const observer = new MutationObserver(() => tagMenuItems(root));
    observer.observe(root, { childList: true, subtree: true });
    return () => observer.disconnect();
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

  function handleKeyDown(e: KeyboardEvent<HTMLDivElement>) {
    if (e.altKey || e.ctrlKey || e.metaKey || !(e.target instanceof Element)) return;
    const item = e.target.closest<HTMLElement>(ITEM);
    const level = item?.closest<HTMLElement>(MENU);
    if (!item || !level) return;
    const submenuId = level.dataset.fileSubmenuPanel;
    const move = (pick: (items: HTMLElement[], at: number) => HTMLElement | undefined) => {
      const items = itemsOf(level);
      pick(items, items.indexOf(item))?.focus();
      e.preventDefault();
    };
    switch (e.key) {
      case "ArrowDown": return move((items, at) => items[(at + 1) % items.length]);
      case "ArrowUp": return move((items, at) => items[(at - 1 + items.length) % items.length]);
      case "Home": return move((items) => items[0]);
      case "End": return move((items) => items[items.length - 1]);
      case "ArrowRight": {
        const id = item.dataset.fileSubmenuTrigger;
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
        const parentOfOpen = e.key === "Escape" && item.dataset.fileSubmenuTrigger === openRef.current?.id;
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

  function handlePointerOver(e: PointerEvent<HTMLDivElement>) {
    if (e.pointerType !== "mouse" || !(e.target instanceof Element)) return;
    if (e.target.closest("[data-file-submenu-panel]")) return cancelPending();
    const id = e.target.closest<HTMLElement>("[data-file-submenu-trigger]")?.dataset.fileSubmenuTrigger;
    if (id) scheduleOpen(id);
    else scheduleClose();
  }

  // Tab and Shift+Tab out of an open submenu into the rest of the menu close it. Focus that
  // leaves the menu altogether (a picker, a dialog) must not: a report renders inside the panel.
  function handleBlur(e: FocusEvent<HTMLDivElement>) {
    const current = openRef.current;
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
        onPointerOver={handlePointerOver}
        onPointerLeave={(e) => { if (e.pointerType === "mouse") scheduleClose(); }}
        onBlur={handleBlur}
      >
        {children}
      </div>
    </FileMenuContext.Provider>
  );
}

/**
 * Chooses the side a panel opens on. The panel starts to the right of its parent row; when that
 * runs past the viewport it flips to the left if the whole panel fits there, and otherwise stays
 * on the right and gives up width instead of leaving the screen. A panel that would run past the
 * bottom edge moves up. On narrow screens the stylesheet makes the panel an inline expansion (not
 * absolutely positioned) and there is nothing to place.
 */
function placePanel(panel: HTMLElement, anchor: HTMLElement) {
  delete panel.dataset.side;
  panel.style.removeProperty("--file-submenu-shift");
  panel.style.removeProperty("--file-submenu-max-width");
  if (getComputedStyle(panel).position !== "absolute") return;
  const viewportWidth = document.documentElement.clientWidth || window.innerWidth;
  const viewportHeight = window.innerHeight;
  const box = panel.getBoundingClientRect();
  if (box.right > viewportWidth - VIEWPORT_MARGIN_PX) {
    const parent = anchor.getBoundingClientRect();
    if (parent.left - SUBMENU_GAP_PX - box.width >= VIEWPORT_MARGIN_PX) {
      panel.dataset.side = "left";
    } else {
      const room = viewportWidth - VIEWPORT_MARGIN_PX - box.left;
      panel.style.setProperty("--file-submenu-max-width", `${Math.max(MIN_PANEL_WIDTH_PX, room)}px`);
    }
  }
  const overflow = box.bottom - (viewportHeight - VIEWPORT_MARGIN_PX);
  if (overflow > 0) {
    const shift = Math.min(overflow, Math.max(0, box.top - VIEWPORT_MARGIN_PX));
    panel.style.setProperty("--file-submenu-shift", `${-shift}px`);
  }
}

interface FileSubmenuProps {
  readonly id: string;
  readonly label: string;
  readonly children: ReactNode;
}

/**
 * One submenu of the File menu. Its items stay mounted while it is folded (`hidden`), because
 * several of them own dialogs and a comparison report that must outlive the panel.
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

  useLayoutEffect(() => {
    if (!isOpen || !panel.current || !wrapper.current) return;
    const place = () => placePanel(panel.current!, wrapper.current!);
    place();
    window.addEventListener("resize", place);
    return () => window.removeEventListener("resize", place);
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
        role="menuitem"
        className="file-submenu-trigger"
        data-file-submenu-trigger={id}
        // `App` closes the menu on any button activation that reaches its container; opening a
        // submenu is not an action, so the trigger carries the same opt-out the comparison button does.
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
        hidden={!isOpen}
      >
        {children}
      </div>
    </div>
  );
}
