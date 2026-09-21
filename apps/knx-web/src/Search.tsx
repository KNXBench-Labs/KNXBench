/** Global search overlay indexing devices, group addresses, and building parts across the tree. */
import { useEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import type { ProjectTree } from "./bindings/ProjectTree";
import type { Selection } from "./selection";
import { buildSearchIndex } from "./treeUtils";
import type { SearchEntry } from "./treeUtils";
import { matchEntries } from "./searchMatch";
import Overlay from "./Overlay";
import { useTranslate } from "./i18n";
import { useGroupAddressFormat } from "./gaNotation";
import type { MessageKey } from "./messages/en";

const KIND_LABEL_KEYS: Record<SearchEntry["kind"], MessageKey> = {
  device: "search.kind.device",
  group_address: "search.kind.groupAddress",
  building_part: "search.kind.buildingPart",
};

const KIND_ORDER: SearchEntry["kind"][] = ["device", "group_address", "building_part"];

// `formatGa` applies to the group-address branch only: a device's address
// is an *individual* address, which is always dotted and never a
// preference (`gaNotation.ts`'s module comment).
function describeEntry(entry: SearchEntry, formatGa: (address: string) => string): string {
  if (entry.kind === "device") {
    return entry.address ? `${entry.label} — ${entry.address}` : entry.label;
  }
  if (entry.kind === "group_address") return `${entry.label} — ${formatGa(entry.address)}`;
  return `${entry.label} — ${entry.path}`;
}

export default function Search(props: {
  tree: ProjectTree;
  onSelect: (sel: Selection) => void;
  onClose: () => void;
}) {
  const { tree, onSelect, onClose } = props;
  const t = useTranslate();
  const formatGa = useGroupAddressFormat();
  const inputRef = useRef<HTMLInputElement>(null);
  const [query, setQuery] = useState("");
  const [highlight, setHighlight] = useState(0);
  const index = useMemo(() => buildSearchIndex(tree), [tree]);
  const results = useMemo(() => matchEntries(index, query), [index, query]);

  const grouped = KIND_ORDER.map((kind) => ({
    kind,
    entries: results.filter((entry) => entry.kind === kind),
  })).filter((group) => group.entries.length > 0);
  const ordered = grouped.flatMap((group) => group.entries);

  useEffect(() => {
    setHighlight(0);
  }, [results]);

  function pick(entry: SearchEntry) {
    onSelect({ kind: entry.kind, id: entry.id });
    onClose();
  }

  function handleKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setHighlight((h) => Math.min(h + 1, ordered.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setHighlight((h) => Math.max(h - 1, 0));
    } else if (e.key === "Enter") {
      const entry = ordered[highlight];
      if (entry) pick(entry);
    }
  }

  return (
    <Overlay label={t("search.overlayLabel")} onClose={onClose} initialFocusRef={inputRef}>
      <input
        ref={inputRef}
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        onKeyDown={handleKeyDown}
        placeholder={t("search.placeholder")}
        role="combobox"
        aria-haspopup="listbox"
        aria-expanded={ordered.length > 0}
        aria-controls="search-results"
        aria-activedescendant={ordered[highlight] ? `search-option-${highlight}` : undefined}
      />
      {query.trim() !== "" && results.length === 0 && <p className="search-empty">{t("search.noMatches")}</p>}
      <ul className="search-results" id="search-results" role="listbox">
        {grouped.map(({ kind, entries }) => (
          <li key={kind} className="search-group" role="group" aria-label={t(KIND_LABEL_KEYS[kind])}>
            <div className="search-group-label" aria-hidden="true">
              {t(KIND_LABEL_KEYS[kind])}
            </div>
            <ul role="presentation">
              {entries.map((entry) => {
                const position = ordered.indexOf(entry);
                return (
                  <li
                    key={`${entry.kind}-${entry.id}`}
                    id={`search-option-${position}`}
                    role="option"
                    aria-selected={position === highlight}
                    className={position === highlight ? "search-result selected" : "search-result"}
                    onClick={() => pick(entry)}
                  >
                    {describeEntry(entry, formatGa)}
                  </li>
                );
              })}
            </ul>
          </li>
        ))}
      </ul>
    </Overlay>
  );
}
