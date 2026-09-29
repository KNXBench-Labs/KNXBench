import { useEffect, useRef, useState } from "react";
import type { KeyboardEvent, ReactNode, RefObject } from "react";
import { useTranslate } from "./i18n";

const VIEWPORT_PADDING = 16;
const RESIZE_STEP = 24;
const MIN_RESIZE_WIDTH = 320;
const MIN_RESIZE_HEIGHT = 256;

export interface OverlayResize {
  width: number;
  height: number;
}

/**
 * The one modal overlay shell (T31, closing D9 / KNOWN_LIMITATIONS.md §20).
 *
 * Owns what every overlay in this application shares: the
 * `.search-overlay`/`.search-panel` structure, `role="dialog"`, dismissal
 * by backdrop click and by `Escape`, initial focus, a focus trap, and
 * focus restoration on close. It owns nothing about lists — highlight
 * state, arrow-key traversal and `Enter` activation genuinely differ
 * between the Search, Command Palette and Catalog Browser dialogs and
 * stay with them.
 */
export const FOCUSABLE_SELECTOR =
  'a[href], button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

function focusableIn(panel: HTMLElement): HTMLElement[] {
  return Array.from(panel.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)).filter(
    (el) => !el.hasAttribute("hidden") && el.getAttribute("aria-hidden") !== "true",
  );
}

export default function Overlay(props: {
  labelledBy?: string;
  label?: string;
  className?: string;
  initialFocusRef?: RefObject<HTMLElement | null>;
  /** Opt-in shared dimensions. CSS also caps the panel at the viewport edge. */
  resizable?: OverlayResize;
  onClose: () => void;
  children: ReactNode;
}) {
  const { labelledBy, label, className, initialFocusRef, resizable, onClose, children } = props;
  const panelRef = useRef<HTMLDivElement | null>(null);
  const pointerStartedInsideRef = useRef(false);
  const t = useTranslate();
  const [size, setSize] = useState(() => ({ width: resizable?.width ?? 0, height: resizable?.height ?? 0 }));

  // Initial focus on mount, restoration on unmount. The dependency list is
  // deliberately empty: this runs once per open, and re-running it on a
  // prop change would yank focus out from under whatever the user is
  // typing into.
  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    const panel = panelRef.current;
    const target = initialFocusRef?.current ?? (panel ? (focusableIn(panel)[0] ?? panel) : null);
    target?.focus();
    return () => {
      previous?.focus?.();
    };
  }, []);

  function resizeFromKeyboard(e: KeyboardEvent<HTMLButtonElement>) {
    const delta = {
      ArrowLeft: [-RESIZE_STEP, 0], ArrowRight: [RESIZE_STEP, 0],
      ArrowUp: [0, -RESIZE_STEP], ArrowDown: [0, RESIZE_STEP],
    }[e.key];
    if (!delta || !panelRef.current) return; // Tab and Escape still reach the shared focus trap.
    e.preventDefault();
    e.stopPropagation();
    // Read the rendered box, not the initial prop: native pointer resizing
    // changes the element's used size without updating React state.
    // The application's root CSS zoom scales DOMRect measurements, but style
    // widths and viewport caps are layout pixels. Keep those units together.
    const configuredScale = Number.parseFloat(
      getComputedStyle(document.documentElement).getPropertyValue("--app-ui-scale"),
    );
    const scale = Number.isFinite(configuredScale) && configuredScale > 0 ? configuredScale : 1;
    const box = panelRef.current.getBoundingClientRect();
    const maxWidth = Math.max(1, window.innerWidth / scale - 2 * VIEWPORT_PADDING);
    const maxHeight = Math.max(1, window.innerHeight / scale - 2 * VIEWPORT_PADDING);
    setSize({
      width: Math.min(maxWidth, Math.max(Math.min(MIN_RESIZE_WIDTH, maxWidth), (box.width / scale || size.width) + delta[0])),
      height: Math.min(maxHeight, Math.max(Math.min(MIN_RESIZE_HEIGHT, maxHeight), (box.height / scale || size.height) + delta[1])),
    });
  }

  function handleKeyDown(e: KeyboardEvent<HTMLDivElement>) {
    if (e.key === "Escape") {
      e.stopPropagation();
      onClose();
      return;
    }
    if (e.key !== "Tab") return;
    const panel = panelRef.current;
    if (!panel) return;
    const items = focusableIn(panel);
    if (items.length === 0) {
      // Nothing to move to, so Tab must not leave the dialog either.
      e.preventDefault();
      return;
    }
    const first = items[0];
    const last = items[items.length - 1];
    const active = document.activeElement as HTMLElement | null;
    const inside = active !== null && panel.contains(active);
    if (e.shiftKey && (!inside || active === first)) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && (!inside || active === last)) {
      e.preventDefault();
      first.focus();
    }
  }

  return (
    <div
      className={resizable ? "search-overlay search-overlay-resizable" : "search-overlay"}
      onPointerDownCapture={(e) => { pointerStartedInsideRef.current = e.target !== e.currentTarget; }}
      onClick={(e) => {
        // A native resize drag can release on the backdrop. Its ensuing click
        // is not an intentional dismissal, even though the click targets us.
        if (e.target === e.currentTarget && !pointerStartedInsideRef.current) onClose();
        pointerStartedInsideRef.current = false;
      }}
    >
      <div
        ref={panelRef}
        className={["search-panel", className, resizable && "search-panel-resizable"].filter(Boolean).join(" ")}
        style={resizable ? { width: `${size.width}px`, height: `${size.height}px` } : undefined}
        role="dialog"
        aria-modal="true"
        aria-labelledby={labelledBy}
        aria-label={label}
        tabIndex={-1}
        onClick={(e) => e.stopPropagation()}
        onKeyDown={handleKeyDown}
      >
        {children}
        {resizable && (
          <button
            type="button"
            className="overlay-resize-key"
            aria-label={t("overlay.resizeHandle")}
            title={t("overlay.resizeHandle")}
            onKeyDown={resizeFromKeyboard}
          ><span aria-hidden="true">⤢</span></button>
        )}
      </div>
    </div>
  );
}
