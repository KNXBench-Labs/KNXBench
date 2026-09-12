import { useEffect, useRef } from "react";
import type { KeyboardEvent, ReactNode, RefObject } from "react";

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
  onClose: () => void;
  children: ReactNode;
}) {
  const { labelledBy, label, className, initialFocusRef, onClose, children } = props;
  const panelRef = useRef<HTMLDivElement | null>(null);

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
    <div className="search-overlay" onClick={onClose}>
      <div
        ref={panelRef}
        className={className ? `search-panel ${className}` : "search-panel"}
        role="dialog"
        aria-modal="true"
        aria-labelledby={labelledBy}
        aria-label={label}
        tabIndex={-1}
        onClick={(e) => e.stopPropagation()}
        onKeyDown={handleKeyDown}
      >
        {children}
      </div>
    </div>
  );
}
