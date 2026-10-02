/** A help tip: one sentence about one control, reachable by pointer and by keyboard alike. */
// ADR-0024's short-form mechanism. Deliberately not a `title` attribute:
// `title` is mouse-only, unstyleable, silent on a disabled control, and
// its delay belongs to the operating system. This is a real focusable
// button with a real `role="tooltip"` description attached to it.

import { useId, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import type { KeyboardEvent } from "react";
import { helpTipAnimates, requestHelpTopic } from "./help";
import type { HelpTopicId } from "./help";
import { useTranslate } from "./i18n";
import type { MessageKey } from "./i18n";

export default function HelpTip(props: { labelKey: MessageKey; textKey: MessageKey; topicId?: HelpTopicId }) {
  const { labelKey, textKey, topicId } = props;
  const t = useTranslate();
  const [open, setOpen] = useState(false);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const bubbleRef = useRef<HTMLSpanElement>(null);
  const [position, setPosition] = useState({ left: 0, top: 0 });
  const bubbleId = `help-tip-${useId()}`;
  useLayoutEffect(() => {
    if (!open) return;
    function place() {
      const trigger = triggerRef.current;
      const bubble = bubbleRef.current;
      if (!trigger || !bubble) return;
      const configured = Number.parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--app-ui-scale"));
      const scale = Number.isFinite(configured) && configured > 0 ? configured : 1;
      const anchor = trigger.getBoundingClientRect();
      const box = bubble.getBoundingClientRect();
      const padding = 8;
      const gap = 6;
      const clamp = (value: number, size: number, extent: number) => Math.max(padding, Math.min(value, extent - size - padding));
      const below = anchor.bottom + gap;
      setPosition({
        left: clamp(anchor.left, box.width, window.innerWidth) / scale,
        top: clamp(below + box.height <= window.innerHeight - padding ? below : anchor.top - box.height - gap, box.height, window.innerHeight) / scale,
      });
    }
    place();
    window.addEventListener("resize", place);
    window.addEventListener("scroll", place, true);
    const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(place);
    if (bubbleRef.current) observer?.observe(bubbleRef.current);
    return () => {
      window.removeEventListener("resize", place);
      window.removeEventListener("scroll", place, true);
      observer?.disconnect();
    };
  }, [open, t]);
  // Read during render rather than latched in an effect: the value is a
  // pure read of two DOM attributes and re-reading it costs nothing, so a
  // motion level changed in Settings applies at this component's next
  // render instead of needing its own subscription. What it does *not* do
  // is re-render this component by itself — the tip is chrome, and chrome
  // that repaints on a settings change is not worth a listener per tip.
  const animates = helpTipAnimates(window, document.documentElement);

  // The permanent local description is visually clipped, not display:none
  // or visibility:hidden. A decorative portal paints identical text outside
  // clipped/transformed ancestors. It does not duplicate the accessible
  // description or need to escape a surrounding modal's inert background.
  function handleKeyDown(e: KeyboardEvent<HTMLButtonElement>) {
    if (e.key !== "Escape" || !open) return;
    // Only while this tip is actually showing. An unconditional stop would
    // eat the Escape that closes a surrounding Overlay dialog.
    e.stopPropagation();
    setOpen(false);
  }

  const bubbleClass = [
    "help-tip-bubble",
    open ? "is-open" : "",
    animates ? "" : "is-still",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <span className="help-tip">
      <button
        ref={triggerRef}
        type="button"
        className="help-tip-trigger"
        aria-label={t(labelKey)}
        aria-describedby={bubbleId}
        data-help-topic={topicId}
        onFocus={() => setOpen(true)}
        onBlur={() => setOpen(false)}
        onPointerEnter={() => setOpen(true)}
        onPointerLeave={() => setOpen(false)}
        // A topic-aware tip keeps its short bubble on focus, while a
        // deliberate click/Enter opens the long-form topic in the panel.
        onClick={() => { setOpen(true); if (topicId) requestHelpTopic(topicId); }}
        onKeyDown={handleKeyDown}
      >
        <span aria-hidden="true">?</span>
      </button>
      <span id={bubbleId} role="tooltip" className="help-tip-description">
        {t(textKey)}
      </span>
      {createPortal(<span ref={bubbleRef} id={`${bubbleId}-visual`} aria-hidden="true" className={bubbleClass} style={open ? position : { left: 0, top: 0 }}>{t(textKey)}</span>, document.body)}
    </span>
  );
}
