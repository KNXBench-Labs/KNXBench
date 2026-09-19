/** A help tip: one sentence about one control, reachable by pointer and by keyboard alike. */
// ADR-0024's short-form mechanism. Deliberately not a `title` attribute:
// `title` is mouse-only, unstyleable, silent on a disabled control, and
// its delay belongs to the operating system. This is a real focusable
// button with a real `role="tooltip"` description attached to it.

import { useId, useState } from "react";
import type { KeyboardEvent } from "react";
import { helpTipAnimates } from "./help";
import { useTranslate } from "./i18n";
import type { MessageKey } from "./i18n";

export default function HelpTip(props: { labelKey: MessageKey; textKey: MessageKey }) {
  const { labelKey, textKey } = props;
  const t = useTranslate();
  const [open, setOpen] = useState(false);
  const bubbleId = `help-tip-${useId()}`;
  // Read during render rather than latched in an effect: the value is a
  // pure read of two DOM attributes and re-reading it costs nothing, so a
  // motion level changed in Settings applies at this component's next
  // render instead of needing its own subscription. What it does *not* do
  // is re-render this component by itself — the tip is chrome, and chrome
  // that repaints on a settings change is not worth a listener per tip.
  const animates = helpTipAnimates(window, document.documentElement);

  // The bubble is in the DOM whether or not it is visible, and
  // `aria-describedby` points at it permanently. That is the whole
  // accessibility argument for this component over a `title`: a screen
  // reader gets the description when focus lands on the trigger, with no
  // hover to simulate and no visibility state to guess at. Visibility is
  // therefore purely a sighted-user concern and lives in CSS.
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
        type="button"
        className="help-tip-trigger"
        aria-label={t(labelKey)}
        aria-describedby={bubbleId}
        onFocus={() => setOpen(true)}
        onBlur={() => setOpen(false)}
        onPointerEnter={() => setOpen(true)}
        onPointerLeave={() => setOpen(false)}
        // Opens, never toggles. A click is preceded by a pointer-enter and
        // a focus that have both already opened the bubble, so a toggle
        // here would read as "clicking the help button hides the help".
        // Escape, blur and pointer-leave are the three ways it closes.
        onClick={() => setOpen(true)}
        onKeyDown={handleKeyDown}
      >
        <span aria-hidden="true">?</span>
      </button>
      <span id={bubbleId} role="tooltip" className={bubbleClass}>
        {t(textKey)}
      </span>
    </span>
  );
}
