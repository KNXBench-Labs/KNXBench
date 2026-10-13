/** Issue #8: searchable group-address combobox that marks DPT mismatches. */
// SPDX-License-Identifier: AGPL-3.0-or-later
import { useEffect, useId, useLayoutEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import type { GroupAddressNode } from "./bindings/GroupAddressNode";
import { useTranslate } from "./i18n";
import { useUiLanguage } from "./uiLanguage";
import { useGroupAddressFormat } from "./gaNotation";
import { dptText } from "./groupAddressView";
import { dptDisplay } from "./dptLabels";
import { dptAgreement, filterPickerAddresses, PICKER_WINDOW } from "./groupAddressPicker";
import { useActiveOptionScroll } from "./useActiveOptionScroll";

/**
 * `addresses` is the caller's linkable set (MODEL-01); the picker never adds,
 * hides or reorders entries. A DPT difference is marked on the option only.
 */
export default function GroupAddressPicker(props: {
  addresses: GroupAddressNode[]; objectDpt: string | null; value: number | null;
  onChange: (id: number | null) => void;
}) {
  const { addresses, objectDpt, value, onChange } = props;
  const t = useTranslate(); const [language] = useUiLanguage(); const formatGa = useGroupAddressFormat();
  const listId = useId();
  const [query, setQuery] = useState("");
  // `typed`: the text is a search. After a choice the text is the chosen label,
  // and reopening lists everything again instead of filtering by that label.
  const [typed, setTyped] = useState(false);
  const [open, setOpen] = useState(false);
  const [highlight, setHighlight] = useState(0);
  const label = (ga: GroupAddressNode) => `${formatGa(ga.address)} ${ga.name}`.trim();
  const matches = useMemo(() => filterPickerAddresses(addresses, typed ? query : "", formatGa, language),
    [addresses, typed, query, formatGa, language]);
  const shown = matches.slice(0, PICKER_WINDOW);
  const active = open && shown.length > 0 ? Math.min(highlight, shown.length - 1) : -1;
  const activeRef = useActiveOptionScroll(active, shown, open);
  const optionId = (index: number) => `${listId}-option-${index}`;

  useEffect(() => { if (value === null && !typed) setQuery(""); }, [value, typed]);
  // The list grows in the flow; inside a scrolling table it would open below
  // the visible area, so opening brings all of it into view (nearest edge).
  const listRef = useRef<HTMLUListElement>(null);
  useLayoutEffect(() => { if (open) listRef.current?.scrollIntoView?.({ block: "nearest", inline: "nearest" }); }, [open]);

  function choose(ga: GroupAddressNode) {
    setQuery(label(ga)); setTyped(false); setOpen(false); setHighlight(0); onChange(ga.id);
  }
  function show() { setOpen(true); setHighlight(0); }
  function onKeyDown(event: KeyboardEvent<HTMLInputElement>) {
    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      if (!open) { show(); return; }
      const step = event.key === "ArrowDown" ? 1 : -1;
      setHighlight(current => Math.max(0, Math.min(shown.length - 1, Math.min(current, shown.length - 1) + step)));
    } else if (event.key === "Enter") {
      if (!open || active < 0) return;
      event.preventDefault(); choose(shown[active]);
    } else if (event.key === "Escape" && open) {
      // Only the open list is closed; a second Escape belongs to the page.
      event.preventDefault(); event.stopPropagation(); setOpen(false);
    } else if (event.key === "Tab") setOpen(false);
  }

  return <div className="ga-picker">
    <input type="text" role="combobox" aria-label={t("gaPicker.label")} placeholder={t("gaPicker.placeholder")}
      aria-autocomplete="list" aria-haspopup="listbox" aria-expanded={open} aria-controls={listId}
      aria-activedescendant={active >= 0 ? optionId(active) : undefined} autoComplete="off" spellCheck={false}
      value={query}
      onFocus={show} onClick={() => { if (!open) show(); }} onBlur={() => setOpen(false)} onKeyDown={onKeyDown}
      onChange={event => {
        setQuery(event.target.value); setTyped(true); setOpen(true); setHighlight(0);
        if (value !== null) onChange(null);
      }} />
    <ul ref={listRef} className="ga-picker-list" id={listId} role="listbox" aria-label={t("gaPicker.label")} hidden={!open}>
      {open && shown.map((ga, index) => {
        const agreement = dptAgreement(objectDpt, ga.dpts);
        return <li key={ga.id} id={optionId(index)} ref={index === active ? activeRef : undefined} role="option"
          aria-selected={index === active} data-ga-option={ga.id} data-dpt-agreement={agreement}
          className={index === active ? "ga-picker-option selected" : "ga-picker-option"}
          // Keep focus in the input, so choosing does not blur and close first.
          onMouseDown={event => event.preventDefault()} onClick={() => choose(ga)}>
          <span className="ga-address">{formatGa(ga.address)}</span>
          <span className="ga-picker-name">{ga.name}</span>
          <small className="ga-picker-dpt">{dptText(t, ga, language)}</small>
          {(agreement === "mismatch" || agreement === "subtype") && objectDpt !== null &&
            <small className="ga-picker-dpt-warning">{t(agreement === "mismatch" ? "gaPicker.dptMismatch" : "gaPicker.dptSubtype",
              { dpt: dptDisplay(objectDpt, language) })}</small>}
        </li>;
      })}
    </ul>
    {open && matches.length === 0 && <p className="ga-picker-empty" role="status">{t("gaPicker.noMatches")}</p>}
    {open && matches.length > shown.length && <p className="ga-picker-window" role="status">
      {t("gaPicker.window", { shown: shown.length, total: matches.length })}</p>}
  </div>;
}
