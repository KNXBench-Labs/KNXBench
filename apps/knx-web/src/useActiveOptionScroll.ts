/** Keeps keyboard-highlighted rows visible without stealing combobox focus. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { useLayoutEffect, useRef } from "react";

export function useActiveOptionScroll(index: number, items: readonly unknown[], active = true) {
  const optionRef = useRef<HTMLLIElement>(null);
  useLayoutEffect(() => {
    if (active) optionRef.current?.scrollIntoView?.({ block: "nearest", inline: "nearest" });
  }, [index, items, active]);
  return optionRef;
}
